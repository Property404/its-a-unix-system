use crate::process::Process;
use anyhow::Result;
use clap::Parser;

/// Create directory.
#[derive(Parser)]
struct Options {
    /// The directories to create.
    #[arg(required(true))]
    directories: Vec<String>,
}

pub async fn mkdir(process: &mut Process, args: Vec<String>) -> Result<()> {
    let options = Options::try_parse_from(args.into_iter())?;
    for arg in options.directories.into_iter() {
        process.get_path(arg)?.create_dir_all()?;
    }
    Ok(())
}
