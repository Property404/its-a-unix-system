use crate::{process::Process, AnsiCode};
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Clear the screen
#[derive(Parser)]
struct Options {}

pub async fn clear(process: &mut Process, args: Vec<String>) -> Result<()> {
    let _options = Options::try_parse_from(args.into_iter())?;
    process
        .stdout
        .write_all(&AnsiCode::Clear.to_bytes())
        .await?;
    Ok(())
}
