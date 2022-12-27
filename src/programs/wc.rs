use crate::{
    process::{ExitCode, Process},
    streams::{file_redirect_in, InputStream, OutputStream},
};
use anyhow::Result;
use clap::Parser;
use futures::try_join;
use std::io::Write;

/// Print line, word, and byte counts for a file.
#[derive(Parser)]
struct Options {
    /// Print the byte count.
    #[arg(short = 'c', long)]
    bytes: bool,
    /// Print the word count.
    #[arg(short, long)]
    words: bool,
    /// Print the line count.
    #[arg(short, long)]
    lines: bool,
    /// The files to count.
    files: Vec<String>,
}

async fn wc_inner<'a>(
    stream: &mut InputStream,
    out: &mut OutputStream,
    options: &Options,
) -> Result<()> {
    let mut lines = 0;
    let mut words = 0;
    let mut bytes = 0;
    while let Ok(line) = stream.get_line().await {
        bytes += line.len();
        words += line.split_whitespace().count();
        lines += 1;
    }
    let mut stats = Vec::new();
    if options.lines {
        stats.push(format!("{lines}"));
    }
    if options.words {
        stats.push(format!("{words}"));
    }
    if options.bytes {
        stats.push(format!("{bytes}"));
    }
    out.write_all(format!("{}\n", stats.join(" ")).as_bytes())?;
    Ok(())
}

pub async fn wc(process: &mut Process) -> Result<ExitCode> {
    let mut options = Options::try_parse_from(process.args.iter())?;

    if !(options.bytes || options.words || options.lines) {
        options.bytes = true;
        options.words = true;
        options.lines = true;
    }

    if options.files.is_empty() {
        let mut stdin = process.stdin.clone();
        wc_inner(&mut stdin, &mut process.stdout, &options).await?;
    } else {
        for file in &options.files {
            let fp = process.get_path(file)?.open_file()?;
            let (mut fp, mut backend) = file_redirect_in(Box::new(fp));
            try_join! {
                async {
                    wc_inner(&mut fp, &mut process.stdout, &options).await?;
                    fp.shutdown().await?;
                    Ok(())
                },
                backend.run()
            }?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
