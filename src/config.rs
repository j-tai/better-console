use rustbox::Color as RbColor;
use rustbox::RB_BOLD;
use rustbox::RB_NORMAL;
use rustbox::RB_REVERSE;
use rustbox::RB_UNDERLINE;
use rustbox::Style;
use serde::de;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub colors: Colors,
    pub trun_left: String,
    pub trun_right: String,
    pub default_status: String,
    pub vertical_move: isize,
    pub horizontal_move: isize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            colors: Colors::default(),
            trun_left: "<".to_string(),
            trun_right: ">".to_string(),
            default_status: "ctrl-Q = quit; arrows/PgUp/PgDn/End = scroll; type to enter command".to_string(),
            vertical_move: 1,
            horizontal_move: 16,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Colors {
    pub command: Color,
    pub file_header: Color,
    pub time: Color,
    pub info: Color,
    pub warn: Color,
    pub error: Color,
    pub severe: Color,
    pub fatal: Color,
    pub other: Color,
    pub text: Color,
    pub truncate: Color,
    pub prompt: Color,
    pub status: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Colors {
            command: Color::new(0, 0, RB_BOLD),
            file_header: Color::new(0, 0, RB_BOLD),
            time: Color::new(0, 0, RB_NORMAL),
            info: Color::new(0, 0, RB_NORMAL),
            warn: Color::new(0, 0, RB_NORMAL),
            error: Color::new(0, 0, RB_NORMAL),
            severe: Color::new(0, 0, RB_NORMAL),
            fatal: Color::new(0, 0, RB_NORMAL),
            other: Color::new(0, 0, RB_NORMAL),
            text: Color::new(0, 0, RB_NORMAL),
            truncate: Color::new(0, 0, RB_REVERSE),
            prompt: Color::new(0, 0, RB_REVERSE),
            status: Color::new(0, 0, RB_BOLD),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub fg: RbColor,
    pub bg: RbColor,
    pub sty: Style,
}

impl Color {
    pub fn new(fg: u16, bg: u16, sty: Style) -> Color {
        Color { fg: RbColor::Byte(fg), bg: RbColor::Byte(bg), sty }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::new(0, 0, RB_NORMAL)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct ColorVisitor;
        impl<'a> Visitor<'a> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.write_str("a color")
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                // Parse the color contained in 's'
                let mut parts = s.split_whitespace().fuse();
                let fg = if let Some(text) = parts.next() {
                    match text.parse::<u16>() {
                        Ok(i) => RbColor::Byte(i),
                        Err(_) => return Err(E::custom(format!("invalid fg color or out of range: '{}'", text))),
                    }
                } else { RbColor::Byte(0) }; // Byte(0) = default
                let bg = if let Some(text) = parts.next() {
                    match text.parse::<u16>() {
                        Ok(i) => RbColor::Byte(i),
                        Err(_) => return Err(E::custom(format!("invalid bg color or out of range: '{}'", text))),
                    }
                } else { RbColor::Byte(0) }; // Byte(0) = default
                let sty = if let Some(text) = parts.next() {
                    let text: &str = text.trim();
                    let mut sty = RB_NORMAL;
                    for ch in text.chars() {
                        sty = sty | match ch {
                            'b' => RB_BOLD,
                            'u' => RB_UNDERLINE,
                            'r' => RB_REVERSE,
                            _ => return Err(E::custom(format!("invalid style character: '{}'", ch))),
                        }
                    }
                    sty
                } else { RB_NORMAL };
                Ok(Color { fg, bg, sty })
            }
        }
        de.deserialize_str(ColorVisitor)
    }
}
