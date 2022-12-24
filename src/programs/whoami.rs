use crate::process::{ExitCode, Process};
use anyhow::{anyhow, Result};
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Prints the current user.
#[derive(Parser)]
struct Options {}

pub async fn whoami(process: &mut Process) -> Result<ExitCode> {
    let _options = Options::try_parse_from(process.args.iter())?;
    let user = process
        .env
        .get("USER")
        .ok_or_else(|| anyhow!("Could not get the user"))?;
    process.stdout.write_all(user.as_bytes()).await?;
    process.stdout.write_all(b"\n").await?;
    Ok(ExitCode::SUCCESS)
}
