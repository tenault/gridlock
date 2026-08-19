// ╭───────────────────────────────────────────────────────────────io/style.rs─╮
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

// ╭───────────────────╮
// │    ENVIRONMENT    │
// ╰───────────────────╯

use super::color::Color;
use super::escape::{Escapable, StyleContext};


// ╭───────────────╮
// │    SYMBOLS    │
// ╰───────────────╯

pub(crate) mod attr {
    pub(crate) const BOLD:          u16 = 1 << 0;
    pub(crate) const DIM:           u16 = 1 << 1;
    pub(crate) const ITALIC:        u16 = 1 << 2;
    pub(crate) const UNDERLINE:     u16 = 1 << 3;
    pub(crate) const BLINK:         u16 = 1 << 4;
    pub(crate) const FAST_BLINK:    u16 = 1 << 5;
    pub(crate) const REVERSE:       u16 = 1 << 6;
    pub(crate) const CONCEALED:     u16 = 1 << 7;
    pub(crate) const STRIKETHROUGH: u16 = 1 << 8;
}


// ╭───────────╮
// │    SGR    │
// ╰───────────╯

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SGR {
    pub bold:          bool,
    pub dim:           bool,
    pub italic:        bool,
    pub underline:     bool,
    pub blink:         bool,
    pub fast_blink:    bool,
    pub reverse:       bool,
    pub concealed:     bool,
    pub strikethrough: bool,
    pub fg: Color,
    pub bg: Color,
}

impl SGR {

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    constructor    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    pub fn new() -> Self { Self::default() }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    delta    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    pub(crate) fn delta_from(&self, prev: &SGR) -> Option<Vec<u8>> {
        if self == prev { return None; }

        let self_attrs = self._attrs_as_mask();
        let prev_attrs = prev._attrs_as_mask();

        let on_attrs  = self_attrs & !prev_attrs;
        let off_attrs = prev_attrs & !self_attrs;

        // heuristic: reset + re-emit vs delta
        if (1 + self_attrs.count_ones()) < (on_attrs.count_ones() + off_attrs.count_ones()) {
            let ctx = StyleContext {
                reset: true,
                off:   None,
                on:    Some(self_attrs),
                fg: if self.fg != Color::Default { Some(self.fg) } else { None },
                bg: if self.bg != Color::Default { Some(self.bg) } else { None },
            };

            return ctx.get_escape();
        }

        let ctx = StyleContext {
            reset: false,
            off: if off_attrs != 0 { Some(off_attrs) } else { None },
            on:  if on_attrs  != 0 { Some(on_attrs)  } else { None },
            fg:  if self.fg != prev.fg { Some(self.fg) } else { None },
            bg:  if self.bg != prev.bg { Some(self.bg) } else { None },
        };

        ctx.get_escape()
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    utility    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    fn _attrs_as_mask(&self) -> u16 {
        (self.bold as u16)            << 0
        | (self.dim as u16)           << 1
        | (self.italic as u16)        << 2
        | (self.underline as u16)     << 3
        | (self.blink as u16)         << 4
        | (self.fast_blink as u16)    << 5
        | (self.reverse as u16)       << 6
        | (self.concealed as u16)     << 7
        | (self.strikethrough as u16) << 8
    }
}


// ╭─────────────────────╮
// │    SUPPORT TYPES    │
// ╰─────────────────────╯

pub struct TerminalStyle { state: SGR }

impl TerminalStyle {

    // ╶╶╶╶╶ constructor ╴╴╴╴╴

    pub fn new() -> Self { Self::default() }

    // ╶╶╶╶╶ styling ╴╴╴╴╴

    pub fn bold(mut self)          -> Self {          self.state.bold = true; self }
    pub fn dim(mut self)           -> Self {           self.state.dim = true; self }
    pub fn italic(mut self)        -> Self {        self.state.italic = true; self }
    pub fn underline(mut self)     -> Self {     self.state.underline = true; self }
    pub fn blink(mut self)         -> Self {         self.state.blink = true; self }
    pub fn fast_blink(mut self)    -> Self {    self.state.fast_blink = true; self }
    pub fn reverse(mut self)       -> Self {       self.state.reverse = true; self }
    pub fn concealed(mut self)     -> Self {     self.state.concealed = true; self }
    pub fn strikethrough(mut self) -> Self { self.state.strikethrough = true; self }

    pub fn fg(mut self, c: Color) -> Self { self.state.fg = c; self }
    pub fn bg(mut self, c: Color) -> Self { self.state.bg = c; self }

    // ╶╶╶╶╶ export ╴╴╴╴╴

    pub fn build(&self) -> SGR { self.state }
}

// ───── EXTENSIONS ─────

impl Default for TerminalStyle {
    fn default() -> Self { Self { state: SGR::new() } }
}
