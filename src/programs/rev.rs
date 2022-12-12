use crate::{
    process::{ExitCode, Process},
    streams::{file_redirect_in, InputStream, OutputStream},
};
use anyhow::Result;
use clap::Parser;
use futures::{io::AsyncWriteExt, try_join};

/// Reverse lines characterwise.
#[derive(Parser)]
struct Options {
    /// The files to reverse.
    files: Vec<String>,
}

pub async fn rev_inner(stream: &mut InputStream, out: &mut OutputStream) -> Result<()> {
    while let Ok(line) = stream.get_line().await {
        let line: String = line.chars().rev().collect();
        out.write_all(line.as_bytes()).await?;
        out.write_all(b"\n").await?;
    }

    Ok(())
}

pub async fn rev(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;

    if options.files.is_empty() {
        let mut stdin = process.stdin.clone();
        rev_inner(&mut stdin, &mut process.stdout).await?;
    } else {
        for file in options.files {
            let fp = process.get_path(file)?.open_file()?;
            let (mut fp, mut backend) = file_redirect_in(Box::new(fp));
            try_join! {
                async {
                    rev_inner(&mut fp, &mut process.stdout).await?;
                    fp.shutdown().await?;
                    Ok(())
                },
                backend.run()
            }?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
