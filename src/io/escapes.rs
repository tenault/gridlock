// ╭─────────────────────────────────────────────────────────────io/escapes.rs─╮
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

// ╭───────────────╮
// │    ESCAPES    │
// ╰───────────────╯

pub(crate) const ESC: u8 = 0x1b;

pub(crate) const CSI: [u8; 2] = [ESC, b'['];

pub(crate) const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
pub(crate) const EXIT_ALT_SCREEN:  &[u8] = b"\x1b[?1049l";


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

pub(crate) fn generate_csi(params: &[u16], cmd: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 + params.len() * 2 + params.len().saturating_sub(1));
    out.extend_from_slice(&CSI);

    for (i, &p) in params.iter().enumerate() {
        if i > 0 { out.push(b';'); }
        int_to_ascii(p, &mut out);
    }

    out.push(cmd);
    out
}

fn int_to_ascii(n: u16, out: &mut Vec<u8>) {
    if n == 0 {
        out.push(b'0');
        return;
    }

    let mut buf = [0u8; 5]; // u16::MAX = 65535
    let mut i = 4;
    let mut v = n;

    while v > 0 && i > 0 {
        buf[i] = (v % 10) as u8 + b'0';
        v /= 10;
        i -= 1;
    }

    out.extend_from_slice(&buf[i + 1..]);
}


// ╭──────────────╮
// │    MACROS    │
// ╰──────────────╯

#[macro_export]
macro_rules! csi {
    // cmd + no params
    (@ [] $cmd:literal) => { crate::io::escapes::generate_csi(&[], $cmd as u8) };

    // cmd + some params
    (@ [$($acc:expr),+] $cmd:literal) => {
        crate::io::escapes::generate_csi(&[$($acc as u16),+], $cmd as u8)
    };

    // param extractor
    (@ [$($acc:expr),*] $param:expr, $($rest:tt)*) => { csi!(@ [$($acc,)* $param] $($rest)*) };

    // entry -> param extractor
    ($($tt:tt)+) => { csi!(@ [] $($tt)+) };
}
