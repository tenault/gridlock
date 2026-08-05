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

use std::io::{self, Write};
use std::os::unix::io::RawFd;

// ┌─────────────┐ ┌╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴┐
// │    TYPES    │    // terminal snapshot
// └─────────────┘ └╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶┘

/// An immutable snapshot of the terminal's state at acquistion time.
#[derive(Debug, Clone)]
pub struct TerminalSnapshot {
    pub termios: libc::termios,
    pub ws_rows: u16,
    pub ws_cols: u16,
    pub tty_fd: RawFd,
}

impl TerminalSnapshot {
    /// Pretty-prints the captured termios flags to a writer (for logging, etc)
    pub fn dump_termios<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let t = &self.termios;
        writeln!(w, "----[ terminal snapshot ]----------------")?;
        writeln!(w, " -> tty_fd  : {}", self.tty_fd)?;
        writeln!(w, " -> winsize : {} cols x {} rows", self.ws_cols, self.ws_rows)?;
        writeln!(w, " -> c_iflag : 0x{:08x}", t.c_iflag)?;
        writeln!(w, " -> c_oflag : 0x{:08x}", t.c_oflag)?;
        writeln!(w, " -> c_cflag : 0x{:08x}", t.c_cflag)?;
        writeln!(w, " -> c_lflag : 0x{:08x}", t.c_lflag)?;

        // c_cc is [u8; N] where N varies by platform
        // we only want the standard POSIX control codes, so we drop index > 10...
        write!(w, " -> c_cc    : [")?;
        for (i, &cc) in t.c_cc.iter().take(11).enumerate() {
            if i > 0 { write!(w, ", ")?; }
            write!(w, "0x{:02x}", cc)?;
        }
        writeln!(w, ", ...]")?; // ...but we should still signal that more codes exist.

        Ok(())
    }
}


// ┌───────────────┐
// │    UTILITY    │
// └───────────────┘

/// Resolves the controlling tty fd via `/dev/tty`.
pub(crate) fn open_tty() -> Result<RawFd, TerminalError> {
    // O_NOCTTY prevents inadvertantly making /dev/tty our session's controlling terminal
    // (relevant if we're a session leader, e.g. after setsid).
    let fd = unsafe {
        libc::open(b"/dev/tty\0".as_ptr() as *const _, libc::O_RDWR | libc::O_NOCTTY)
    };

    if fd < 0 { return Err(TerminalError::BadOpenTTY(io::Error::last_os_error())); }

    Ok(fd)
}

/// Returns `(ws_row, ws_col)` via `ioctl(TIOCGWINSZ)`.
pub(crate) fn query_winsize(fd: RawFd) -> Result<(u16, u16), TerminalError> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    if rc != 0 { return Err(TerminalError::BadWinSize(io::Error::last_os_error())); }

    Ok((ws.ws_row, ws.ws_col))
}


// ┌──────────────┐ ┌╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴┐
// │    ERRORS    │    // terminal error
// └──────────────┘ └╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶┘

/// Errors that can arise during terminal state acquisition.
#[derive(Debug)]
pub enum TerminalError {
    /// `/dev/tty` could not be opened (e.g. no controlling terminal).
    BadOpenTTY(io::Error),
    /// `ioctl(TIOCGWINSZ)` failed.
    BadWinSize(io::Error),
    /// `tcgetattr` failed on the controlling tty.
    BadGetAttr(io::Error),
    /// `tcsetattr` failed on the controlling tty.
    BadSetAttr(io::Error),
    /// Signal handler failed during install.
    BadInstallHandler { signal: libc::c_int, source: io::Error },
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadOpenTTY(e) => write!(f, "Failed to open /dev/tty: {e}"),
            Self::BadWinSize(e) => write!(f, "ioctl(TIOCGWINSZ) failed: {e}"),
            Self::BadGetAttr(e) => write!(f, "tcgetattr failed: {e}"),
            Self::BadSetAttr(e) => write!(f, "tcsetattr failed: {e}"),
            Self::BadInstallHandler { signal, source } => {
                let name = match *signal {
                    libc::SIGINT => "SIGINT",
                    libc::SIGTERM => "SIGTERM",
                    _ => "unknown",
                };
                write!(f, "Failed to install handler for {name}: {source}")
            },
        }
    }
}

impl std::error::Error for TerminalError {}
