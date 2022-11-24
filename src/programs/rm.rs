use crate::process::Process;
use anyhow::{bail, Result};

pub async fn rm(process: &mut Process, args: Vec<String>) -> Result<()> {
    if args.len() < 2 {
        bail!("rm: missing operand")
    }
    for arg in args.into_iter().skip(1) {
        let path = process.get_path(arg)?;
        if path.is_dir()? {
            bail!("rm: cannot operate recursively");
        }
        path.remove_file()?;
    }
    Ok(())
}
