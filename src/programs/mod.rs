use crate::process::Process;
use anyhow::Result;
mod cat;
mod common;
mod cowsay;
mod cp;
mod echo;
mod fortune;
mod grep;
mod ls;
mod mkdir;
mod mv;
mod rm;
mod shell;
mod sponge;
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
    } else if command == "sh" {
        Some(shell::shell(process, args).await)
    } else if command == "rm" {
        Some(rm::rm(process, args).await)
    } else if command == "mv" {
        Some(mv::mv(process, args).await)
    } else if command == "cp" {
        Some(cp::cp(process, args).await)
    } else if command == "grep" {
        Some(grep::grep(process, args).await)
    } else if command == "sponge" {
        Some(sponge::sponge(process, args).await)
    } else {
        None
    }
}
