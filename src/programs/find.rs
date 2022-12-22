use crate::process::{ExitCode, Process};
use anyhow::{bail, Result};
use clap::Parser;
use futures::AsyncWriteExt;
use vfs::VfsPath;

/// Search for files/directories
#[derive(Parser)]
struct Options {
    /// The directories to search.
    directories: Vec<String>,
}

pub async fn find(process: &mut Process) -> Result<ExitCode> {
    let mut options = Options::try_parse_from(process.args.iter())?;

    if options.directories.is_empty() {
        options.directories.push(String::from("."));
    }

    let paths: Result<Vec<(VfsPath, String)>> = options
        .directories
        .into_iter()
        .map(|mut path_expression| {
            let dir = process.get_path(&path_expression)?;
            if !path_expression.ends_with('/') {
                path_expression.push('/');
            }
            if !dir.exists()? {
                bail!("No such file or directory: {}", dir.as_str());
            }
            if !dir.is_dir()? {
                bail!("Not a directory: {}", dir.as_str());
            }
            Ok((dir, path_expression))
        })
        .collect();

    for (path, expressed_as) in paths? {
        for entity in path.walk_dir()? {
            let dis = entity?.as_str().to_string();
            let path_str = format!("{}/", path.as_str());
            let mut dis = dis.replacen(path_str.as_str(), expressed_as.as_str(), 1);
            dis.push('\n');
            process.stdout.write_all(dis.as_bytes()).await?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
