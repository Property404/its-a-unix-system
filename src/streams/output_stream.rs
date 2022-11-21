use anyhow::Result;
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    io::AsyncWrite,
    select,
    stream::StreamExt,
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

const NEWLINE: u8 = 0x0a;
const CARRIAGE_RETURN: u8 = 0x0c;

pub trait TerminalWriter: Sized {
    fn send(&mut self, content: &str) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

pub enum OutputCommand {
    Bytes(Vec<u8>),
    Flush,
    Shutdown(oneshot::Sender<()>),
}

pub struct OutputStreamBackend<T: TerminalWriter> {
    writer: T,
    rx: UnboundedReceiver<OutputCommand>,
}

impl<T: TerminalWriter> OutputStreamBackend<T> {
    pub fn new(writer: T, rx: UnboundedReceiver<OutputCommand>) -> Self {
        Self { writer, rx }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut buffer = Vec::new();
        loop {
            select! {
                command = self.rx.next() => {
                    match command.expect("End of command channel") {
                        OutputCommand::Bytes(bytes) => {
                            for byte in bytes {
                                buffer.push(byte);
                                if byte == NEWLINE || byte == CARRIAGE_RETURN {
                                    self.write(&buffer)?;
                                    buffer.clear();
                                }
                            }
                        },
                        OutputCommand::Flush => {
                            self.write(&buffer)?;
                            buffer.clear();
                        },
                        OutputCommand::Shutdown(signal) => {
                            self.writer.shutdown()?;
                            signal.send(()).expect("Could not send shutdown signal");
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<()> {
        let buffer = std::str::from_utf8(buf)?;
        self.writer.send(buffer)?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct OutputStream {
    tx: UnboundedSender<OutputCommand>,
}

impl OutputStream {
    pub fn from_writer<T: TerminalWriter>(writer: T) -> (Self, OutputStreamBackend<T>) {
        let (tx, rx) = mpsc::unbounded();
        let backend = OutputStreamBackend::new(writer, rx);
        (Self { tx }, backend)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel::<()>();
        self.tx.unbounded_send(OutputCommand::Shutdown(tx))?;
        rx.await?;
        self.tx.close_channel();
        Ok(())
    }
}

impl io::Write for OutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .unbounded_send(OutputCommand::Bytes(buf.to_vec()))
            .map_err(|_| io::ErrorKind::Other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tx
            .unbounded_send(OutputCommand::Flush)
            .map_err(|_| io::ErrorKind::Other)?;
        Ok(())
    }
}

impl AsyncWrite for OutputStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut tx = self.tx.clone();
        match tx.poll_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                if tx.start_send(OutputCommand::Bytes(buf.to_vec())).is_err() {
                    return Poll::Ready(Err(io::ErrorKind::Other.into()));
                }
                Poll::Ready(Ok(buf.len()))
            }
            Poll::Ready(Err(_)) => Poll::Ready(Err(io::ErrorKind::Other.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut tx = self.tx.clone();
        match tx.poll_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                if tx.start_send(OutputCommand::Flush).is_err() {
                    return Poll::Ready(Err(io::ErrorKind::Other.into()));
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(_)) => Poll::Ready(Err(io::ErrorKind::Other.into())),
        }
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        unimplemented!("Cannot close");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{io::AsyncWriteExt, try_join};

    #[derive(Default)]
    struct MockTerminalWriter {
        pub content: String,
    }

    impl TerminalWriter for MockTerminalWriter {
        fn send(&mut self, content: &str) -> Result<()> {
            self.content += content.into();
            Ok(())
        }
        fn shutdown(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[futures_test::test]
    async fn test() {
        let writer = MockTerminalWriter::default();
        let (mut stream, mut backend) = OutputStream::from_writer(writer);

        try_join!(backend.run(), async move {
            stream.write("Hello World!".as_bytes()).await?;
            stream.flush().await?;
            stream.write("\nGoodbye\n".as_bytes()).await?;
            stream.shutdown().await?;
            Ok(())
        })
        .unwrap();
        assert_eq!(backend.writer.content, "Hello World!\nGoodbye\n")
    }
}
