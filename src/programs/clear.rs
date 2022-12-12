use crate::{
    process::{ExitCode, Process},
    AnsiCode,
};
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Clear the screen
#[derive(Parser)]
struct Options {}

pub async fn clear(process: &mut Process) -> Result<ExitCode> {
    let _options = Options::try_parse_from(process.args.iter())?;
    process
        .stdout
        .write_all(&AnsiCode::Clear.to_bytes())
        .await?;
    Ok(ExitCode::SUCCESS)
}
