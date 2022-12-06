use crate::{
    streams::{InputMode, InputStream, OutputStream},
    AnsiCode,
};
use anyhow::Result;
use futures::io::AsyncWriteExt;
use std::io::Read;
use vfs::VfsPath;

async fn move_cursor_left(stdout: &mut OutputStream, n: usize) -> Result<()> {
    for _ in 0..n {
        stdout.write_all(&AnsiCode::CursorLeft.to_bytes()).await?;
    }
    Ok(())
}

async fn move_cursor_right(stdout: &mut OutputStream, n: usize) -> Result<()> {
    for _ in 0..n {
        stdout.write_all(&AnsiCode::CursorRight.to_bytes()).await?;
    }
    Ok(())
}

/// This trait indicates that a struct can record or retrieve command history.
pub trait History {
    fn get_records(&self) -> Result<Vec<String>>;
    fn add_record(&self, record: &str) -> Result<()>;
}

/// Read and write history to/from a file.
pub struct FileBasedHistory {
    file: VfsPath,
}

impl FileBasedHistory {
    pub fn new(file: VfsPath) -> Self {
        Self { file }
    }
}

impl History for FileBasedHistory {
    fn get_records(&self) -> Result<Vec<String>> {
        if !self.file.exists()? {
            self.file.create_file()?;
        }
        let mut file = self.file.open_file()?;
        let mut records = String::new();
        file.read_to_string(&mut records)?;
        Ok(records.trim().split('\n').map(String::from).collect())
    }

    fn add_record(&self, record: &str) -> Result<()> {
        let mut file = if self.file.exists()? {
            self.file.append_file()?
        } else {
            self.file.create_file()?
        };
        std::io::Write::write_all(&mut file, record.as_bytes())?;
        std::io::Write::write_all(&mut file, b"\n")?;
        Ok(())
    }
}

/// A GNU Readline-like implementation.
pub struct Readline<T: History> {
    /// The prompt to show, e.g "$ "
    prompt: String,
    history: T,
}

impl<T: History> Readline<T> {
    pub fn new(prompt: String, history: T) -> Self {
        Self { prompt, history }
    }
    /// Get next line.
    pub async fn get_line<F>(
        &mut self,
        stdin: &mut InputStream,
        stdout: &mut OutputStream,
        completer: Option<F>,
    ) -> Result<String>
    where
        F: Fn(String, usize) -> Result<Vec<String>>,
    {
        stdin.set_mode(InputMode::Char).await?;

        let result = self.get_line_inner(stdin, stdout, completer).await;

        stdin.set_mode(InputMode::Line).await?;

        if let Ok(result) = result.as_ref() {
            self.history.add_record(result)?;
        }

        result
    }

    async fn get_line_inner<F>(
        &self,
        stdin: &mut InputStream,
        stdout: &mut OutputStream,
        completer: Option<F>,
    ) -> Result<String>
    where
        F: Fn(String, usize) -> Result<Vec<String>>,
    {
        let mut cursor = 0;
        let mut buffers = self.history.get_records()?;
        buffers.push(String::new());
        let mut buffer_index = buffers.len() - 1;

        stdout.write_all(self.prompt.as_bytes()).await?;
        loop {
            let buffer = buffers
                .get_mut(buffer_index)
                .expect("History out of bounds");

            move_cursor_left(stdout, cursor).await?;
            stdout
                .write_all(&AnsiCode::ClearToEndOfLine.to_bytes())
                .await?;
            stdout.write_all(buffer.as_bytes()).await?;
            move_cursor_left(stdout, buffer.len() - cursor).await?;
            stdout.flush().await?;

            let c = stdin.get_char().await?;
            if c == '\x1b' {
                // Throw away bracket
                let _ = stdin.get_char().await?;

                match stdin.get_char().await? {
                    // Up/Down arrow - Move up/down in history
                    mode @ ('A' | 'B') => {
                        if mode == 'A' && buffer_index > 0 {
                            buffer_index -= 1;
                        } else if mode == 'B' && buffer_index < buffers.len() - 1 {
                            buffer_index += 1
                        } else {
                            continue;
                        }

                        let len = buffers[buffer_index].len();
                        if cursor >= len {
                            move_cursor_left(stdout, cursor - len).await?;
                        } else {
                            move_cursor_right(stdout, len - cursor).await?;
                        }
                        cursor = len;
                    }
                    // Right arrow - move right
                    'C' => {
                        if cursor < buffer.len() {
                            move_cursor_right(stdout, 1).await?;
                            cursor += 1;
                        }
                    }
                    // Left arrow - move left
                    'D' => {
                        if cursor > 0 {
                            move_cursor_left(stdout, 1).await?;
                            cursor -= 1;
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // ^A
            if c == '\x01' {
                move_cursor_left(stdout, cursor).await?;
                cursor = 0;
            // ^B
            } else if c == '\x02' {
                if cursor > 0 {
                    move_cursor_left(stdout, 1).await?;
                }
                cursor = cursor.saturating_sub(1);
            // ^E
            } else if c == '\x05' {
                move_cursor_right(stdout, buffer.len() - cursor).await?;
                cursor = buffer.len();
            // ^F
            } else if c == '\x06' {
                if cursor < buffer.len() {
                    move_cursor_right(stdout, 1).await?;
                    cursor += 1;
                }
            // Tab completions
            } else if c == '\t' {
                let Some(ref completer) = completer else {continue;};
                if buffer.is_empty()
                    || cursor == 0
                    || buffer.chars().next_back().unwrap().is_whitespace()
                {
                    continue;
                }

                let start = buffer[0..cursor].rfind(' ').map(|x| x + 1).unwrap_or(0);
                let section = &buffer[0..cursor];
                let word = &section[start..];
                let mut suggestions = completer(section.into(), start)?;

                if suggestions.is_empty() {
                    continue;
                } else if suggestions.len() == 1 {
                    let suggestion = suggestions.pop().unwrap();
                    let new_cursor = cursor - word.len() + suggestion.len();
                    *buffer = format!("{}{}{}", &buffer[0..start], suggestion, &buffer[cursor..]);
                    if cursor >= new_cursor {
                        move_cursor_left(stdout, cursor - new_cursor).await?;
                    } else {
                        move_cursor_right(stdout, new_cursor - cursor).await?;
                    }
                    cursor = new_cursor;
                } else {
                    stdout.write_all(b"\n").await?;
                    for suggestion in suggestions {
                        stdout.write_all(suggestion.as_str().as_bytes()).await?;
                        stdout.write_all(b" ").await?;
                    }
                    stdout.write_all(b"\n").await?;
                    stdout.write_all(self.prompt.as_bytes()).await?;
                    move_cursor_right(stdout, cursor).await?;
                }

            // Newline (\n or ^J)
            } else if c == '\n' {
                // An interesting bug appears without this next line.
                // The character behind the cursor will be deleted!
                // The bug probably lies in term.js
                stdout
                    .write_all(&AnsiCode::CursorResetColumn.to_bytes())
                    .await?;
                stdout.write_all(b"\n").await?;
                // Todo: I think there's a way to move out of the vector instead of cloning.
                return Ok(buffer.clone());
            // Backspace
            } else if c == '\x08' {
                if cursor > 0 {
                    cursor -= 1;
                    buffer.remove(cursor);
                    move_cursor_left(stdout, 1).await?;
                }
            // Ignore unknown commands
            } else if (c as u8) < 0x20 {
                // Do nothing
            } else {
                buffer.insert(cursor, c);
                cursor += 1;
                move_cursor_right(stdout, 1).await?;
            }
        }
    }
}
