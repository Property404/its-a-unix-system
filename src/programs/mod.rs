use crate::process::Process;
use anyhow::Result;
mod cat;
mod cowsay;
mod echo;
mod fortune;
mod ls;
mod mkdir;
mod shell;
mod touch;

pub use shell::shell;

pub async fn get_program(process: &mut Process, args: Vec<String>) -> Option<Result<()>> {
    // could be ref
    let command = args[0].clone();

    if command == "echo" {
        Some(echo::echo(process, args).await)
    } else if command == "cowsay" {
        Some(cowsay::cowsay(process, args).await)
    } else if command == "ls" {
        Some(ls::ls(process, args).await)
    } else if command == "touch" {
        Some(touch::touch(process, args).await)
    } else if command == "mkdir" {
        Some(mkdir::mkdir(process, args).await)
    } else if command == "cat" {
        Some(cat::cat(process, args).await)
    } else if command == "fortune" {
        Some(fortune::fortune(process, args).await)
    } else {
        None
    }
}
