use crate::{
    process::{ExitCode, Process},
    streams::{file_redirect_in, InputStream, OutputStream},
};
use anyhow::Result;
use clap::Parser;
use futures::{try_join, AsyncReadExt};
use std::io::Write;

/// Show last n lines.
#[derive(Parser)]
struct Options {
    /// How many lines to show.
    #[arg(short, default_value = "10")]
    n: usize,
    /// The file to show.
    file: Option<String>,
}

pub async fn tail_inner(
    stdin: &mut InputStream,
    stdout: &mut OutputStream,
    n: usize,
) -> Result<()> {
    let mut contents = String::new();
    stdin.read_to_string(&mut contents).await?;
    let contents: Vec<_> = contents.split('\n').collect();
    let contents = contents[contents.len().saturating_sub(n + 1)..].join("\n");
    stdout.write_all(contents.as_bytes())?;
    Ok(())
}

pub async fn tail(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    let n = options.n;

    if let Some(file) = options.file {
        let fp = process.get_path(file)?.open_file()?;
        let (mut fp, mut backend) = file_redirect_in(Box::new(fp));
        try_join! {
            async {
                tail_inner(&mut fp, &mut process.stdout, n).await?;
                fp.shutdown().await?;
                Ok(())
            },
            backend.run()
        }?;
    } else {
        let mut stdout = process.stdout.clone();
        tail_inner(&mut process.stdin, &mut stdout, n).await?;
    }

    Ok(ExitCode::SUCCESS)
}
