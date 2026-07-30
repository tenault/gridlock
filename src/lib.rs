use std::io::{self, Write};
use std::os::unix::io::RawFd;

// ┌─────────────────┐
// │ Terminal Errors │
// └─────────────────┘

/// Errors that can arise during terminal state acquisition.
#[derive(Debug)]
pub enum TerminalError {
    /// `/dev/tty` could not be opened (e.g. no controlling terminal).
    BadOpenTTY(io::Error),
    /// `tcgetattr` failed on the controlling TTY.
    BadGetAttr(io::Error),
    /// `tcsetattr` failed on the controlling TTY.
    BadSetAttr(io::Error),
    /// `ioctl(TIOCGWINSZ)` failed.
    BadWinSize(io::Error),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadOpenTTY(e) => write!(f, "failed to open /dev/tty: {e}"),
            Self::BadGetAttr(e) => write!(f, "tcgetattr failed: {e}"),
            Self::BadSetAttr(e) => write!(f, "tcsetattr failed: {e}"),
            Self::BadWinSize(e) => write!(f, "ioctl(TIOCGWINSZ) failed: {e}"),
        }
    }
}

impl std::error::Error for TerminalError {}


// ┌───────────────────┐
// │ Terminal Snapshot │
// └───────────────────┘

/// An immutable snapshot of the terminal's state at acquistion time.
#[derive(Debug, Clone)]
pub struct TerminalSnapshot {
    pub orig_termios: libc::termios,
    pub ws_rows: u16,
    pub ws_cols: u16,
    pub tty_fd: RawFd,
}

impl TerminalSnapshot {
    /// Pretty-prints the captured termios flags to a writer (for logging, etc)
    pub fn dump_termios<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let t = &self.orig_termios;
        writeln!(w, "---===[ TERMINAL SNAPSHOT ]===---")?;
        writeln!(w, "tty_fd  :: {}", self.tty_fd)?;
        writeln!(w, "winsize :: {} cols x {} rows", self.ws_cols, self.ws_rows)?;
        writeln!(w, "c_iflag :: 0x{:08x}", t.c_iflag)?;
        writeln!(w, "c_oflag :: 0x{:08x}", t.c_oflag)?;
        writeln!(w, "c_cflag :: 0x{:08x}", t.c_cflag)?;
        writeln!(w, "c_lflag :: 0x{:08x}", t.c_lflag)?;

        // c_cc is [u8; N] where N varies by platform (32 for linux, 20 for macos)
        // we only want the standard POSIX control codes, so we drop index > 10
        write!(w, "c_cc    :: [")?;
        for (i, &cc) in t.c_cc.iter().take(11).enumerate() {
            if i > 0 { write!(w, ", ")?; }
            write!(w, "0x{:02x}", cc)?;
        }
        writeln!(w, ", ...]")?; // we should still signal that more codes exist.

        Ok(())
    }
}

impl Drop for TerminalSnapshot {
    /// Attempts terminal restore on drop, in case the RAII guard itself was moved-out-of or
    /// [`std::mem::forget`]ted. In normal flow, this is a no-op.
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(self.tty_fd, libc::TCSANOW, &self.orig_termios); }
    }
}


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
        // resolve the controlling tty
        let tty_fd = {
            // O_NOCTTY to prevent inadvertantly making /dev/tty our session's controlling
            // terminal (relevant if we're a session leader, e.g. after setsid).
            let fd = unsafe {
                libc::open(
                    b"/dev/tty\0".as_ptr() as *const _,
                    libc::O_RDWR | libc::O_NOCTTY,
                )
            };

            if fd < 0 { return Err(TerminalError::BadOpenTTY(io::Error::last_os_error())); }

            fd
        };

        // save snapshot
        let orig_termios = {
            let mut t: libc::termios = unsafe { std::mem::zeroed() };
            let rc = unsafe { libc::tcgetattr(tty_fd, &mut t) };

            if rc != 0 {
                unsafe { libc::close(tty_fd); } // close the fd we just opened before erring
                return Err(TerminalError::BadGetAttr(io::Error::last_os_error()));
            }

            t
        };

        // capture winsize
        let (rows, cols) = {
            let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
            let rc = unsafe { libc::ioctl(tty_fd, libc::TIOCGWINSZ, &mut ws) };

            if rc != 0 {
                unsafe { libc::close(tty_fd); }
                return Err(TerminalError::BadWinSize(io::Error::last_os_error()));
            }

            (ws.ws_row, ws.ws_col)
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
            // restore termios first
            let rc = unsafe { libc::tcsetattr(snap.tty_fd, libc::TCSANOW, &snap.orig_termios) };
            let term_error = if rc != 0 { Some(TerminalError::BadSetAttr(io::Error::last_os_error())) } else { None };

            // close the tty fd
            unsafe { libc::close(snap.tty_fd); }

            // bubble up any errors
            if let Some(e) = term_error { return Err(e); }
        }

        Ok(())
    }
}

impl Drop for TerminalGuard { fn drop(&mut self) { let _ = self.restore(); } }
