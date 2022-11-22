use crate::process::Process;
use anyhow::{bail, Result};
use std::io::Write;

pub async fn ls(process: &mut Process, args: Vec<String>) -> Result<()> {
    let path = if args.len() == 1 {
        process.cwd.clone()
    } else {
        let dir = process.cwd.join(&args[1])?;
        if !dir.exists()? {
            bail!("ls: no such file or directory: {}", dir.as_str());
        }
        if !dir.is_dir()? {
            bail!("ls: not a directory: {}", dir.as_str());
        }
        dir
    };

    for entity in path.walk_dir()? {
        let entity = entity?;
        if entity.parent().as_ref() != Some(&path) {
            break;
        }
        let display = format!("{}\n", entity.filename());
        process.stdout.write_all(display.as_bytes())?;
    }
    Ok(())
}
