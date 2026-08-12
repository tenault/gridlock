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

use std::time::{SystemTime, UNIX_EPOCH};

use gridlock::Terminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = Terminal::acquire().expect("Failed to acquire terminal.");
    let mut rng = LCG::from_time();
    let mut buf = [0u8; 16];

    loop {
        match term.read(&mut buf) {
            Ok(n) if n > 0 => {
                for &b in &buf[..n] {
                    let x = rng.next() % term.cols as u32;
                    let y = rng.next() & term.rows as u32;

                    term.move_cursor(x as u16, y as u16)?;
                    term.write(&format!("{:02x}", b));
                }
            }
            _ => {}
        }
    }
}

// mini-RNG (because rust is too prideful to include it in std::)
struct LCG(u32);
impl LCG {
    fn from_time() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u32)
            .unwrap_or(0xC0FFEE);

        LCG(if seed == 0 { 0xC0FFEE } else { seed })
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        self.0
    }
}
