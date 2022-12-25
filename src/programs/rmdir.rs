use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;

/// Remove a directory if empty.
#[derive(Parser)]
struct Options {
    /// The directories to remove.
    #[arg(required(true))]
    dirs: Vec<String>,
}

pub async fn rmdir(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    for dir in options.dirs {
        let path = process.get_path(dir)?;

        if !path.is_dir()? {
            bail!("Not a directory");
        }

        if path.read_dir()?.next().is_some() {
            bail!("Directory not empty");
        }

        path.remove_file()?;
    }
    Ok(ExitCode::SUCCESS)
}
