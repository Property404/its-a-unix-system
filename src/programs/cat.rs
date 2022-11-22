use crate::process::Process;
use anyhow::{bail, Result};
use std::io::Write;

pub async fn cat(process: &mut Process, args: Vec<String>) -> Result<()> {
    if args.len() == 1 {
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
    for arg in args.into_iter().skip(1) {
        let path = process.cwd.join(arg)?;
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
