// ╭──────────────────────────────────────────────────────────────io/escape.rs─╮
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

use super::style::attr;
use super::color::Color;


// ╭───────────────╮
// │    SYMBOLS    │
// ╰───────────────╯

pub(crate) const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
pub(crate) const EXIT_ALT_SCREEN:  &[u8] = b"\x1b[?1049l";

const ESC: u8 = 0x1b;
const CSI: [u8; 2] = [ESC, b'['];

const CUP: u8 = b'H';
const CHA: u8 = b'G';
const VPA: u8 = b'd';
const SGR: u8 = b'm';


// ╭─────────────────────────╮
// │    ESCAPE GENERATORS    │
// ╰─────────────────────────╯

fn _build_cursor_escape(ctx: CursorContext) -> Option<Vec<u8>> {
    match (ctx.x, ctx.y) {
        (None, None)    => None,
        (Some(x), None) => Some(_build_csi(&[x + 1], CHA)),
        (None, Some(y)) => Some(_build_csi(&[y + 1], VPA)),

        (Some(x), Some(y)) => {
            if x == 0 && y == 0 { Some(_build_short_csi(CUP)) }
            else { Some(_build_csi(&[y + 1, x + 1], CUP)) }
        },
    }
}

fn _build_style_escape(ctx: StyleContext) -> Option<Vec<u8>> {
    let mut params: Vec<u8> = Vec::new();

    if ctx.reset { params.push(0); }

    if let Some(mask) = ctx.off { _extract_attr_params(mask, false, &mut params); }
    if let Some(mask) = ctx.on  { _extract_attr_params(mask, true,  &mut params); }

    if let Some(ref color) = ctx.fg { _extract_color_params(color, false, &mut params); }
    if let Some(ref color) = ctx.bg { _extract_color_params(color, true,  &mut params); }

    if params.is_empty() { None } else { Some(_build_csi(&params, SGR)) }
}


// ╭─────────────────────╮
// │    SUPPORT TYPES    │
// ╰─────────────────────╯

#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorContext {
    pub(crate) x: Option<u16>,
    pub(crate) y: Option<u16>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StyleContext {
    pub(crate) reset: bool,
    pub(crate) off:   Option<u16>,
    pub(crate) on:    Option<u16>,
    pub(crate) fg:    Option<Color>,
    pub(crate) bg:    Option<Color>,
}


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

// ───── CSI ─────

fn _build_csi<C: CSI + Copy>(params: &[C], cmd: u8) -> Vec<u8> {
    let capacity = 3                      // ESC + `[` + cmd
        + params.len() * C::MAX_DECIMALS  // Params as decimals
        + params.len().saturating_sub(1); // `;` separators

    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&CSI);

    for (i, &p) in params.iter().enumerate() {
        if i > 0 { out.push(b';'); }
        p.write_decimals(&mut out);
    }

    out.push(cmd);
    out
}

fn _build_short_csi(cmd: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(3);
    out.extend_from_slice(&CSI);
    out.push(cmd);
    out
}

fn _serialize_int(n: usize, out: &mut Vec<u8>) {
    if n == 0 {
        out.push(b'0');
        return;
    }

    let mut buf = [0u8; 20]; // u64::MAX = 18446744073709551615
    let mut i = 19;
    let mut v = n;

    while v > 0 {
        buf[i] = (v % 10) as u8 + b'0';
        v /= 10;
        i -= 1;
    }

    out.extend_from_slice(&buf[i + 1..]);
}

// ───── STYLE ─────

fn _extract_attr_params(mask: u16, enable: bool, out: &mut Vec<u8>) {
    const ATTR_CODES: &[(u16, u8, u8)] = &[
        (attr::BOLD,          1, 22), // shares disable with DIM
        (attr::DIM,           2, 22),
        (attr::ITALIC,        3, 23),
        (attr::UNDERLINE,     4, 24),
        (attr::BLINK,         5, 25),
        (attr::FAST_BLINK,    6, 25), // shares disable with BLINK
        (attr::REVERSE,       7, 27),
        (attr::CONCEALED,     8, 28),
        (attr::STRIKETHROUGH, 9, 29),
    ];

    for &(flag, on, off) in ATTR_CODES {
        if mask & flag != 0 { out.push(if enable { on } else { off }); }
    }
}

fn _extract_color_params(color: &Color, bg: bool, out: &mut Vec<u8>) {
    const BASE:   u8 = 30;
    const EXTEND: u8 = 38;
    const RESET:  u8 = 39;
    const BRIGHT: u8 = 90;

    let shift: u8 = if bg { 10 } else { 0 };

    match color {
        Color::Default => { out.push(RESET + shift); },
        Color::Indexed(n) => {
            if *n < 8 { out.push(BASE + shift + *n); }
            else if *n < 16 { out.push(BRIGHT + shift + (*n - 8)); }
            else { out.extend_from_slice(&[EXTEND + shift, 5, *n]); }
        },
        Color::RGB(r, g, b) => { out.extend_from_slice(&[EXTEND + shift, 2, *r, *g, *b]); },
    }
}


// ╭──────────────────╮
// │    EXTENSIONS    │
// ╰──────────────────╯

// ───── ESCAPABLE ─────

pub(crate) trait Escapable { fn get_escape(&self) -> Option<Vec<u8>>; }

impl Escapable for CursorContext {
    fn get_escape(&self) -> Option<Vec<u8>> { _build_cursor_escape(*self) }
}

impl Escapable for StyleContext {
    fn get_escape(&self) -> Option<Vec<u8>> { _build_style_escape(*self) }
}

// ───── CSI ─────

trait CSI { const MAX_DECIMALS: usize; fn write_decimals(&self, out: &mut Vec<u8>); }

impl CSI for u8 {
    const MAX_DECIMALS: usize = 3; // u8::MAX = 255
    fn write_decimals(&self, out: &mut Vec<u8>) { _serialize_int(*self as usize, out) }
}

impl CSI for u16 {
    const MAX_DECIMALS: usize = 5; // u16::MAX = 65535
    fn write_decimals(&self, out: &mut Vec<u8>) { _serialize_int(*self as usize, out) }
}

impl CSI for u32 {
    const MAX_DECIMALS: usize = 10; // u32::MAX = 4294967295
    fn write_decimals(&self, out: &mut Vec<u8>) { _serialize_int(*self as usize, out) }
}

impl CSI for u64 {
    const MAX_DECIMALS: usize = 20; // u64::MAX = 18446744073709551615
    fn write_decimals(&self, out: &mut Vec<u8>) { _serialize_int(*self as usize, out) }
}

impl CSI for usize {
    const MAX_DECIMALS: usize = const {
        if cfg!(target_pointer_width = "64") { 20 } else { 10 }
    };

    fn write_decimals(&self, out: &mut Vec<u8>) { _serialize_int(*self as usize, out) }
}

