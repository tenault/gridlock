```
                     ┏━┓    ┏━━┓              ┏━┓
                     ┃ ┃    ┗┓ ┃              ┃ ┃
┏━━━┓┏┓┏━━━━━┓┏━┓┏━━━┛ ┃     ┃ ┃┏━━━━━┓┏━━━━━┓┃ ┃┏━━┓
┃ ┏━┓ ┃┃ ┏━━━┛┃ ┃┃ ┏━┓ ┃     ┃ ┃┃ ┏━┓ ┃┃ ┏━━━┛┃ ┗┛┏━┛
┃ ┗━┛ ┃┃ ┃    ┃ ┃┃ ┗━┛ ┃ ┏━┓ ┃ ┃┃ ┗━┛ ┃┃ ┗━━━┓┃ ┏┓┗━┓
┗━━━┓ ┃┗━┛    ┗━┛┗━━━┛┗┛ ┗━┛ ┗━━┛┗━━━━┛┗━━━━━┛┗━┛┗━━┛
┏━━━┛ ┃ /////////////////////////////// v0.2 polymer
┗━━━━━┛
```
---

<div align="center">
    <p>A new kind of library for modern TUIs, built for terminals that matter.</p>
    <p><a href="https://github.com/tenault/gridlock/blob/stable/LICENSE.md"><img src="https://img.shields.io/badge/license-MPL--2.0-blue?style=for-the-badge" alt="license badge" /></a>&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://img.shields.io/badge/version-0.2.0-rebeccapurple?style=for-the-badge" alt="version badge" />&nbsp;&nbsp;&nbsp;&nbsp;<img src="https://img.shields.io/badge/installation-not_recommended-red?style=for-the-badge" alt="installation badge" /></p>
</div>

---

> [!WARNING]
> Gridlock is under heavy development, and is ___not___ suitable for real-world use. Feel free to follow along with the development, or check out [evolution](https://github.com/tenault/gridlock/tree/evolution) to peek at the current state of things. Use of this library is <ins>strongly discouraged</ins> until this disclaimer disappears.

## Overview

Currently, `gridlock` provides a safe abstraction for terminal I/O on Unix systems, including:
- State acquisition and restoration (termios, alternate screen)
- Cursor movement
- Text styling (attributes + colors)
- Signal handling for (mostly) graceful interruptions
- 256-color and 24-bit truecolor support

## Quick Start
Add to `Cargo.toml`:
```toml
[dependencies]
gridlock = { git = "https://github.com/tenault/gridlock" }
```

Then, in `main.rs`:
```rust,no_run
use gridlock::{Terminal, TerminalStyle, Color};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut term = Terminal::acquire()?;

    // Move cursor and set style
    term.move_cursor_to(10, 5)?;

    let style = TerminalStyle::new()
        .bold()
        .fg(Color::green())
        .build();

    term.apply_style(&style)?;
    term.write("Hello gridlock!")?;

    // Terminal restored on Drop
    Ok(())
}
```
