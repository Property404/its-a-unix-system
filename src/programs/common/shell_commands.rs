//! Internal shell commands.
//!
//! Note that these don't take arguments from the process, because that process is the shell
//! itself.

use crate::{
    process::Process,
    programs::common::readline::{NullHistory, Readline},
};
use anyhow::Result;
use clap::Parser;
use futures::AsyncWriteExt;

/// List of all internal shell commands.
pub const COMMANDS: [&str; 3] = ["cd", "env", "read"];

/// Change directory.
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

/// Display environmental variables.
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

/// Display read user input and write to environmental variable.
pub async fn read(process: &mut Process, args: Vec<String>) -> Result<()> {
    /// Write user input to a variable.
    #[derive(Parser)]
    struct Options {
        /// The variable to read to.
        variable: String,
        /// A prompt to use.
        #[arg(short, long)]
        prompt: Option<String>,
    }
    let options = Options::try_parse_from(args.iter())?;

    let mut stdout = process.stdout.clone();
    let mut stdin = process.stdin.clone();

    let mut readline = Readline::new(options.prompt.unwrap_or_default(), NullHistory::default());

    let line = readline
        .get_line(&mut stdin, &mut stdout, |_, _| Ok(Vec::new()))
        .await?;
    process.env.insert(options.variable, line);

    Ok(())
}
