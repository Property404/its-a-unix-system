use crate::process::{ExitCode, Process};
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Print the name of the current working directory
#[derive(Parser)]
struct Options {}

pub async fn pwd(process: &mut Process) -> Result<ExitCode> {
    let _options = Options::try_parse_from(process.args.iter())?;

    let cwd = &process.cwd;
    let cwd = if cwd.is_root() { "/" } else { cwd.as_str() };

    process.stdout.write_all(cwd.as_bytes()).await?;
    process.stdout.write_all(b"\n").await?;
    Ok(ExitCode::SUCCESS)
}
