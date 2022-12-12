use crate::{
    process::{ExitCode, Process},
    streams::file_redirect_out,
};
use anyhow::Result;
use clap::Parser;
use futures::{future::try_join_all, try_join};
use std::io::Write;

/// Read from stdin and write to stdout and file.
#[derive(Parser)]
struct Options {
    /// The files to which to write
    files: Vec<String>,
}

pub async fn tee(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut outs = Vec::new();
    let mut backends = Vec::new();

    for file in options.files {
        let fp = process.get_path(file)?.create_file()?;
        let (out, backend) = file_redirect_out(Box::new(fp));
        outs.push(out);
        backends.push(backend);
    }

    try_join! {
        async {
            while let Ok(line) = process.stdin.get_line().await {
                process.stdout.write_all(line.as_bytes())?;
                process.stdout.write_all(b"\n")?;
                for out in &mut outs {
                    out.write_all(line.as_bytes())?;
                    out.write_all(b"\n")?;
                }
            }
            for out in &mut outs {
                out.shutdown().await?;
            }
            Ok(())
        },
        async {
            let mut futures = Vec::new();
            for backend in &mut backends {
                futures.push(backend.run());
            }
            try_join_all(futures).await?;
            Ok::<(), anyhow::Error>(())
        }
    }?;

    Ok(ExitCode::SUCCESS)
}
