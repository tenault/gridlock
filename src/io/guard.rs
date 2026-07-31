use std::os::unix::io::RawFd;

use super::tty::{self, TerminalError, TerminalSnapshot};
use super::termios;

// ┌─────────────────────┐
// │ Terminal RAII Guard │
// └─────────────────────┘

/// Owns the terminal snapshot and guarantees restoration on drop.
///
/// Construct with [`TerminalGuard::acquire`]. While the guard is alive, you can call
/// [`TerminalGuard::snapshot`] to inspect the original state, or [`TerminalGuard::tty_fd`] to get
/// the file descriptor for subsequent `tcsetattr` calls.
pub struct TerminalGuard {
    snapshot: Option<TerminalSnapshot>,
}

impl TerminalGuard {
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

    /// Borrows the snapshot, panics if guard was already released.
    pub fn snapshot(&self) -> &TerminalSnapshot {
        self.snapshot
            .as_ref()
            .expect("TerminalGuard::snapshot called after release")
    }

    /// The file descriptor to use for all subsequent `tcgetattr`/`tcsetattr` calls.
    pub fn tty_fd(&self) -> RawFd { self.snapshot().tty_fd }

    /// Explicitly restores the terminal and releases the guard early.
    pub fn release(mut self) -> Result<(), TerminalError> { self.restore() }

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
