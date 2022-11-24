use crate::process::Process;
use anyhow::Result;
use std::io::Write;

pub async fn touch(process: &mut Process, args: Vec<String>) -> Result<()> {
    if args.len() < 2 {
        process.stderr.write_all(b"touch: missing file operand\n")?;
        return Ok(());
    }
    for arg in args.into_iter().skip(1) {
        process.get_path(arg)?.create_file()?;
    }
    Ok(())
}
