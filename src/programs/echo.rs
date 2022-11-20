use crate::process::Process;
use anyhow::Result;
use futures::io::AsyncWriteExt;

pub async fn echo(process: &mut Process, args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter().skip(1);
    if let Some(first_argument) = args.next() {
        process.stdout.write_all(first_argument.as_bytes()).await?;
    }
    for item in args {
        process.stdout.write_all(b" ").await?;
        process.stdout.write_all(item.as_bytes()).await?;
    }
    process.stdout.write_all(b"\n").await?;

    Ok(())
}
