use crate::process::{ExitCode, Process};
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

pub async fn echo(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut args = options.args.into_iter();
    let mut ends_with_line_feed = false;

    if let Some(first_argument) = args.next() {
        process.stdout.write_all(first_argument.as_bytes()).await?;
        ends_with_line_feed = first_argument.ends_with('\n');
    }
    for item in args {
        process.stdout.write_all(b" ").await?;
        process.stdout.write_all(item.as_bytes()).await?;
        ends_with_line_feed = item.ends_with('\n');
    }

    // echo on Linux seems to not print an extra newline if stuff-to-be-echoed ends with a newline
    // already. Which kind of makes sense from a usabilty perspective.
    // Let's emulate that.
    if !options.no_newline && !ends_with_line_feed {
        process.stdout.write_all(b"\n").await?;
    }

    Ok(ExitCode::SUCCESS)
}
