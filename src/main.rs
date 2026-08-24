// ╭───────────────────────────────────────────────────────────────────main.rs─╮
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

use gridlock::{Color, Terminal, TerminalStyle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = Terminal::acquire().expect("Failed to acquire terminal.");
    let mut xsr = XORShiftRNG::new();
    let mut buf = [0u8; 16];

    loop {
        match term.read(&mut buf) {
            Ok(n) if n > 0 => {
                if buf[0] == b'q' || buf[0] == b'Q' { break; }

                let x = rand(&(term.cols as usize), &mut xsr);
                let y = rand(&(term.rows as usize), &mut xsr);

                let mut style = TerminalStyle::new();

                if xsr.next_normal() < 0.3 { style = style.bold(); }
                if xsr.next_normal() < 0.3 { style = style.dim(); }
                if xsr.next_normal() < 0.3 { style = style.italic(); }
                if xsr.next_normal() < 0.3 { style = style.underline(); }
                if xsr.next_normal() < 0.3 { style = style.blink(); }
                if xsr.next_normal() < 0.3 { style = style.reverse(); }
                if xsr.next_normal() < 0.3 { style = style.strikethrough(); }

                let pick_fg = xsr.next_normal();
                let fg = if pick_fg < 0.3 {
                    Color::default()
                } else if pick_fg < 0.6 {
                    Color::indexed(rand(&256, &mut xsr) as u8)
                } else {
                    Color::rgb(
                        rand(&256, &mut xsr) as u8,
                        rand(&256, &mut xsr) as u8,
                        rand(&256, &mut xsr) as u8,
                    )
                };

                let pick_bg = xsr.next_normal();
                let bg = if pick_bg < 0.3 {
                    Color::default()
                } else if pick_bg < 0.6 {
                    Color::indexed(rand(&256, &mut xsr) as u8)
                } else {
                    Color::rgb(
                        rand(&256, &mut xsr) as u8,
                        rand(&256, &mut xsr) as u8,
                        rand(&256, &mut xsr) as u8,
                    )
                };

                style = style.fg(fg).bg(bg);

                term.move_cursor_to(x as u16, y as u16)?;
                term.apply_style(&style.build())?;
                term.write("hello, gridlock")?;
            },
            _ => {}
        }
    }

    Ok(())
}

// mini-RNG (because rust is too prideful to include it in std::)
struct XORShiftRNG { state: u64 }
impl XORShiftRNG {
    fn new() -> Self {
        Self {
            state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_normal(&mut self) -> f64 { ((self.next() >> 11) as f64) / ((1u64 << 53) as f64) }
}

fn rand(max: &usize, xsr: &mut XORShiftRNG) -> usize {
    let cap = usize::MAX - (usize::MAX % *max as usize);

    loop { // it'll find a number... eventually...
        let num = xsr.next() as usize;
        if num <= cap { return num % *max }
    }
}
