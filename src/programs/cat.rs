use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;
use std::io::Write;

/// Concatenate files.
#[derive(Parser)]
struct Options {
    /// The files to concatenate.
    files: Vec<String>,
}

pub async fn cat(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    if options.files.is_empty() {
        loop {
            if let Ok(line) = process.stdin.get_line().await {
                process.stdout.write_all(line.as_bytes())?;
                process.stdout.write_all(b"\n")?;
            } else {
                // Ignore 'unexpected end of file'
                return Ok(ExitCode::Success);
            }
        }
    }

    let mut contents = String::new();
    for arg in options.files.into_iter() {
        let path = process.get_path(arg)?;
        if !path.exists()? {
            bail!("No such file {}", path.as_str());
        }
        if !path.is_file()? {
            bail!("{} is not a file", path.as_str());
        }
        let mut file = path.open_file()?;
        contents.clear();
        file.read_to_string(&mut contents)?;
        process.stdout.write_all(contents.as_bytes())?
    }

    Ok(ExitCode::SUCCESS)
}
