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

use gridlock::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = Terminal::acquire().expect("Failed to acquire terminal.");
    let mut xsr = XORShiftRNG::new();
    let mut buf = [0u8; 16];

    loop {
        match term.read(&mut buf) {
            Ok(n) if n > 0 => {
                let x = rand(&(term.cols as usize), &mut xsr);
                let y = rand(&(term.rows as usize), &mut xsr);

                term.move_cursor_to(x as u16, y as u16)?;

                for &b in &buf[..n] {
                    term.write(&format!("{:02x}", b));
                }
            }
            _ => {}
        }
    }
}

// mini-RNG (because rust is too prideful to include it in std::)
struct XORShiftRNG { state: u64 }
impl XORShiftRNG {
    pub fn new() -> Self {
        Self {
            state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        }
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

pub fn rand(max: &usize, xsr: &mut XORShiftRNG) -> usize {
    let cap = usize::MAX - (usize::MAX % *max as usize);

    loop { // it'll find a number... eventually...
        let num = xsr.next() as usize;
        if num <= cap { return num % *max }
    }
}
