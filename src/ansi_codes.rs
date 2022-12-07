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
    Clear,
    ClearLine,
    ClearToEndOfLine,
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
            AnsiCode::Clear => "\x1b[c".into(),
            AnsiCode::ClearLine => "\x1b[2K".into(),
            AnsiCode::ClearToEndOfLine => "\x1b[0K".into(),
        }
    }
}

/// Represents a control character.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ControlChar {
    A = 1,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

impl PartialEq<char> for ControlChar {
    fn eq(&self, c: &char) -> bool {
        (*self as u8) == (*c as u8)
    }
}
impl PartialEq<ControlChar> for char {
    fn eq(&self, c: &ControlChar) -> bool {
        (*self as u8) == (*c as u8)
    }
}
