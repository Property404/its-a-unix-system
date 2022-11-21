use crate::process::Process;
use anyhow::Result;
use std::io::Write;

pub async fn touch(process: &mut Process, args: Vec<String>) -> Result<()> {
    if args.len() < 2 {
        process.stdout.write_all(b"touch: missing file_operand\n")?;
        return Ok(());
    }
    for arg in args.into_iter().skip(1) {
        process.cwd.join(arg)?.create_file()?;
    }
    Ok(())
}
