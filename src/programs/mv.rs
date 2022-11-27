use crate::process::Process;
use anyhow::{bail, Result};
use clap::Parser;

/// Move a file.
#[derive(Parser)]
struct Options {
    /// The source file.
    src: String,
    /// The destination file or directory.
    dest: String,
}

pub async fn mv(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    let src = process.get_path(&options.src)?;
    if !src.exists()? {
        bail!("mv: src does not exist");
    }

    let mut dest = process.get_path(&options.dest)?;
    if dest.exists()? && dest.is_dir()? {
        dest = dest.join(src.filename())?;
    }
    // We have to check exists() twice because it could have changed.
    if dest.exists()? && dest.is_file()? {
        dest.remove_file()?;
    }
    src.move_file(&dest)?;
    Ok(())
}
