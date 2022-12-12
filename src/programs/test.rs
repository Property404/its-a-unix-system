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
    let options = Options::try_parse_from(process.args.iter())?;

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
