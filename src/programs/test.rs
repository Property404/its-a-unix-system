use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;
use regex::Regex;

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

pub async fn test(process: &Process) -> Result<ExitCode> {
    if process.args.is_empty() {
        bail!("No arguments");
    }

    let mut invert = false;

    // If we're aliased with '[', then end with ']'
    let args = if process.args[0] == "[" {
        if process.args.last().expect("No last item") != "]" {
            bail!("Expected ']'");
        }
        &process.args[0..process.args.len() - 1]
    } else {
        &process.args[..]
    };

    // Invert?
    let args = if args.get(1).map(|s| s == "!").unwrap_or(false) {
        invert = true;
        &args[1..]
    } else {
        args
    };

    let options = Options::try_parse_from(args)?;

    let mut result = match options.op.as_str() {
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
        // Regex match
        "=~" => {
            let pattern = Regex::new(&options.arg2)?;
            if pattern.is_match(&options.arg1) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        _ => bail!("Unknown argument"),
    };

    if invert {
        if result == ExitCode::SUCCESS {
            result = ExitCode::FAILURE
        } else {
            result = ExitCode::SUCCESS
        }
    }

    Ok(result)
}
