// ╭─────────────────────────────────────────────────────────────cell/color.rs─╮
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

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    utility    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    /// Packs the color into a `u32` for storage in a `CellGrid` SoA lane.
    ///
    /// Because 24-bit RGB is the widest gamut terminals can display directly, any format beyond the
    /// three core models (Default, Indexed, RGB) is converted lossy.
    ///
    /// - Top byte encodes variant: `0x00` = Default, `0x01` = Indexed, `0x02` = RGB
    /// - `Default` packs `0x00000000...` (blank cell marker)
    ///
    /// ```text
    ///  32              24                                            0
    /// ╭┴───────────────┼─────────────────────────────────────────────┴╮
    /// │  <color_type>  │               ┊   (payload)   ┊              │
    /// ╰────────────────┴──────────────────────────────────────────────╯
    /// ```
    pub(crate) const fn pack(self) -> u32 {
        match self {
            Self::Default => 0,
            Self::Indexed(n) => (1u32 << 24)
                | (n as u32),
            Self::RGB(r, g, b) => (2u32 << 24)
                | ((r as u32) << 16)
                | ((g as u32) << 8)
                | (b as u32),
        }
    }

    /// Unpacks a `CellGrid` SoA `u32` lane back into a Color variant.
    pub(crate) const fn unpack(v: u32) -> Self {
        let tag = (v >> 24) as u8;
        match tag {
            0 => Self::Default,
            1 => Self::Indexed((v & 0xff) as u8),
            2 => Self::RGB(
                ((v >> 16) & 0xff) as u8,
                ((v >> 8) & 0xff) as u8,
                (v & 0xff) as u8,
            ),
            _ => Self::Default,
        }
    }
}


// ╭──────────────────╮
// │    EXTENSIONS    │
// ╰──────────────────╯

impl Default for Color {
    fn default() -> Self { Self::Default }
}
