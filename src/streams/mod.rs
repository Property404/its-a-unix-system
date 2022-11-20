mod input_stream;
mod output_stream;
mod pipe;
mod standard_streams;
use anyhow::Result;
use futures::try_join;
pub use input_stream::InputStream;
pub use input_stream::{InputStreamBackend, TerminalReader};
pub use output_stream::OutputStream;
pub use output_stream::{OutputStreamBackend, TerminalWriter};
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
