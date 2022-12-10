//! Internal shell commands.
//!
//! Note that these don't take arguments from the process, because that process is the shell
//! itself.

use crate::process::Process;
use anyhow::Result;
use clap::Parser;
use futures::AsyncWriteExt;

pub async fn cd(process: &mut Process, args: Vec<String>) -> Result<()> {
    /// Change directory.
    #[derive(Parser)]
    struct Options {
        /// The directory to enter.
        directory: Option<String>,
    }
    let options = Options::try_parse_from(args.iter())?;

    if let Some(directory) = options.directory {
        let new_path = process.get_path(directory)?;
        if new_path.is_dir()? {
            process.cwd = new_path;
        } else {
            process.stderr.write_all(b"cd: ").await?;
            process
                .stdout
                .write_all(new_path.as_str().as_bytes())
                .await?;
            process.stderr.write_all(b": No such directory\n").await?;
        }
    }

    Ok(())
}

pub async fn env(process: &mut Process, args: Vec<String>) -> Result<()> {
    /// Display environmental variables.
    #[derive(Parser)]
    struct Options {}
    let _options = Options::try_parse_from(args.iter())?;

    for (id, value) in process.env.iter() {
        process
            .stdout
            .write_all(format!("{id}={value}\n").as_bytes())
            .await?;
    }

    Ok(())
}
