use crate::process::Process;
use anyhow::{anyhow, bail, Result};
mod common;

pub use sh::sh as shell;

// Run a program from '/bin' or somewhere.
async fn exec_external_program(
    process: &mut Process,
    mut args: Vec<String>,
) -> Result<Option<Result<()>>> {
    let command = args[0].clone();
    let mut run_command = false;

    let root = process.cwd.root();
    let paths = process
        .env
        .get("PATH")
        .ok_or_else(|| anyhow!("PATH not set"))?
        .split(':')
        .map(|path| root.join(path));

    for path in paths {
        let path = path?;
        for entity in path.walk_dir()? {
            let entity = entity?;
            // Don't delve into children.
            if entity.parent().as_ref() != Some(&path) {
                break;
            }

            if entity.is_file()? && entity.filename() == command {
                args.insert(1, entity.as_str().into());
                run_command = true;
            }
        }
    }

    //  Necessary to do it this way because VfsPath::walk_dir() does not return a Sync object
    if run_command {
        Ok(Some(shell(process, args).await))
    } else {
        Ok(None)
    }
}

macro_rules! implement {
    ($($cmd:ident),*) => {
        $(
            mod $cmd;
        )*
        pub async fn get_program(process: &mut Process, args: Vec<String>) -> Result<Option<Result<()>>> {
            if args.is_empty() {
                bail!("At least one argument is required to execute a program");
            }
            // could be ref
            let command = &args[0];
            $(
                if command == stringify!($cmd) {
                    Ok(Some($cmd::$cmd(process, args).await))
                } else
            )*
            {
                exec_external_program(process, args).await
            }
        }
    }
}

implement!(cat, cowsay, cp, echo, fortune, grep, ls, mkdir, mv, rm, sh, sponge, tee, touch);
