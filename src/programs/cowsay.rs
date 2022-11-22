use crate::process::Process;
use anyhow::Result;
use futures::io::AsyncReadExt;
use std::io::Write;

const MAX_WIDTH: usize = 40;

pub async fn cowsay(process: &mut Process, args: Vec<String>) -> Result<()> {
    let text = if args.len() == 1 {
        let mut text = String::new();
        process.stdin.read_to_string(&mut text).await?;
        text
    } else {
        args.into_iter().skip(1).collect::<Vec<_>>().join(" ")
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
