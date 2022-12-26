use crate::process::{ExitCode, Process};
use anyhow::{anyhow, Result};
use std::io::Write;
mod common;

pub use sh::sh as shell;

// Run a program from '/bin' or somewhere.
async fn exec_external_program(
    process: &mut Process,
    command: &str,
) -> Result<Option<Result<ExitCode>>> {
    let root = process.cwd.root();

    if command.starts_with('/') || command.starts_with("./") {
        let mut contents = String::new();
        process
            .get_path(command)?
            .open_file()?
            .read_to_string(&mut contents)?;
        let mut ctx = sh::ShellContext::default();
        return Ok(Some(sh::run_script(&mut ctx, process, &contents).await));
    }

    let paths = process
        .env
        .get("PATH")
        .ok_or_else(|| anyhow!("PATH not set"))?
        .split(':')
        .map(|path| root.join(path));

    for path in paths {
        let path = path?;
        for entity in path.read_dir()? {
            if entity.is_file()? && entity.filename() == command {
                let mut contents = String::new();
                entity.open_file()?.read_to_string(&mut contents)?;
                let mut ctx = sh::ShellContext::default();
                return Ok(Some(sh::run_script(&mut ctx, process, &contents).await));
            }
        }
    }

    Ok(None)
}

macro_rules! implement {
    ($($cmd:ident),*) => {
        $(
            mod $cmd;
        )*
        pub async fn exec_program(process: &mut Process, command: &str) -> Result<Option<ExitCode>> {
            let result = $(
                if command == stringify!($cmd) {
                    Some($cmd::$cmd(process).await)
                } else
            )*
            {
                exec_external_program(process, command).await?
            };

            Ok(match result {
                None => None,
                Some(Ok(code)) => Some(code),
                Some(Err(e)) => {
                    process.stderr.write_all(command.as_bytes())?;
                    process.stderr.write_all(b": ")?;
                    process.stderr.write_all(e.to_string().as_bytes())?;
                    process.stderr.write_all(b"\n")?;
                    Some(ExitCode::FAILURE)
                }
            })
        }
    }
}

implement!(
    cat, clear, cowsay, cp, echo, fortune, find, grep, head, ls, mkdir, mv, pwd, rev, rm, rmdir,
    sed, sh, sort, sponge, tail, tee, test, touch, vi, wc, whoami
);
