use crate::process::{ExitCode, Process};
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncReadExt;
use std::io::Write;

const MAX_WIDTH: usize = 40;
const COWS_DIR: &str = "/usr/share/cowsay/cows";
const DEFAULT_COW: &str = "cow";

/// Have a cow say things
#[derive(Parser)]
struct Options {
    /// The things to say.
    args: Vec<String>,
    /// List cow files.
    #[arg(short)]
    list: bool,
    /// The cowfile to use.
    #[arg(short)]
    file: Option<String>,
}

pub async fn cowsay(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    if options.list {
        process.stdout.write_all(b"Cows in ")?;
        process.stdout.write_all(COWS_DIR.as_bytes())?;
        process.stdout.write_all(b":\n")?;
        for file in process.cwd.join(COWS_DIR)?.read_dir()? {
            process.stdout.write_all(file.filename().as_bytes())?;
            process.stdout.write_all(b"\n")?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let text = if options.args.is_empty() {
        let mut text = String::new();
        process.stdin.read_to_string(&mut text).await?;
        text
    } else {
        options.args.into_iter().collect::<Vec<_>>().join(" ")
    };

    let lines = textwrap::wrap(text.trim(), MAX_WIDTH);
    let mut width = 0;
    for line in lines.iter() {
        if line.len() > width {
            width = line.len();
        }
    }

    process.stdout.write_all(b" ")?;
    for _ in 0..width + 2 {
        process.stdout.write_all(b"_")?;
    }
    process.stdout.write_all(b"\n")?;

    for (index, line) in lines.iter().enumerate() {
        process.stdout.write_all(if lines.len() > 1 {
            if index == 0 {
                b"/"
            } else if index == lines.len() - 1 {
                b"\\"
            } else {
                b"|"
            }
        } else {
            b"<"
        })?;
        process.stdout.write_all(b" ")?;
        process.stdout.write_all(line.as_bytes())?;
        for _ in 0..=width - line.len() {
            process.stdout.write_all(b" ")?;
        }
        process.stdout.write_all(if lines.len() > 1 {
            if index == 0 {
                b"\\"
            } else if index == lines.len() - 1 {
                b"/"
            } else {
                b"|"
            }
        } else {
            b">"
        })?;
        process.stdout.write_all(b"\n")?;
    }
    process.stdout.write_all(b" ")?;
    for _ in 0..width + 2 {
        process.stdout.write_all(b"-")?;
    }
    process.stdout.write_all(b"\n")?;

    let cow_file = options.file.unwrap_or_else(|| DEFAULT_COW.into());
    let mut cow_file = process.cwd.join(COWS_DIR)?.join(cow_file)?.open_file()?;
    let mut cow = Vec::new();
    cow_file.read_to_end(&mut cow)?;

    process.stdout.write_all(&cow)?;
    Ok(ExitCode::SUCCESS)
}
