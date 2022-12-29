use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Locate a command.
#[derive(Parser)]
struct Options {
    /// Print all matching commands.
    #[arg(short)]
    all: bool,
    /// The command to locate.
    command: String,
}

pub async fn which(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    let Some(paths) = process.env.get("PATH") else {
        bail!("No ${{PATH}} environmental variable");
    };

    let mut code = ExitCode::FAILURE;
    for path in paths.split(':') {
        let path = process.get_path(path)?.join(&options.command)?;
        if path.exists()? && path.is_file()? {
            process.stdout.write_all(path.as_str().as_bytes()).await?;
            process.stdout.write_all(b"\n").await?;
            code = ExitCode::SUCCESS;
            if !options.all {
                break;
            }
        }
    }

    Ok(code)
}
