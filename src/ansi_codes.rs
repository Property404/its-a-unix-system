use std::string::ToString;

#[allow(unused)]
#[derive(Copy, Clone)]
/// Various ANSI escape sequences.
pub enum AnsiCode {
    CursorUp,
    CursorDown,
    CursorRight,
    CursorLeft,
    CursorResetColumn,
    ClearLine,
}

impl AnsiCode {
    /// Get byte representation of ANSI code.
    pub fn to_bytes(self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl ToString for AnsiCode {
    fn to_string(&self) -> String {
        match self {
            AnsiCode::CursorUp => "\x1b[A".into(),
            AnsiCode::CursorDown => "\x1b[B".into(),
            AnsiCode::CursorRight => "\x1b[C".into(),
            AnsiCode::CursorLeft => "\x1b[D".into(),
            AnsiCode::CursorResetColumn => "\x1b[G".into(),
            AnsiCode::ClearLine => "\x1b[2K".into(),
        }
    }
}
