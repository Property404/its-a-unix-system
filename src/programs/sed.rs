use crate::{
    process::{ExitCode, Process},
    streams::{file_redirect_in, InputStream, OutputStream},
};
use anyhow::Result;
use clap::Parser;
use futures::try_join;
use sedregex::ReplaceCommand;
use std::io::Write;

/// Stream edit by regex
#[derive(Parser)]
struct Options {
    /// The regex pattern to use.
    pattern: String,
    /// The files to filter.
    files: Vec<String>,
}

async fn sed_inner<'a>(
    stream: &mut InputStream,
    out: &mut OutputStream,
    pattern: &ReplaceCommand<'a>,
) -> Result<()> {
    while let Ok(line) = stream.get_line().await {
        let line = pattern.execute(&line);
        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
    }

    Ok(())
}

pub async fn sed(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let pattern = ReplaceCommand::new(&options.pattern)?;

    if options.files.is_empty() {
        let mut stdin = process.stdin.clone();
        sed_inner(&mut stdin, &mut process.stdout, &pattern).await?;
    } else {
        for file in options.files {
            let fp = process.get_path(file)?.open_file()?;
            let (mut fp, mut backend) = file_redirect_in(Box::new(fp));
            try_join! {
                async {
                    sed_inner(&mut fp, &mut process.stdout, &pattern).await?;
                    fp.shutdown().await?;
                    Ok(())
                },
                backend.run()
            }?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
