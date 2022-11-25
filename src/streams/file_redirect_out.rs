use crate::streams::{output_stream::OutputStreamBackend, OutputStream, TerminalWriter};
use anyhow::Result;
use std::io::Write;

pub fn file_redirect_out(
    file: Box<dyn Write + Send>,
) -> (OutputStream, OutputStreamBackend<FileOutWriter>) {
    let writer = FileOutWriter { file };
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);

    (output_stream, output_bkend)
}

pub struct FileOutWriter {
    file: Box<dyn Write + Send>,
}

impl TerminalWriter for FileOutWriter {
    fn send(&mut self, content: &str) -> Result<()> {
        self.file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}
