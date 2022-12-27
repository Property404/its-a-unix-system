//! Internal shell commands.
//!
//! Note that these don't take arguments from the process, because that process is the shell
//! itself.

use crate::{
    filesystem::vfs_path_to_str,
    process::{ExitCode, Process},
    programs::{
        self,
        common::readline::{NullHistory, Readline},
        sh::ShellContext,
    },
};
use anyhow::{bail, Result};
use clap::Parser;
use futures::AsyncWriteExt;

/// List of all internal shell commands.
pub const COMMANDS: [&str; 7] = ["cd", "env", "export", "read", "exit", "exec", "source"];

/// Exit shell.
pub async fn exit(
    ctx: &mut ShellContext,
    _process: &mut Process,
    args: Vec<String>,
) -> Result<ExitCode> {
    /// Exit shell.
    #[derive(Parser)]
    struct Options {
        /// The exit status.
        status: Option<u8>,
    }

    let options = Options::try_parse_from(args.iter())?;
    let code = options.status.map(ExitCode::from).unwrap_or_default();
    ctx.do_exit_with = Some(code);
    Ok(code)
}

pub async fn exec(
    ctx: &mut ShellContext,
    process: &mut Process,
    args: Vec<String>,
) -> Result<ExitCode> {
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

    ctx.do_exit_with = Some(code);
    Ok(code)
}

pub async fn source(
    ctx: &mut ShellContext,
    process: &mut Process,
    args: Vec<String>,
) -> Result<ExitCode> {
    /// Execute commands from file in current shell.
    #[derive(Parser)]
    struct Options {
        /// The program to run.
        file: String,
        /// The arguments to pass
        args: Vec<String>,
    }
    let options = Options::try_parse_from(args.iter())?;

    let mut script = String::new();
    process
        .get_path(&options.file)?
        .open_file()?
        .read_to_string(&mut script)?;

    let old_arguments = process.args.clone();
    process.args = vec![options.file.clone()];
    process.args.extend(options.args);

    let result = programs::sh::run_script(ctx, process, &script).await;

    process.args = old_arguments;
    result
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
        // `cd -` changes to the previous directory
        let new_path = if directory == "-" {
            if let Some(old_pwd) = process.env.get("OLDPWD") {
                process.get_path(old_pwd)?
            } else {
                bail!("OLDPWD not set");
            }
        } else {
            process.get_path(directory)?
        };

        if new_path.is_dir()? {
            process
                .env
                .insert("OLDPWD".into(), vfs_path_to_str(&process.cwd).into());
            process.cwd = new_path;
            process
                .env
                .insert("PWD".into(), vfs_path_to_str(&process.cwd).into());
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

/// Mark variable to be used in environment.
pub async fn export(
    ctx: &mut ShellContext,
    process: &mut Process,
    args: Vec<String>,
) -> Result<ExitCode> {
    /// Display environmental variables.
    #[derive(Parser)]
    struct Options {
        /// A variable and an optional value.
        expressions: Vec<String>,
    }

    let options = Options::try_parse_from(args.iter())?;

    for expression in options.expressions {
        let (identifier, value) = if expression.contains('=') {
            let (identifier, value) = expression
                .split_once('=')
                .expect("Bug: expected equals sign");
            (identifier.into(), value.into())
        } else {
            let value = process
                .env
                .get(&expression)
                .or_else(|| ctx.variables.get(&expression))
                .cloned()
                .unwrap_or_default();
            (expression, value)
        };

        ctx.variables.insert(identifier.clone(), value.clone());
        process.env.insert(identifier, value);
    }

    Ok(ExitCode::SUCCESS)
}

/// Display read user input and write to environmental variable.
pub async fn read(
    ctx: &mut ShellContext,
    process: &mut Process,
    args: Vec<String>,
) -> Result<ExitCode> {
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

    let mut readline = Readline::new(NullHistory::default());

    let line = readline
        .get_line(
            &options.prompt.unwrap_or_default(),
            &mut stdin,
            &mut stdout,
            |_, _| Ok(Vec::new()),
        )
        .await?;
    ctx.variables.insert(options.variable.clone(), line.clone());
    if process.env.contains_key(&options.variable) {
        process.env.insert(options.variable, line);
    }

    Ok(ExitCode::SUCCESS)
}
