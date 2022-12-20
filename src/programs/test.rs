use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;

/// Evaluate conditional expression.
#[derive(Parser)]
struct Options {
    /// The first argument.
    arg1: String,
    /// The operation.
    op: String,
    /// The second argument.
    arg2: String,
}

pub async fn test(process: &mut Process) -> Result<ExitCode> {
    if process.args.is_empty() {
        bail!("No arguments");
    }

    // If we're aliased with '[', then end with ']'
    let args = if process.args[0] == "[" {
        if process.args.last().expect("No last item") != "]" {
            bail!("Expected ']'");
        }
        &process.args[0..process.args.len() - 1]
    } else {
        &process.args[..]
    };

    let options = Options::try_parse_from(args)?;

    let result = match options.op.as_str() {
        // Equal
        "==" => {
            if options.arg1 == options.arg2 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        // Not equal
        "!=" => {
            if options.arg1 != options.arg2 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => bail!("Unknown argument"),
    };
    Ok(result)
}
