use crate::process::Process;
use anyhow::Result;
mod cowsay;
mod echo;
mod shell;

pub use shell::shell;

pub async fn get_program(process: &mut Process, args: Vec<String>) -> Option<Result<()>> {
    // could be ref
    let command = args[0].clone();

    if command == "echo" {
        Some(echo::echo(process, args).await)
    } else if command == "cowsay" {
        Some(cowsay::cowsay(process, args).await)
    } else {
        None
    }
}
