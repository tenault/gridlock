// ╭───────────────────────────────────────────────────────────────io/color.rs─╮
// │                                                                           │
// │                                ┏━┓    ┏━━┓              ┏━┓               │
// │                                ┃ ┃    ┗┓ ┃              ┃ ┃               │
// │           ┏━━━┓┏┓┏━━━━━┓┏━┓┏━━━┛ ┃     ┃ ┃┏━━━━━┓┏━━━━━┓┃ ┃┏━━┓           │
// │           ┃ ┏━┓ ┃┃ ┏━━━┛┃ ┃┃ ┏━┓ ┃     ┃ ┃┃ ┏━┓ ┃┃ ┏━━━┛┃ ┗┛┏━┛           │
// │           ┃ ┗━┛ ┃┃ ┃    ┃ ┃┃ ┗━┛ ┃ ┏━┓ ┃ ┃┃ ┗━┛ ┃┃ ┗━━━┓┃ ┏┓┗━┓           │
// │           ┗━━━┓ ┃┗━┛    ┗━┛┗━━━┛┗┛ ┗━┛ ┗━━┛┗━━━━┛┗━━━━━┛┗━┛┗━━┛           │
// │           ┏━━━┛ ┃ ////////////////////////////////////////////            │
// │           ┗━━━━━┛                                                         │
// │                                                                           │
// │                copyright (c) 2026 Malakai Smith (@tenault)                │
// │                                                                           │
// │    This Source Code Form is subject to the terms of the Mozilla Public    │
// │    License, v. 2.0. If a copy of the MPL was not distributed with this    │
// │         file, You can obtain one at https://mozilla.org/MPL/2.0.          │
// │                                                                           │
// ╰───────────────────────────────────────────────────────────────────────────╯

// ╭─────────────╮
// │    COLOR    │
// ╰─────────────╯

/// Represents all forms of a terminal color
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Terminal default
    Default,

    /// 256-color palette index
    ///
    /// - `0-15`:    Standard ANSI + bright variants
    /// - `16-231`:  6x6x6 RGB color cube
    /// - `232-255`: Grayscale ramp
    Indexed(u8),

    /// 24-bit RGB truecolor
    RGB(u8, u8, u8),
}

impl Color {

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    constructors    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    // ───── BASE ─────

    pub const fn default()                -> Self { Self::Default      }
    pub const fn indexed(n: u8)           -> Self { Self::Indexed(n)   }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self::RGB(r, g, b) }

    // ───── ANSI-16 ─────

    pub const fn black()   -> Self { Self::Indexed(0) }
    pub const fn red()     -> Self { Self::Indexed(1) }
    pub const fn green()   -> Self { Self::Indexed(2) }
    pub const fn yellow()  -> Self { Self::Indexed(3) }
    pub const fn blue()    -> Self { Self::Indexed(4) }
    pub const fn magenta() -> Self { Self::Indexed(5) }
    pub const fn cyan()    -> Self { Self::Indexed(6) }
    pub const fn white()   -> Self { Self::Indexed(7) }

    pub const fn bright_black()   -> Self { Self::Indexed(8)  }
    pub const fn bright_red()     -> Self { Self::Indexed(9)  }
    pub const fn bright_green()   -> Self { Self::Indexed(10) }
    pub const fn bright_yellow()  -> Self { Self::Indexed(11) }
    pub const fn bright_blue()    -> Self { Self::Indexed(12) }
    pub const fn bright_magenta() -> Self { Self::Indexed(13) }
    pub const fn bright_cyan()    -> Self { Self::Indexed(14) }
    pub const fn bright_white()   -> Self { Self::Indexed(15) }

    // ───── GRAYSCALE ─────

    pub fn gray(level: u8) -> Self { Self::Indexed(232 + level.min(23)) }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    conversions    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    // HSL <-> RGB and much more coming in a future update...
}


// ╭──────────────────╮
// │    EXTENSIONS    │
// ╰──────────────────╯

impl Default for Color {
    fn default() -> Self { Self::Default }
}
