use crate::process::Process;
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Create a file if it does not exist.
#[derive(Parser)]
struct Options {
    /// The file(s) to touch or create.
    #[arg(required(true))]
    files: Vec<String>,
}

pub async fn touch(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    for arg in options.files.into_iter() {
        if arg == "me" {
            process.stderr.write_all(
            b"Absolutely not.\n").await?;
        }
        process.get_path(arg)?.create_file()?;
    }
    Ok(())
}
