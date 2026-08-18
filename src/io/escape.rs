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
// │    DEFINITIONS    │
// ╰───────────────────╯

pub(crate) const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
pub(crate) const EXIT_ALT_SCREEN:  &[u8] = b"\x1b[?1049l";

pub(crate) trait Escapable { fn get_escape(&self) -> Option<Vec<u8>>; }

const ESC: u8 = 0x1b;
const CSI: [u8; 2] = [ESC, b'['];

trait CSI {
    const MAX_DECIMALS: usize;
    fn write_decimals(&self, out: &mut Vec<u8>);
}


// ╭─────────────────────────╮
// │    ESCAPE DISPATCHER    │
// ╰─────────────────────────╯

pub(crate) fn build(context: EscapeContext) -> Option<Vec<u8>> {
    match context {
        EscapeContext::Cursor(ctx) => _build_cursor_escape(ctx),
    }
}

fn _build_cursor_escape(ctx: CursorContext) -> Option<Vec<u8>> {
    match ctx {
        CursorContext::Null => None,
        CursorContext::Point { x, y } => {
            if x == 0 && y == 0 { Some(_build_csi::<u8>(&[], b'H')) }
            else { Some(_build_csi(&[y + 1, x + 1], b'H')) }
        },
        CursorContext::Column { x } => Some(_build_csi(&[x + 1], b'G')),
        CursorContext::Row    { y } => Some(_build_csi(&[y + 1], b'd')),
    }
}


// ╭─────────────────────╮
// │    SUPPORT TYPES    │
// ╰─────────────────────╯

#[derive(Clone, Copy, Debug)]
pub(crate) enum EscapeContext {
    Cursor(CursorContext),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CursorContext {
    Null,
    Point  { x: u16, y: u16 }, // CUP
    Column { x: u16 },         // CHA
    Row    { y: u16 },         // VPA
}


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

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

impl CSI for u8 {
    const MAX_DECIMALS: usize = 3; // u8::MAX = 255
    fn write_decimals(&self, out: &mut Vec<u8>) {
        _serialize_int(*self as usize, out)
    }
}

impl CSI for u16 {
    const MAX_DECIMALS: usize = 5; // u16::MAX = 65535
    fn write_decimals(&self, out: &mut Vec<u8>) {
        _serialize_int(*self as usize, out)
    }
}

impl CSI for u32 {
    const MAX_DECIMALS: usize = 10; // u32::MAX = 4294967295
    fn write_decimals(&self, out: &mut Vec<u8>) {
        _serialize_int(*self as usize, out)
    }
}

impl CSI for u64 {
    const MAX_DECIMALS: usize = 20; // u64::MAX = 18446744073709551615
    fn write_decimals(&self, out: &mut Vec<u8>) {
        _serialize_int(*self as usize, out)
    }
}

impl CSI for usize {
    const MAX_DECIMALS: usize = const {
        if cfg!(target_pointer_width = "64") { 20 } else { 10 }
    };

    fn write_decimals(&self, out: &mut Vec<u8>) {
        _serialize_int(*self as usize, out)
    }
}
