mod file_redirect_in;
mod file_redirect_out;
mod input_stream;
mod output_stream;
mod pipe;
mod standard_streams;
use anyhow::Result;
pub use file_redirect_in::file_redirect_in;
pub use file_redirect_out::file_redirect_out;
use futures::try_join;
pub use input_stream::{InputStream, InputStreamBackend, TerminalReader};
pub use output_stream::{OutputStream, OutputStreamBackend, TerminalWriter};
pub use pipe::pipe;
pub use standard_streams::standard;

pub struct Backend<R: TerminalReader, W: TerminalWriter> {
    input_bkend: InputStreamBackend<R>,
    output_bkend: OutputStreamBackend<W>,
}

impl<R: TerminalReader, W: TerminalWriter> Backend<R, W> {
    pub async fn run(&mut self) -> Result<()> {
        try_join! {
            self.input_bkend.run(),
            self.output_bkend.run(),
        }?;
        Ok(())
    }
}
