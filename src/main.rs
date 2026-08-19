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
                let x = rand(&(term.cols as usize), &mut xsr);
                let y = rand(&(term.rows as usize), &mut xsr);

                term.move_cursor_to(x as u16, y as u16)?;

                match buf[0] {
                    b'1' => {
                        let style = TerminalStyle::new()
                            .bold()
                            .fg(Color::red())
                            .build();

                        term.apply_style(&style)?;
                        term.write("bold red")?;
                    },
                    b'2' => {
                        let style = TerminalStyle::new()
                            .dim()
                            .fg(Color::green())
                            .build();

                        term.apply_style(&style)?;
                        term.write("dim green")?;
                    },
                    b'3' => {
                        let style = TerminalStyle::new()
                            .italic()
                            .fg(Color::yellow())
                            .build();

                        term.apply_style(&style)?;
                        term.write("italic yellow")?;
                    },
                    b'4' => {
                        let style = TerminalStyle::new()
                            .underline()
                            .fg(Color::blue())
                            .build();

                        term.apply_style(&style)?;
                        term.write("underlined blue")?;
                    },
                    b'5' => {
                        let style = TerminalStyle::new()
                            .blink()
                            .fg(Color::magenta())
                            .build();

                        term.apply_style(&style)?;
                        term.write("blinking magenta")?;
                    },
                    b'6' => {
                        let style = TerminalStyle::new()
                            .fast_blink()
                            .fg(Color::cyan())
                            .build();

                        term.apply_style(&style)?;
                        term.write("fast blinking cyan")?;
                    },
                    b'7' => {
                        let style = TerminalStyle::new()
                            .fg(Color::bright_red())
                            .reverse()
                            .build();

                        term.apply_style(&style)?;
                        term.write("reversed red")?;
                    },
                    b'8' => {
                        let style = TerminalStyle::new()
                            .concealed()
                            .build();

                        term.apply_style(&style);
                        term.write("concealed")?;
                    },
                    b'9' => {
                        let style = TerminalStyle::new()
                            .strikethrough()
                            .fg(Color::bright_green())
                            .build();

                        term.apply_style(&style);
                        term.write("struckthrough green")?;
                    }
                    b'0' | b'r' => {
                        let style = TerminalStyle::new().build();
                        term.write("RESET")?;
                    }
                    b'q' => break,
                    _ => {},
                }
            }
            _ => {}
        }
    }

    Ok(())
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
