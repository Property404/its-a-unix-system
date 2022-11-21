use crate::process::Process;
use anyhow::Result;
use std::io::Write;

pub async fn ls(process: &mut Process, args: Vec<String>) -> Result<()> {
    let directories = if args.len() == 1 {
        process.cwd.walk_dir()?
    } else {
        let dir = process.cwd.join(&args[1])?;
        dir.walk_dir()?
    };

    for entity in directories {
        let entity = entity?;
        if entity.parent().as_ref() != Some(&process.cwd) {
            break;
        }
        let display = format!("{}\n", entity.filename());
        process.stdout.write_all(display.as_bytes())?;
    }
    Ok(())
}
