use crate::process::{ExitCode, Process};
use anyhow::Result;
use ascii::AsciiChar;
use clap::Parser;
use futures::io::AsyncWriteExt;

/// Echo args to standard out.
#[derive(Parser)]
struct Options {
    /// Do not append a newline.
    #[arg(short)]
    no_newline: bool,
    /// Interpret escape sequences
    #[arg(short)]
    escapes: bool,
    /// The arguments to echo.
    args: Vec<String>,
}

pub async fn echo(process: &mut Process) -> Result<ExitCode> {
    let options = Options::try_parse_from(process.args.iter())?;
    let mut args = options.args.into_iter();
    let mut ends_with_line_feed = false;

    if let Some(item) = args.next() {
        let item = if options.escapes {
            unescape(&item)
        } else {
            item
        };
        process.stdout.write_all(item.as_bytes()).await?;
        ends_with_line_feed = item.ends_with('\n');
    }
    for item in args {
        let item = if options.escapes {
            unescape(&item)
        } else {
            item
        };
        process.stdout.write_all(b" ").await?;
        process.stdout.write_all(item.as_bytes()).await?;
        ends_with_line_feed = item.ends_with('\n');
    }

    // echo on Linux seems to not print an extra newline if stuff-to-be-echoed ends with a newline
    // already. Which kind of makes sense from a usabilty perspective.
    // Let's emulate that.
    if !options.no_newline && !ends_with_line_feed {
        process.stdout.write_all(b"\n").await?;
    }

    Ok(ExitCode::SUCCESS)
}

// Unescape an escaped string.
fn unescape(escaped: &str) -> String {
    let mut escaped = escaped.chars();
    let mut unescaped = String::new();
    loop {
        let Some(c) = escaped.next() else { break };
        if c == '\\' {
            let c = match escaped.next().unwrap_or('\\') {
                'e' => AsciiChar::ESC.as_char(),
                'b' => AsciiChar::BackSpace.as_char(),
                't' => '\t',
                '0' => '\0',
                'r' => '\r',
                'n' => '\n',
                '\\' => '\\',
                'x' => {
                    let value = 16 * escaped.next().and_then(|c| c.to_digit(16)).unwrap_or(0)
                        + escaped.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                    if let Some(c) = char::from_u32(value) {
                        unescaped.push(c);
                    }
                    continue;
                }
                'u' => {
                    let mut value: u32 = 0;
                    for _ in 0..8 {
                        value *= 16;
                        value += escaped.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                    }
                    if let Some(c) = char::from_u32(value) {
                        unescaped.push(c);
                    }
                    continue;
                }
                x => x,
            };
            unescaped.push(c);
        } else {
            unescaped.push(c);
        }
    }

    unescaped
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn escape_sequences() {
        assert_eq!(unescape("h\\\\ello\\nworld\\t"), "h\\ello\nworld\t");

        assert_eq!(unescape("\\x08"), "\x08");
        assert_eq!(unescape("\\x34"), "\x34");

        // Edge case: Ending in backslash
        assert_eq!(unescape("hi\\"), "hi\\");
        assert_eq!(unescape("hi\\\\"), "hi\\");
    }
}
