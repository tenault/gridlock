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

use super::error::TerminalError;


// ╭───────────╮
// │    TTY    │
// ╰───────────╯

/// Interface for the controlling tty.
pub(crate) struct TTY { fd: Option<RawFd> }

impl TTY {

    // ───── constructor ─────

    /// Resolves the controlling terminal file descriptor via `/dev/tty`.
    pub(crate) fn open() -> Result<Self, TerminalError> {
        let fd = unsafe {
            // O_NOCTTY prevents inadvertently making /dev/tty our session's controlling terminal
            // (relevant if we're a session leader, e.g. after setsid).
            libc::open(b"/dev/tty\0".as_ptr() as *const _, libc::O_RDWR | libc::O_NOCTTY)
        };

        if fd < 0 { return Err(TerminalError::OpenTTY { source: io::Error::last_os_error() }); }

        Ok(Self { fd: Some(fd) })
    }

    // ───── cleanup ─────

    /// Closes the controlling terminal file descriptor.
    pub(crate) fn close(&mut self) -> Result<(), TerminalError> {
        if let Some(fd) = self.fd.take() {
            if unsafe { libc::close(fd) } != 0 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EBADF) { return Ok(()); } // duplicate close ok
                return Err(TerminalError::CloseTTY {
                    fd,
                    source: err,
                });
            }
        }

        Ok(()) // no fd to close, we're chillin
    }

    // ───── terminal i/o ─────

    /// Writes all bytes to the tty fd, with `EINTR` handling and partial-write recovery.
    ///
    /// Loops until all bytes are written, or an error occurs (excluding `EINTR`). Returns the
    /// number of bytes written.
    pub(crate) fn write_raw(&self, buf: &[u8]) -> Result<usize, TerminalError> {
        let fd = self.fd()?;

        let mut index = 0;
        let total = buf.len();

        while index < total {
            let count = unsafe {
                libc::write(fd, buf[index..].as_ptr() as *const _, total - index)
            };

            if count < 0 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EINTR) { continue; } // retry on interrupts
                return Err(TerminalError::Write {
                    fd,
                    written: index,
                    total,
                    source: err,
                });
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
        let fd = self.fd()?;

        let count = unsafe {
            libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len())
        };

        if count < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) { return self.read_raw(buf); }
            return Err(TerminalError::Read {
                fd,
                read: 0,
                source: err,
            });
        }

        Ok(count as usize)
    }

    // ───── termios ─────

    /// Gets termios via `tcgetattr()`.
    pub(crate) fn get_termios(&self) -> Result<libc::termios, TerminalError> {
        let fd = self.fd()?;
        let mut t: libc::termios = unsafe { std::mem::zeroed() };

        if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
            return Err(TerminalError::GetTermios {
                fd,
                source: io::Error::last_os_error(),
            });
        }

        Ok(t)
    }

    /// Sets termios via `tcsetattr()`.
    pub(crate) fn set_termios(&self, t: &libc::termios) -> Result<(), TerminalError> {
        let fd = self.fd()?;

        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, t) } != 0 {
            return Err(TerminalError::SetTermios {
                fd,
                source: io::Error::last_os_error(),
            });
        }

        Ok(())
    }

    /// Ingests and aggressively reduces current termios to enter a known raw state.
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

    /// Returns the controlling terminal file descriptor, or errors if closed.
    pub(crate) fn fd(&self) -> Result<RawFd, TerminalError> {
        self.fd.ok_or(TerminalError::InvalidFd)    
    }

    /// Returns `(ws_row, ws_col)` via `ioctl(TIOCGWINSZ)`.
    pub(crate) fn query_winsize(&self) -> Result<(u16, u16), TerminalError> {
        let fd = self.fd()?;
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };

        if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) } != 0 {
            return Err(TerminalError::WinSize {
                fd,
                source: io::Error::last_os_error(),
            });
        }

        Ok((ws.ws_row, ws.ws_col))
    }
}

impl Drop for TTY {
    fn drop(&mut self) { let _ = self.close(); } // swallow errors, we just wanna close
}


// ╭─────────────────────╮
// │    SUPPORT TYPES    │
// ╰─────────────────────╯

/// An immutable snapshot of the terminal's state at acquistion time.
#[derive(Debug, Clone)]
pub(crate) struct TerminalSnapshot {
    pub(crate) fd: RawFd,
    pub(crate) termios: libc::termios,
    pub(crate) ws_rows: u16,
    pub(crate) ws_cols: u16,
}

impl TerminalSnapshot {
    /// Pretty-prints the saved termios to a writer (for logging, etc)
    pub(crate) fn dump_termios<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let t = &self.termios;
        writeln!(w, "----[ terminal snapshot ]----------------")?;
        writeln!(w, " -> tty_fd  : {}", self.fd)?;
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
