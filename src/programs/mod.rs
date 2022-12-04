use crate::process::Process;
use anyhow::Result;
mod common;

pub use sh::sh as shell;

macro_rules! implement {
    ($($cmd:ident),*) => {
        $(
            mod $cmd;
        )*
        pub async fn get_program(process: &mut Process, args: Vec<String>) -> Option<Result<()>> {
            // could be ref
            let command = &args[0];
            $(
                if command == stringify!($cmd) {
                    Some($cmd::$cmd(process, args).await)
                } else
            )*
            {
                None
            }
        }
    }
}

implement!(cat, cowsay, cp, echo, fortune, grep, ls, mkdir, mv, rm, sh, sponge, tee, touch);
