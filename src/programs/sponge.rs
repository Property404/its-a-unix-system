use crate::process::Process;
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncReadExt;
use std::io::Write;

/// Soak up standard input and write to file.
#[derive(Parser)]
struct Options {
    /// The file to which to write
    file: String,
}

pub async fn sponge(process: &mut Process) -> Result<()> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut content = String::new();

    process.stdin.read_to_string(&mut content).await?;

    let path = process.get_path(options.file)?;
    let mut file = path.create_file()?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
