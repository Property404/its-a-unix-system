use anyhow::Result;
use futures::{
    channel::{
        mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    io::{AsyncRead, AsyncReadExt},
    select,
    stream::{FusedStream, Stream, StreamExt},
    SinkExt,
};
use std::{
    io,
    ops::DerefMut,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

const NEWLINE: u8 = 0x0a;

pub trait TerminalReader: Sized + FusedStream<Item = Vec<u8>> + Unpin {}

enum InputCommand {
    Shutdown(oneshot::Sender<()>),
}

pub struct InputStreamBackend<T: TerminalReader> {
    frontend_tx: UnboundedSender<u8>,
    command_rx: Receiver<InputCommand>,
    reader: T,
}

impl<T: TerminalReader> InputStreamBackend<T> {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            select! {
                bytes = self.reader.next() => {
                    let bytes = bytes.expect("End of stream reached!");
                    for byte in bytes {
                        self.frontend_tx.unbounded_send(byte).expect("TODO: log this error");
                    }
                },
                command = self.command_rx.next() => {
                    let command = command.expect("End of command stream reached!");
                    match command {
                        InputCommand::Shutdown(signal) => {
                            signal.send(()).expect("Could not send shutdown signal");
                            break;
                        }
                    }
                }

            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct InputStream {
    backend_rx: Arc<Mutex<UnboundedReceiver<u8>>>,
    command_tx: Sender<InputCommand>,
}

impl InputStream {
    pub fn from_reader<T: TerminalReader>(reader: T) -> (InputStream, InputStreamBackend<T>) {
        let (frontend_tx, backend_rx) = mpsc::unbounded();
        let (command_tx, command_rx) = mpsc::channel(1000);

        (
            InputStream {
                backend_rx: Arc::new(Mutex::new(backend_rx)),
                command_tx,
            },
            InputStreamBackend {
                reader,
                frontend_tx,
                command_rx,
            },
        )
    }

    pub async fn get_line(&mut self) -> Result<String> {
        let mut line = Vec::new();
        let mut buffer = [0; 1];

        loop {
            self.read_exact(&mut buffer).await?;
            let byte = buffer[0];
            if byte == NEWLINE {
                return Ok(String::from_utf8_lossy(&line).to_string());
            }
            line.push(byte)
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let (tx, rx) = oneshot::channel::<()>();
        self.command_tx.send(InputCommand::Shutdown(tx)).await?;
        rx.await?;
        Ok(())
    }
}

impl AsyncRead for InputStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut buffer = Vec::new();

        let mut rx = self.backend_rx.lock().expect("Poisoned lock");
        while let Poll::Ready(Some(byte)) =
            unsafe { Pin::new_unchecked(rx.deref_mut()) }.poll_next(cx)
        {
            buffer.push(byte);
            if buffer.len() == buf.len() {
                break;
            }
        }

        if buffer.is_empty() {
            if rx.is_terminated() {
                return Poll::Ready(Ok(0));
            }
            return Poll::Pending;
        } else {
            buf[..buffer.len()].copy_from_slice(&buffer[..]);
        }

        Poll::Ready(Ok(buffer.len()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::try_join;

    #[derive(Default)]
    struct MockTerminalReader {
        pub contents: Vec<String>,
    }

    impl Stream for MockTerminalReader {
        type Item = Vec<u8>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.contents.pop().map(|s| s.as_bytes().to_vec()))
        }
    }

    impl FusedStream for MockTerminalReader {
        fn is_terminated(&self) -> bool {
            self.contents.is_empty()
        }
    }

    impl TerminalReader for MockTerminalReader {}

    #[futures_test::test]
    async fn test() {
        let mut reader = MockTerminalReader {
            contents: vec![
                String::from("Oh wow...sports.\n"),
                String::from("I smell death!\n"),
                String::from("It's Lapis."),
            ],
        };

        assert_eq!(
            String::from_utf8_lossy(&(reader.next().await.unwrap())),
            "It's Lapis.".to_string()
        );

        let (mut stream, mut backend) = InputStream::from_reader(reader);

        try_join!(backend.run(), async move {
            let string = stream.get_line().await.unwrap();
            assert_eq!(string, String::from("I smell death!"));
            let string = stream.get_line().await.unwrap();
            assert_eq!(string, String::from("Oh wow...sports."));
            stream.shutdown().await.unwrap();
            Ok(())
        })
        .unwrap();
    }
}
