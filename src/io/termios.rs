// ╭───────────────────────────────────────────────────────────────────io/termios.rs─╮
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
// ╰─────────────────────────────────────────────────────────────────────────────────╯

use std::io;
use std::os::unix::io::RawFd;

use super::tty::TerminalError;


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Ingests and aggressively reduces given termios to bring the terminal to a known raw state.
///
/// This results in `c_iflag`, `c_oflag`, `c_lflag`, `c_cc[VMIN]` and `c_cc[VTIME]` being zeroed,
/// ensuring consistent behavior regardless of any prior flags set (by the environment or other
/// programs).
pub(crate) fn uncook(fd: RawFd, term: &libc::termios) -> Result<(), TerminalError> {
    let mut t = *term;

    t.c_iflag = 0;
    t.c_lflag = 0;
    t.c_oflag = 0;

    t.c_cc[libc::VMIN] = 0;
    t.c_cc[libc::VTIME] = 0;

    set_termios(fd, &t)
}

/// Gets termios via `tcgetattr`.
///
/// Does __not__ close the tty fd on error. Caller is responsible for cleanup.
pub(crate) fn get_termios(fd: RawFd) -> Result<libc::termios, TerminalError> {
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
        return Err(TerminalError::BadGetAttr(io::Error::last_os_error()));
    }

    Ok(t)
}

/// Sets termios via `tcsetattr`.
///
/// Does __not__ close the tty fd on error. Caller is responsible for cleanup.
pub(crate) fn set_termios(fd: RawFd, t: &libc::termios) -> Result<(), TerminalError> {
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, t) } != 0 {
        return Err(TerminalError::BadSetAttr(io::Error::last_os_error()));
    }

    Ok(())
}
