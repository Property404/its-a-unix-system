use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;
use futures::AsyncReadExt;
use std::{collections::HashSet, io::Write};

/// Sort files or stdin.
#[derive(Parser)]
struct Options {
    /// Sort in reverse order.
    #[arg(short, long)]
    reverse: bool,
    /// Remove repeated lines.
    #[arg(short, long)]
    unique: bool,
    /// The files to concatenate and sort.
    files: Vec<String>,
}

pub async fn sort(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut contents = String::new();

    if options.files.is_empty() {
        process.stdin.read_to_string(&mut contents).await?;
    }

    for arg in options.files.into_iter() {
        let path = process.get_path(arg)?;
        if !path.exists()? {
            bail!("No such file {}", path.as_str());
        }
        if !path.is_file()? {
            bail!("{} is not a file", path.as_str());
        }
        let mut file = path.open_file()?;
        file.read_to_string(&mut contents)?;
    }

    let mut lines: Vec<&str> = contents.lines().collect();
    if options.unique {
        let set = lines.into_iter().collect::<HashSet<_>>();
        lines = set.into_iter().collect();
    }

    lines.sort();

    if options.reverse {
        lines = lines.into_iter().rev().collect();
    }

    for line in lines {
        process.stdout.write_all(line.as_bytes())?;
        process.stdout.write_all(b"\n")?;
    }

    Ok(ExitCode::SUCCESS)
}
