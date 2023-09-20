use crate::process::{ExitCode, Process};
use anyhow::Result;
use clap::Parser;

/// Create directory.
#[derive(Parser)]
struct Options {
    /// The directories to create.
    #[arg(required(true))]
    directories: Vec<String>,
}

pub async fn mkdir(process: &Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    for arg in options.directories.into_iter() {
        process.get_path(arg)?.create_dir_all()?;
    }
    Ok(ExitCode::SUCCESS)
}
