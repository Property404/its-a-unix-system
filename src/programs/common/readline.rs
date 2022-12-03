use crate::{
    streams::{InputMode, InputStream, OutputStream},
    AnsiCode,
};
use anyhow::Result;
use futures::io::AsyncWriteExt;

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

/// A GNU Readline-like implementation.
pub struct Readline {
    /// The prompt to show, e.g "$ "
    prompt: String,
}

impl Readline {
    pub fn new(prompt: String) -> Self {
        Self { prompt }
    }
    /// Get next line.
    pub async fn get_line(
        &self,
        stdin: &mut InputStream,
        stdout: &mut OutputStream,
    ) -> Result<String> {
        stdin.set_mode(InputMode::Char).await?;

        let result = self.get_line_inner(stdin, stdout).await;

        stdin.set_mode(InputMode::Line).await?;

        result
    }

    async fn get_line_inner(
        &self,
        stdin: &mut InputStream,
        stdout: &mut OutputStream,
    ) -> Result<String> {
        let mut buffer = String::new();
        let mut cursor = 0;

        stdout.write_all(self.prompt.as_bytes()).await?;
        loop {
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
                    'C' => {
                        if cursor < buffer.len() {
                            move_cursor_right(stdout, 1).await?;
                            cursor += 1;
                        }
                    }
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
            // Newline (\n or ^J)
            } else if c == '\x0A' {
                // An interesting bug appears without this next line.
                // The character behind the cursor will be deleted!
                // The bug probably lies in term.js
                stdout
                    .write_all(&AnsiCode::CursorResetColumn.to_bytes())
                    .await?;
                stdout.write_all(b"\n").await?;
                return Ok(buffer);
            // Backspace
            } else if c == '\x08' {
                if cursor > 0 {
                    cursor -= 1;
                    buffer.remove(cursor);
                    move_cursor_left(stdout, 1).await?;
                }
            } else {
                buffer.insert(cursor, c);
                cursor += 1;
                move_cursor_right(stdout, 1).await?;
            }
        }
    }
}
