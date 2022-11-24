use anyhow::Result;
use std::io::Write;
const RESET: &str = "\u{001b}[0m";

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum Color {
    Red,
    Blue,
    Green,
}

impl Color {
    const fn as_fg(&self) -> &str {
        match self {
            Self::Red => "\u{001b}[31m",
            Self::Green => "\u{001b}[32m",
            Self::Blue => "\u{001b}[34m",
        }
    }
}

pub struct ColorPicker {
    active: bool,
    color: Option<Color>,
}

impl ColorPicker {
    pub fn new(active: bool) -> Self {
        Self {
            active,
            color: None,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = Some(color)
    }

    pub fn reset(&mut self) {
        self.color = None
    }

    pub fn write(&self, writer: &mut impl Write, text: &str) -> Result<()> {
        if self.active {
            if let Some(color) = self.color {
                writer.write_all(color.as_fg().as_bytes())?;
            }
        }
        writer.write_all(text.as_bytes())?;
        if self.active {
            writer.write_all(RESET.as_bytes())?;
        }
        Ok(())
    }
}
