use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;

/// Remove/unlink a file.
#[derive(Parser)]
struct Options {
    /// Ignore nonexistent files.
    #[arg(short, long)]
    force: bool,
    /// Recursively remove directories and their contents.
    #[arg(short, long)]
    recursive: bool,
    /// The file(s) to remove
    #[arg(required(true))]
    files: Vec<String>,
}

pub async fn rm(process: &Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    for file in options.files {
        let path = process.get_path(file)?;

        if !options.recursive && path.is_dir()? {
            bail!("cannot operate recursively");
        }

        let result = path.remove_file();
        if !options.force {
            result?
        }
    }
    Ok(ExitCode::SUCCESS)
}
