// ╭─────────────────────────────────────────────────────────────io/termios.rs─╮
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

use std::io;
use std::os::unix::io::RawFd;

use super::tty::TerminalError;


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Gets the terminal state via `tcgetattr`.
///
/// Does __not__ close the tty fd on error. Caller is responsible for cleanup.
pub(crate) fn get_termios(fd: RawFd) -> Result<libc::termios, TerminalError> {
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::tcgetattr(fd, &mut t) };
    if rc != 0 { return Err(TerminalError::BadGetAttr(io::Error::last_os_error())); }

    Ok(t)
}

/// Sets the termios via `tcsetattr`.
///
/// Does __not__ close the tty fd on error. Caller is responsible for cleanup.
pub(crate) fn set_termios(fd: RawFd, t: &libc::termios) -> Result<(), TerminalError> {
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, t) } != 0 {
        return Err(TerminalError::BadSetAttr(io::Error::last_os_error()));
    }

    Ok(())
}
