// ┌─────────────────────────────────────────────────────────────────────────────────┐
// │                                                                                 │
// │    ┏━━━━━━━┓ ┏━━━━━━━┓ ┏━┓ ┏━━━━━━━┓ ┏━┓       ┏━━━━━━━┓ ┏━━━━━━━┓ ┏━┓ ┏━━━┓    │
// │    ┃ ┏━━━━━┛ ┃ ┏━━━┓ ┃ ┃ ┃ ┗━┓ ┏━┓ ┃ ┃ ┃       ┃ ┏━━━┓ ┃ ┃ ┏━━━━━┛ ┃ ┃ ┃ ┏━┛    │
// │    ┃ ┃ ┏━━━┓ ┃ ┗━━━┛ ┃ ┃ ┃   ┃ ┃ ┃ ┃ ┃ ┃       ┃ ┃   ┃ ┃ ┃ ┃       ┃ ┗━┛ ┗━┓    │
// │    ┃ ┃ ┗━┓ ┃ ┃ ┏━┓ ┏━┛ ┃ ┃   ┃ ┃ ┃ ┃ ┃ ┃       ┃ ┃   ┃ ┃ ┃ ┃       ┃ ┏━━━┓ ┃    │
// │    ┃ ┗━━━┛ ┃ ┃ ┃ ┃ ┗━┓ ┃ ┃ ┏━┛ ┗━┛ ┃ ┃ ┗━━━━━┓ ┃ ┗━━━┛ ┃ ┃ ┗━━━━━┓ ┃ ┃   ┃ ┃    │
// │    ┗━━━━━━━┛ ┗━┛ ┗━━━┛ ┗━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━━━━━━━┛ ┗━┛   ┗━┛    │
// │                                                                                 │
// │                   copyright (c) 2026 Malakai Smith (@tenault)                   │
// │                                                                                 │
// │       This Source Code Form is subject to the terms of the Mozilla Public       │
// │       License, v. 2.0. If a copy of the MPL was not distributed with this       │
// │            file, You can obtain one at https://mozilla.org/MPL/2.0.             │
// │                                                                                 │
// └─────────────────────────────────────────────────────────────────────────────────┘

use std::os::unix::io::RawFd;

use super::tty::{self, TerminalError, TerminalSnapshot};
use super::termios;

// ┌─────────────┐ ┌╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴┐
// │    TYPES    │    // terminal RAII guard
// └─────────────┘ └╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶┘

/// Owns the terminal snapshot and guarantees state restoration on drop.
///
/// Construct with [`TerminalGuard::acquire`]. While the guard is alive, you can call
/// [`TerminalGuard::snapshot`] to inspect the original state, or [`TerminalGuard::tty_fd`] to get
/// the file descriptor for subsequent `tcsetattr` calls.
pub struct TerminalGuard {
    snapshot: Option<TerminalSnapshot>,
}

impl TerminalGuard {

    // ───── constructor ─────

    /// Acquires the current terminal state.
    ///
    /// Opens `/dev/tty` instead of assuming `fd 0` is a terminal, so this succeeds even when
    /// `stdin` is redirected.
    pub fn acquire() -> Result<Self, TerminalError> {
        let tty_fd = tty::open_tty()?;

        let orig_termios = match termios::get_termios(tty_fd) {
            Ok(t) => t,
            Err(e) => {
                unsafe { libc::close(tty_fd); }
                return Err(e);
            }
        };

        let (rows, cols) = match tty::query_winsize(tty_fd) {
            Ok(size) => size,
            Err(e) => {
                unsafe { libc::close(tty_fd); }
                return Err(e);
            }
        };

        Ok(Self {
            snapshot: Some(TerminalSnapshot {
                orig_termios,
                ws_rows: rows,
                ws_cols: cols,
                tty_fd,
            }),
        })
    }

    // ───── utility ─────

    /// Borrows the snapshot, panics if the guard was already released.
    pub fn snapshot(&self) -> &TerminalSnapshot {
        self.snapshot
            .as_ref()
            .expect("guard.snapshot() called after release!")
    }

    /// Gets the controlling tty fd from the snapshot, useful for subsequent `tcsetattr` calls.
    pub fn tty_fd(&self) -> RawFd { self.snapshot().tty_fd }

    // ───── cleanup ─────

    /// Explicitly restores the terminal and releases the guard early.
    pub fn release(mut self) -> Result<(), TerminalError> { self.restore() }

    // ╶╶╶╶╶ internal ╴╴╴╴╴

    fn restore(&mut self) -> Result<(), TerminalError> {
        if let Some(snap) = self.snapshot.take() {
            let err = termios::set_termios(snap.tty_fd, &snap.orig_termios);
            unsafe { libc::close(snap.tty_fd); }
            return err;
        }

        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) { let _ = self.restore(); }
}
