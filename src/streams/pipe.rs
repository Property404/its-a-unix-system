use crate::streams::{Backend, InputStream, OutputStream, TerminalReader, TerminalWriter};
use anyhow::{anyhow, Result};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    stream::{FusedStream, Stream},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub fn pipe() -> (InputStream, OutputStream, Backend<PipeReader, PipeWriter>) {
    let (tx, stream) = mpsc::unbounded();
    let writer = PipeWriter { tx };
    let reader = PipeReader { stream };
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);
    let (input_stream, input_bkend) = InputStream::from_reader(reader);
    let backend = Backend {
        input_bkend,
        output_bkend,
    };

    (input_stream, output_stream, backend)
}

pub struct PipeWriter {
    tx: UnboundedSender<Vec<u8>>,
}

impl TerminalWriter for PipeWriter {
    fn send(&mut self, content: &str) -> Result<()> {
        self.tx
            .unbounded_send(content.as_bytes().to_vec())
            .map_err(|_| anyhow!("Broken pipe"))?;
        Ok(())
    }
    fn shutdown(&mut self) -> Result<()> {
        self.tx.close_channel();
        Ok(())
    }
}

pub struct PipeReader {
    stream: UnboundedReceiver<Vec<u8>>,
}

impl TerminalReader for PipeReader {
    fn shutdown(&mut self) -> Result<()> {
        self.stream.close();
        Ok(())
    }
}

impl Stream for PipeReader {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

impl FusedStream for PipeReader {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::{io::AsyncWriteExt, try_join};

    #[futures_test::test]
    async fn make_pipe() {
        let (mut pin, mut pout, mut backend) = pipe();
        try_join! {
            backend.run(),
            async {
                pout.write_all(b"Hello\nWorld!\n").await?;
                assert_eq!(pin.get_line().await?, "Hello");
                assert_eq!(pin.get_line().await?, "World!");
                pout.shutdown().await?;
                pin.shutdown().await?;
                Ok(())
            }
        }
        .unwrap();
    }
}
