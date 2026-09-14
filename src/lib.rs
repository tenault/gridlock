// ╭────────────────────────────────────────────────────────────────────lib.rs─╮
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

//! # Gridlock
//!
//! A new kind of terminal library for modern TUIs.
//!
//! ## Overview
//!
//! Currently, `gridlock` provides a safe abstraction for terminal I/O on Unix systems, including:
//! - State acquisition and restoration (termios, alternate screen)
//! - Cursor movement
//! - Text styling (attributes + colors)
//! - Signal handling for mostly graceful interruptions
//! - 256-color and 24-bit truecolor support
//!
//! ## Design Philosophy
//!
//! - __Safety__: The terminal is restored on every exit path, even on panics (leaves no trace)
//! - __Performance__: Syscalls, escape sequences, and writes are minimized where possible
//! - __Containment__: Avoids cruft and dependency bloat, preferring in-house code
//! - __Unix-first__: Built for the future, refusing support for consumer-hostile operating systems
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gridlock::{Terminal, TerminalStyle, Color};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut term = Terminal::acquire()?;
//!
//!     // Move cursor and set style
//!     term.move_cursor_to(10, 5)?;
//!
//!     let style = TerminalStyle::new()
//!         .bold()
//!         .fg(Color::green())
//!         .build();
//!
//!     term.apply_style(&style)?;
//!     term.write("Hello gridlock!")?;
//!
//!     // Terminal restored on Drop
//!     Ok(())
//! }
//! ```
//!
//! ## Safety Considerations
//!
//! - Terminal restored on every exit path (normal, panic, or signal)
//! - Signal handlers are `async-signal-safe` and can fire at any point
//! - Multiple `Terminal::acquire()` calls fail with [`TerminalError::IllegalGuard`]
//! - The `Terminal` struct is `!Send` and `!Sync` by design (owning the tty file descriptor)
//!
//! ## Architecture
//!
//! ```text
//! lib.rs
//! ├── cell/
//! │   ├── mod.rs      // Module exports
//! │   ├── color.rs    // Color jazz (ANSI, 256, RGB)
//! │   ├── cursor.rs   // Virtual cursor tracking
//! │   └── style.rs    // Text styling (SGR)
//! ├── core/
//! │   ├── mod.rs      // Module exports
//! │   ├── error.rs    // Error types
//! │   └── terminal.rs // Main terminal interface
//! ├── io/
//! │   ├── mod.rs      // Module exports
//! │   ├── escape.rs   // ANSI escape generation
//! │   ├── signal.rs   // Signal handlers
//! │   └── tty.rs      // Low-level TTY operations
//! └── unicode/
//!     ├── mod.rs      // Module exports
//!     ├── grapheme.rs // Grapheme Cluster segmentation (UAX #29)
//!     ├── sentence.rs // Word boundary detection       (UAX #29)
//!     └── word.rs     // Sentence boundary detection   (UAX #29)
//! ```
//!
//! ## Platforms
//!
//! Supports all unix-like systems (Linux, macOS, BSD) via `/dev/tty` and POSIX interfaces. There is
//! no planned support for Windows at this time.
//!
//! ## License
//!
//! Licensed under the Mozilla Public License v2.0. See LICENSE.md or <https://mozilla.org/MPL/2.0/>
//! for details.
//!
//! [`TerminalError::IllegalGuard`]: crate::TerminalError::IllegalGuard

// ╭───────────────────╮
// │    ENVIRONMENT    │
// ╰───────────────────╯

pub mod cell;
pub mod core;
pub mod io;
pub mod unicode;

pub use cell::{Color, TerminalStyle};
pub use core::{Terminal, TerminalError};
