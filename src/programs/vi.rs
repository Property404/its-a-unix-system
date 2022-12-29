use crate::{
    process::{ExitCode, Process},
    programs::common::readline::{NullHistory, Readline},
    streams::{InputMode, InputStream, OutputStream},
    utils, AnsiCode, ControlChar,
};
use anyhow::{anyhow, Result};
use ascii::{AsciiChar, ToAsciiChar};
use clap::Parser;
use std::io::{Read, Write};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Insert,
    Normal,
}

async fn error(stdin: &mut InputStream, stdout: &mut OutputStream, message: &str) -> Result<()> {
    stdout.write_all(&AnsiCode::Clear.to_bytes())?;
    stdout.write_all(message.as_bytes())?;
    stdout.write_all(b"\n\nPress any key to continue\n")?;
    stdin.get_char().await?;
    Ok(())
}

/// Visual file editor.
///
/// Press the 'i' key to go into "input mode"
/// Press <esc> to go into "normal mode"
///
/// Use the arrow keys to navigate in either mode.
///
/// Save and quit: <esc> :wq
/// Quit without saving: <esc> :q
#[derive(Parser)]
#[command(verbatim_doc_comment)]
struct Options {
    /// The file to edit.
    file: Option<String>,
}

pub async fn vi(process: &mut Process) -> Result<ExitCode> {
    let height = utils::js_term_get_screen_height();
    let mut options = Options::try_parse_from(&process.args)?;

    let mut stdin = process.stdin.clone();
    stdin.set_mode(InputMode::Char).await?;
    let mut stdout = process.stdout.clone();

    let mut buffers: Vec<String> = if let Some(file) = &options.file {
        let mut contents = String::new();
        let file = process.get_path(file)?;
        if file.exists()? {
            let mut file = file.open_file()?;
            file.read_to_string(&mut contents)?;
            contents.trim_end().split('\n').map(|s| s.into()).collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    stdout.write_all(&AnsiCode::Clear.to_bytes())?;

    let mut mode = Mode::Normal;
    let mut offset = 0;
    let mut row = 0;
    let mut column = 0;

    let mut readline = Readline::new(NullHistory::default());

    for (i, buffer) in buffers.iter().enumerate() {
        stdout.write_all(&AnsiCode::AbsolutePosition(i, column).to_bytes())?;
        stdout.write_all(buffer.as_bytes())?;
        if i == height - 1 {
            break;
        }
    }

    let mut reset = false;
    loop {
        if buffers.is_empty() {
            buffers.push(String::new());
        }

        let mut buffer = buffers
            .get(row)
            .ok_or_else(|| anyhow!("no such row"))?
            .clone();

        if reset {
            stdout.write_all(&AnsiCode::Clear.to_bytes())?;
            let end = std::cmp::min(offset + height, buffers.len());
            for (i, buffer) in buffers[offset..end].iter().enumerate() {
                stdout.write_all(&AnsiCode::AbsolutePosition(i, 0).to_bytes())?;
                stdout.write_all(buffer.as_bytes())?;
            }
            stdin.set_mode(InputMode::Char).await?;

            reset = false;
        }

        stdout.write_all(&AnsiCode::AbsolutePosition(row - offset, 0).to_bytes())?;
        stdout.write_all(&AnsiCode::ClearLine.to_bytes())?;
        stdout.write_all(buffer.as_bytes())?;
        row = std::cmp::min(row, buffers.len());
        column = std::cmp::min(
            column,
            if mode == Mode::Normal {
                buffer.len().saturating_sub(1)
            } else {
                buffer.len()
            },
        );
        stdout.write_all(&AnsiCode::AbsolutePosition(row - offset, column).to_bytes())?;
        stdout.flush()?;
        let c = stdin.get_char().await?;

        if c == AsciiChar::ESC {
            match stdin.get_char().await?.to_ascii_char()? {
                AsciiChar::BracketOpen => match stdin.get_char().await? {
                    // Up/Down arrow
                    mode @ ('A' | 'B' | 'C' | 'D') => {
                        if mode == 'A' {
                            row = row.saturating_sub(1);
                        } else if mode == 'B' && row < buffers.len() - 1 {
                            row += 1;
                        } else if mode == 'C' {
                            column += 1
                        } else if mode == 'D' {
                            column = column.saturating_sub(1);
                        }
                    }
                    _ => continue,
                },
                AsciiChar::ESC => {
                    if mode == Mode::Insert {
                        column = column.saturating_sub(1);
                        mode = Mode::Normal;
                    }
                }
                _ => continue,
            }
        }

        if c == ControlChar::A {
            column = 0
        } else if c == ControlChar::E {
            column = buffer.len();
        } else if mode == Mode::Insert {
            if c == AsciiChar::BackSpace {
                if column > 0 {
                    column -= 1;
                    buffer.remove(column);
                    *buffers.get_mut(row).ok_or_else(|| anyhow!("No such row"))? = buffer;
                // Merge this line with previous
                } else if row > 0 {
                    reset = true;
                    buffers.remove(row);
                    let prev = buffers
                        .get_mut(row - 1)
                        .ok_or_else(|| anyhow!("No such row"))?;
                    column = prev.len();
                    row -= 1;
                    prev.push_str(&buffer);
                }
            } else if c == ControlChar::D {
                column = column.saturating_sub(1);
                mode = Mode::Normal;
            } else if c == AsciiChar::LineFeed {
                *buffers.get_mut(row).ok_or_else(|| anyhow!("No such row"))? =
                    buffer[0..column].into();
                row += 1;
                buffers.insert(row, buffer[column..].into());
                column = 0;
                reset = true;
            } else if !c.is_control() {
                buffer.insert(column, c);
                column += 1;
                *buffers.get_mut(row).ok_or_else(|| anyhow!("No such row"))? = buffer;
            }
        } else if c == 'i' || c == 'I' {
            mode = Mode::Insert;
            if c == 'I' {
                column = 0;
            }
        } else if c == 'a' {
            mode = Mode::Insert;
            column = std::cmp::min(buffer.len(), column + 1);
        } else if c == 'A' {
            column = buffer.len();
            mode = Mode::Insert;
        } else if c == 'H' {
            row = offset;
        } else if c == 'L' {
            row = std::cmp::min(buffers.len() - 1, offset + height - 1);
        } else if c == 'x' || c == 'r' || c == 's' {
            if column < buffer.len() {
                buffer.remove(column);
                if c == 'r' {
                    buffer.insert(column, stdin.get_char().await?);
                }
                *buffers.get_mut(row).ok_or_else(|| anyhow!("No such row"))? = buffer;
            }
            if c == 's' {
                mode = Mode::Insert;
            }
        } else if c == '$' {
            column = buffer.len();
        } else if c == 'G' {
            column = 0;
            row = buffers.len() - 1;
            reset = true;
        } else if c == 'g' {
            if stdin.get_char().await? == 'g' {
                column = 0;
                row = 0;
                reset = true;
            }
        } else if c == 'f' {
            let target = stdin.get_char().await?;
            if column < buffer.len() {
                column += buffer[column + 1..]
                    .find(target)
                    .map(|x| x + 1)
                    .unwrap_or(0);
            }
        } else if c == 'F' {
            let target = stdin.get_char().await?;
            column = buffer[0..column].rfind(target).unwrap_or(column);
        } else if c == 'd' {
            let next = stdin.get_char().await?;
            if next == 'd' {
                buffers.remove(row);
                row = std::cmp::min(row, buffers.len().saturating_sub(1));
            }
            reset = true;
        } else if c == 'o' || c == 'O' {
            column = 0;
            if c == 'o' {
                row = std::cmp::min(buffers.len(), row + 1);
            }
            buffers.insert(row, String::new());
            mode = Mode::Insert;
            reset = true;
        } else if c == '0' || c == '^' {
            column = 0;
        } else if c == 'k' {
            row = row.saturating_sub(1);
        } else if c == 'h' {
            column = column.saturating_sub(1);
        } else if c == 'j' && row < buffers.len() - 1 {
            row += 1;
        } else if c == 'l' && column < buffer.len() {
            column += 1;
        // Move forward one word
        } else if c == 'w' || c == 'W' {
            if column == buffer.len().saturating_sub(1) {
                if row < buffers.len().saturating_sub(1) {
                    column = 0;
                    row += 1;
                }
            } else {
                let mut hit_delim = false;
                for letter in buffer[column..].chars() {
                    let is_delim = letter.is_whitespace();
                    if is_delim {
                        hit_delim = true
                    } else if hit_delim {
                        break;
                    }
                    column += 1;
                }
            }
        // Move backward one word
        } else if c == 'b' || c == 'B' {
            if column == 0 {
                if row > 0 {
                    row -= 1;
                    column = buffers[row].len();
                }
            } else {
                let mut hit_delim = false;
                for letter in buffer[..column].chars().rev() {
                    let is_delim = letter.is_whitespace();
                    if is_delim {
                        hit_delim = true
                    } else if hit_delim {
                        break;
                    }
                    column -= 1;
                }
            }
        } else if c == ':' {
            reset = true;

            // Get command
            stdout.write_all(&AnsiCode::AbsolutePosition(height, 0).to_bytes())?;
            let command = readline
                .get_line(":", &mut stdin, &mut stdout, |_, _| Ok(Default::default()))
                .await?;
            stdin.set_mode(InputMode::Char).await?;
            let command: Vec<_> = command.split_whitespace().collect();

            if command.is_empty() {
                /* Do nothing */
            } else if "write".starts_with(command[0]) || command[0] == "wq" {
                // Set file name if non exists yet.
                if options.file.is_none() {
                    if let Some(name) = command.get(1) {
                        options.file = Some(name.to_string());
                    } else {
                        error(&mut stdin, &mut stdout, "No file name").await?;
                        continue;
                    }
                }
                let file_to_save = command
                    .get(1)
                    .map(|s| String::from(*s))
                    .or_else(|| options.file.clone())
                    .expect("BUG: file name should have been set in previous line");

                // save
                let contents = buffers.join("\n") + "\n";
                let mut file = process.get_path(file_to_save)?.create_file()?;
                file.write_all(contents.as_bytes())?;

                if command[0] == "wq" {
                    break;
                }
            } else if "quit".starts_with(command[0]) {
                if command.len() > 1 {
                    error(&mut stdin, &mut stdout, "Unexpected arguments").await?;
                } else {
                    break;
                }
            } else {
                error(
                    &mut stdin,
                    &mut stdout,
                    format!("Unknown command: {}", command[0]).as_str(),
                )
                .await?;
            }
        }

        while row < offset {
            offset -= 1;
            stdout.write_all(&AnsiCode::PopBottom.to_bytes())?;
            stdout.write_all(&AnsiCode::PushTop.to_bytes())?;
        }

        while row - offset >= height {
            offset += 1;
            stdout.write_all(&AnsiCode::PopTop.to_bytes())?;
        }
    }

    stdout.write_all(&AnsiCode::Clear.to_bytes())?;
    Ok(ExitCode::SUCCESS)
}
