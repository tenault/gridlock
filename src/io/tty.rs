// ╭─────────────────────────────────────────────────────────────────io/tty.rs─╮
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

use std::io::{self, Write};
use std::os::unix::io::RawFd;


// ╭───────────╮
// │    TTY    │
// ╰───────────╯

pub(crate) struct TTY { pub(crate) fd: RawFd }

impl TTY {

    // ───── constructor ─────

    /// Resolves the controlling tty fd via `/dev/tty`.
    pub(crate) fn open() -> Result<Self, TerminalError> {
        let fd = unsafe {
            // O_NOCTTY prevents inadvertantly making /dev/tty our session's controlling terminal
            // (relevant if we're a session leader, e.g. after setsid).
            libc::open(b"/dev/tty\0".as_ptr() as *const _, libc::O_RDWR | libc::O_NOCTTY)
        };

        if fd < 0 { return Err(TerminalError::BadOpenTTY(io::Error::last_os_error())); }

        Ok(Self { fd })
    }

    // ───── read/write ─────

    /// Writes all bytes to the tty fd, with `EINTR` handling and partial-write recovery.
    ///
    /// Loops until all bytes are written, or an error occurs (excluding `EINTR`). Returns the
    /// number of bytes written.
    pub(crate) fn write_raw(&self, buf: &[u8]) -> Result<usize, TerminalError> {
        let mut index = 0;
        while index < buf.len() {
            let count = unsafe {
                libc::write(self.fd, buf[index..].as_ptr() as *const _, buf.len() - index)
            };

            if count < 0 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EINTR) { continue; } // retry on interrupts
                return Err(TerminalError::BadWrite(err));
            }

            if count == 0 { break; } // EOF in stdout is rare, but technically not an error

            index += count as usize;
        }

        Ok(index)
    }

    /// Reads available bytes from the tty fd, non-blocking.
    ///
    /// Because the guard configures with `VMIN=0` and `VTIME=0`, this returns immediately with
    /// `0-N` bytes read (`0` means no input pending, not `EOF`).
    pub(crate) fn read_raw(&self, buf: &mut [u8]) -> Result<usize, TerminalError> {
        let count = unsafe {
            libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len())
        };

        if count < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) { return self.read_raw(buf); }
            return Err(TerminalError::BadRead(err));
        }

        Ok(count as usize)
    }

    // ───── termios ─────

    /// Gets termios via `tcgetattr`.
    pub(crate) fn get_termios(&self) -> Result<libc::termios, TerminalError> {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(self.fd, &mut t) } != 0 {
            return Err(TerminalError::BadGetAttr(io::Error::last_os_error()));
        }

        Ok(t)
    }

    /// Sets termios via `tcsetattr`.
    pub(crate) fn set_termios(&self, t: &libc::termios) -> Result<(), TerminalError> {
        if unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, t) } != 0 {
            return Err(TerminalError::BadSetAttr(io::Error::last_os_error()));
        }

        Ok(())
    }

    /// Ingests and aggressively reduces current termios to enter a known raw mode.
    ///
    /// This results in `c_iflag`, `c_oflag`, `c_lflag`, `c_cc[VMIN]` and `c_cc[VTIME]` being
    /// zeroed, ensuring consistent behavior regardless of any prior flags set (by the environment
    /// or other programs).
    pub(crate) fn uncook(&self) -> Result<(), TerminalError> {
        let mut t = self.get_termios()?;

        t.c_iflag = 0;
        t.c_lflag = 0;
        t.c_oflag = 0;

        t.c_cc[libc::VMIN] = 0;
        t.c_cc[libc::VTIME] = 0;

        self.set_termios(&t)
    }

    // ───── utility ─────

    /// Returns `(ws_row, ws_col)` via `ioctl(TIOCGWINSZ)`.
    pub(crate) fn query_winsize(&self) -> Result<(u16, u16), TerminalError> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(self.fd, libc::TIOCGWINSZ, &mut ws) } != 0 {
            return Err(TerminalError::BadWinSize(io::Error::last_os_error()));
        }

        Ok((ws.ws_row, ws.ws_col))
    }
}

impl Drop for TTY {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { libc::close(self.fd); }
        }
    }
}


// ╭─────────────────────────╮
// │    TERMINAL SNAPSHOT    │
// ╰─────────────────────────╯

/// An immutable snapshot of the terminal's state at acquistion time.
#[derive(Debug, Clone)]
pub(crate) struct TerminalSnapshot {
    pub termios: libc::termios,
    pub ws_rows: u16,
    pub ws_cols: u16,
    pub tty_fd: RawFd,
}

impl TerminalSnapshot {
    /// Pretty-prints the captured termios flags to a writer (for logging, etc)
    pub(crate) fn dump_termios<W: Write>(&self, w: &mut W) -> io::Result<()> {
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


// ╭──────────────╮
// │    ERRORS    │
// ╰──────────────╯

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
    /// `read_raw` failed.
    BadRead(io::Error),
    /// `write_raw` failed.
    BadWrite(io::Error),
    /// Signal handler failed during install.
    BadInstallHandler { signal: libc::c_int, source: io::Error },
    /// `::acquire()` failed due to existing guard.
    ExistingGuard,
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadOpenTTY(e) => write!(f, "Failed to open /dev/tty: {e}"),
            Self::BadWinSize(e) => write!(f, "ioctl(TIOCGWINSZ) failed: {e}"),
            Self::BadGetAttr(e) => write!(f, "tcgetattr failed: {e}"),
            Self::BadSetAttr(e) => write!(f, "tcsetattr failed: {e}"),
            Self::BadRead(e)    => write!(f, "raw read failed: {e}"),
            Self::BadWrite(e)   => write!(f, "raw write failed: {e}"),
            Self::BadInstallHandler { signal, source } => {
                let name = match *signal {
                    libc::SIGINT => "SIGINT",
                    libc::SIGTERM => "SIGTERM",
                    _ => "unknown",
                };
                write!(f, "Failed to install handler for {name}: {source}")
            },
            Self::ExistingGuard => write!(f, "acquire() failed due to existing guard."),
        }
    }
}

impl std::error::Error for TerminalError {}
