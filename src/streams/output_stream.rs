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

pub trait TerminalWriter: Sized + Send {
    fn send(&mut self, content: &str) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    /// Is this being output to a terminal?
    fn to_terminal(&self) -> bool {
        false
    }
}

enum Command {
    Bytes(Vec<u8>),
    Flush,
    ToTerminal(oneshot::Sender<bool>),
    Shutdown(oneshot::Sender<()>),
}

pub struct OutputStreamBackend<T: TerminalWriter> {
    writer: T,
    rx: UnboundedReceiver<Command>,
}

impl<T: TerminalWriter> OutputStreamBackend<T> {
    fn new(writer: T, rx: UnboundedReceiver<Command>) -> Self {
        Self { writer, rx }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut buffer = Vec::new();
        loop {
            select! {
                command = self.rx.next() => {
                    match command.expect("End of command channel") {
                        Command::Bytes(bytes) => {
                            let mut flush = false;
                            for byte in bytes {
                                buffer.push(byte);
                                if byte == NEWLINE || byte == CARRIAGE_RETURN {
                                    flush = true;
                                }
                            }
                            if flush {
                                self.write(&buffer)?;
                                buffer.clear();
                            }
                        },
                        Command::Flush => {
                            if ! buffer.is_empty() {
                                self.write(&buffer)?;
                                buffer.clear();
                            }
                        },
                        Command::Shutdown(signal) => {
                            if ! buffer.is_empty() {
                                self.write(&buffer)?;
                                buffer.clear();
                            }
                            self.writer.shutdown()?;
                            signal.send(()).expect("Could not send shutdown signal");
                            return Ok(());
                        },
                        Command::ToTerminal(signal) => {
                            let _ = signal.send(self.writer.to_terminal());
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
    tx: UnboundedSender<Command>,
}

impl OutputStream {
    pub fn from_writer<T: TerminalWriter>(writer: T) -> (Self, OutputStreamBackend<T>) {
        let (tx, rx) = mpsc::unbounded();
        let backend = OutputStreamBackend::new(writer, rx);
        (Self { tx }, backend)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel::<()>();
        self.tx.unbounded_send(Command::Shutdown(tx))?;
        rx.await?;
        self.tx.close_channel();
        Ok(())
    }

    /// Query the backend to check if this is really being output to a terminal.
    pub async fn to_terminal(&self) -> Result<bool> {
        let (tx, rx) = oneshot::channel::<bool>();
        self.tx.unbounded_send(Command::ToTerminal(tx))?;
        Ok(rx.await?)
    }
}

impl io::Write for OutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .unbounded_send(Command::Bytes(buf.to_vec()))
            .map_err(|_| io::ErrorKind::Other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tx
            .unbounded_send(Command::Flush)
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
                if tx.start_send(Command::Bytes(buf.to_vec())).is_err() {
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
                if tx.start_send(Command::Flush).is_err() {
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
