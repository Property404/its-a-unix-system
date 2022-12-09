use crate::process::Process;
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Echo args to standard out.
#[derive(Parser)]
struct Options {
    /// Do not append a newline.
    #[arg(short)]
    no_newline: bool,
    /// The arguments to echo.
    args: Vec<String>,
}

pub async fn echo(process: &mut Process) -> Result<()> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut args = options.args.into_iter();
    if let Some(first_argument) = args.next() {
        process.stdout.write_all(first_argument.as_bytes()).await?;
    }
    for item in args {
        process.stdout.write_all(b" ").await?;
        process.stdout.write_all(item.as_bytes()).await?;
    }

    if !options.no_newline {
        process.stdout.write_all(b"\n").await?;
    }

    Ok(())
}
