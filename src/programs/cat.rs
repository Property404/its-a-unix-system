use crate::process::Process;
use anyhow::{bail, Result};
use clap::Parser;
use std::io::Write;

/// Concatenate files.
#[derive(Parser)]
struct Options {
    /// The files to concatenate.
    files: Vec<String>,
}

pub async fn cat(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    if options.files.is_empty() {
        loop {
            if let Ok(line) = process.stdin.get_line().await {
                process.stdout.write_all(line.as_bytes())?;
                process.stdout.write_all(b"\n")?;
            } else {
                // Ignore 'unexpected end of file'
                return Ok(());
            }
        }
    }

    let mut contents = String::new();
    for arg in options.files.into_iter() {
        let path = process.get_path(arg)?;
        if !path.exists()? {
            bail!("cat: No such file {}", path.as_str());
        }
        if !path.is_file()? {
            bail!("cat: {} is not a file", path.as_str());
        }
        let mut file = path.open_file()?;
        contents.clear();
        file.read_to_string(&mut contents)?;
        process.stdout.write_all(contents.as_bytes())?
    }

    Ok(())
}
