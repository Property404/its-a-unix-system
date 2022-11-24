use crate::streams::{output_stream::OutputStreamBackend, OutputStream, TerminalWriter};
use anyhow::Result;
use std::io::Write;
use vfs::VfsPath;

pub fn file_redirect_out(
    file: VfsPath,
    append: bool,
) -> (OutputStream, OutputStreamBackend<FileOutWriter>) {
    let writer = FileOutWriter { file, append };
    let (output_stream, output_bkend) = OutputStream::from_writer(writer);

    (output_stream, output_bkend)
}

pub struct FileOutWriter {
    file: VfsPath,
    append: bool,
}

impl TerminalWriter for FileOutWriter {
    fn send(&mut self, content: &str) -> Result<()> {
        // This is a truly terrible approach and I hate it. I haven't figured out a better way to
        // do this, given that append_file() and create_file() don't return Send types. MAYBE I
        // could submit a PR, but I'm sure it's even possible with the way Vfs works.
        let mut file = if self.append {
            self.file.append_file()?
        } else {
            self.append = true;
            self.file.create_file()?
        };
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}
