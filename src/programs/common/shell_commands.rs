//! Internal shell commands.
//!
//! Note that these don't take arguments from the process, because that process is the shell
//! itself.

use crate::{
    process::{ExitCode, Process},
    programs::{
        self,
        common::readline::{NullHistory, Readline},
    },
};
use anyhow::{bail, Result};
use clap::Parser;
use futures::AsyncWriteExt;

/// List of all internal shell commands.
pub const COMMANDS: [&str; 5] = ["cd", "env", "read", "exit", "exec"];

/// Exit shell.
pub async fn exit(process: &mut Process, args: Vec<String>) -> Result<ExitCode> {
    /// Exit shell.
    #[derive(Parser)]
    struct Options {}
    let _options = Options::try_parse_from(args.iter())?;
    let code = ExitCode::SUCCESS;
    process.do_exit_with = Some(code);
    Ok(code)
}

pub async fn exec(process: &mut Process, args: Vec<String>) -> Result<ExitCode> {
    /// Replace shell with program.
    #[derive(Parser)]
    struct Options {
        /// The program to run.
        command: String,
        /// Pass name as zeroth argument.
        #[arg(short = 'a')]
        name: Option<String>,
        /// The arguments to pass, starting with argv[1].
        args: Vec<String>,
    }
    let options = Options::try_parse_from(args.iter())?;

    if let Some(name) = options.name {
        process.args = vec![name];
    } else {
        process.args = vec![options.command.clone()];
    }
    process.args.extend(options.args);

    let Some(code) = programs::exec_program(process, &options.command).await? else {
        bail!("Cannot find {}", options.command);
    };

    process.do_exit_with = Some(code);
    Ok(code)
}

/// Change directory.
pub async fn cd(process: &mut Process, args: Vec<String>) -> Result<ExitCode> {
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

    Ok(ExitCode::SUCCESS)
}

/// Display environmental variables.
pub async fn env(process: &mut Process, args: Vec<String>) -> Result<ExitCode> {
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

    Ok(ExitCode::SUCCESS)
}

/// Display read user input and write to environmental variable.
pub async fn read(process: &mut Process, args: Vec<String>) -> Result<ExitCode> {
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

    Ok(ExitCode::SUCCESS)
}
