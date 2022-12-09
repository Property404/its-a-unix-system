use crate::process::Process;
use anyhow::Result;
use clap::Parser;
use futures::io::AsyncReadExt;
use std::io::Write;

const MAX_WIDTH: usize = 40;

/// Have a cow say things
#[derive(Parser)]
struct Options {
    /// The things to say.
    args: Vec<String>,
}

pub async fn cowsay(process: &mut Process) -> Result<()> {
    let options = Options::try_parse_from(process.args.iter())?;
    let text = if options.args.is_empty() {
        let mut text = String::new();
        process.stdin.read_to_string(&mut text).await?;
        text
    } else {
        options.args.into_iter().collect::<Vec<_>>().join(" ")
    };

    let lines = textwrap::wrap(text.trim(), MAX_WIDTH);
    let mut width = 0;
    for line in lines.iter() {
        if line.len() > width {
            width = line.len();
        }
    }

    process.stdout.write_all(b" ")?;
    for _ in 0..width + 2 {
        process.stdout.write_all(b"_")?;
    }
    process.stdout.write_all(b"\n")?;

    for (index, line) in lines.iter().enumerate() {
        process.stdout.write_all(if lines.len() > 1 {
            if index == 0 {
                b"/"
            } else if index == lines.len() - 1 {
                b"\\"
            } else {
                b"|"
            }
        } else {
            b"<"
        })?;
        process.stdout.write_all(b" ")?;
        process.stdout.write_all(line.as_bytes())?;
        for _ in 0..=width - line.len() {
            process.stdout.write_all(b" ")?;
        }
        process.stdout.write_all(if lines.len() > 1 {
            if index == 0 {
                b"\\"
            } else if index == lines.len() - 1 {
                b"/"
            } else {
                b"|"
            }
        } else {
            b">"
        })?;
        process.stdout.write_all(b"\n")?;
    }
    process.stdout.write_all(b" ")?;
    for _ in 0..width + 2 {
        process.stdout.write_all(b"-")?;
    }

    process.stdout.write_all(
        b"
         \\    ^__^
          \\   (oo)\\_______
              (__)\\       )\\/\\
                  ||----w |
                  ||     ||
",
    )?;
    Ok(())
}
