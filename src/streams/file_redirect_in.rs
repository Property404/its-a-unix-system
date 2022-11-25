use crate::streams::{InputStream, InputStreamBackend, TerminalReader};
use futures::stream::{FusedStream, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use vfs::path::SeekAndRead;

pub fn file_redirect_in(
    file: Box<dyn SeekAndRead + Send>,
) -> (InputStream, InputStreamBackend<FileInReader>) {
    let reader = FileInReader {
        file,
        terminated: false,
    };
    let (input_stream, input_bkend) = InputStream::from_reader(reader);

    (input_stream, input_bkend)
}

pub struct FileInReader {
    file: Box<dyn SeekAndRead + Send>,
    terminated: bool,
}

impl TerminalReader for FileInReader {}

impl Stream for FileInReader {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buffer = [0; 32];
        match self.file.read(&mut buffer) {
            Err(_) | Ok(0) => {
                self.terminated = true;
                Poll::Ready(None)
            }
            Ok(size) => Poll::Ready(Some(buffer[0..size].to_vec())),
        }
    }
}

impl FusedStream for FileInReader {
    fn is_terminated(&self) -> bool {
        self.terminated
    }
}
