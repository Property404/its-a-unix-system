use crate::{
    process::{ExitCode, Process},
    streams::{file_redirect_in, InputStream, OutputStream},
};
use anyhow::Result;
use clap::Parser;
use futures::try_join;
use std::io::Write;

/// Show first n lines.
#[derive(Parser)]
struct Options {
    /// How many lines to show.
    #[arg(short, default_value = "10")]
    n: usize,
    /// The file to show.
    file: Option<String>,
}

pub async fn head_inner(
    stdin: &mut InputStream,
    stdout: &mut OutputStream,
    n: usize,
) -> Result<()> {
    for _ in 0..n {
        if let Ok(line) = stdin.get_line().await {
            stdout.write_all(line.as_bytes())?;
            stdout.write_all(b"\n")?;
        } else {
            // Ignore 'unexpected end of file'
            return Ok(());
        }
    }
    Ok(())
}

pub async fn head(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    let n = options.n;

    if let Some(file) = options.file {
        let fp = process.get_path(file)?.open_file()?;
        let (mut fp, mut backend) = file_redirect_in(Box::new(fp));
        try_join! {
            async {
                head_inner(&mut fp, &mut process.stdout, n).await?;
                fp.shutdown().await?;
                Ok(())
            },
            backend.run()
        }?;
    } else {
        let mut stdout = process.stdout.clone();
        head_inner(&mut process.stdin, &mut stdout, n).await?;
    }

    Ok(ExitCode::SUCCESS)
}
