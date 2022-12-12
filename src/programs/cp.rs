use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;

/// Copy a file.
#[derive(Parser)]
struct Options {
    /// The source file.
    src: String,
    /// The destination file or directory.
    dest: String,
}

pub async fn cp(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let src = process.get_path(&options.src)?;
    if !src.exists()? {
        bail!("src does not exist");
    }

    let mut dest = process.get_path(&options.dest)?;
    if dest.exists()? && dest.is_dir()? {
        dest = dest.join(src.filename())?;
    }
    // We have to check exists() twice because it could have changed.
    if dest.exists()? && dest.is_file()? {
        dest.remove_file()?;
    }
    src.copy_file(&dest)?;
    Ok(ExitCode::SUCCESS)
}
