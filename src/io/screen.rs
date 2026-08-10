// ╭──────────────────────────────────────────────────────────────io/screen.rs─╮
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

use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

use super::error::TerminalError;


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

pub(crate) const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
pub(crate) const EXIT_ALT_SCREEN:  &[u8] = b"\x1b[?1049l";

/// Idempotency flag to protect entry/exits of the alternate screen buffer.
static ALT_SCREEN_ACTIVE: AtomicBool = AtomicBool::new(false);


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Enters the alternate screen buffer via `libc::write` (idempotent).
pub(crate) fn enter_alt_screen(fd: RawFd) -> Result<(), TerminalError> {
    if ALT_SCREEN_ACTIVE.load(Ordering::Acquire) { return Ok(()); }

    let count = unsafe {
        libc::write(fd, ENTER_ALT_SCREEN.as_ptr() as *const _, ENTER_ALT_SCREEN.len())
    };

    if count < 0 { return Err(TerminalError::EnterAlternateScreen); }

    ALT_SCREEN_ACTIVE.store(true, Ordering::Release);
    Ok(())
}

/// Exits the alternate screen buffer via `libc::write` (idempotent).
pub(crate) fn exit_alt_screen(fd: RawFd) -> Result<(), TerminalError> {
    if !ALT_SCREEN_ACTIVE.load(Ordering::Acquire) { return Ok(()); }

    let count = unsafe {
        libc::write(fd, EXIT_ALT_SCREEN.as_ptr() as *const _, EXIT_ALT_SCREEN.len())
    };

    if count < 0 { return Err(TerminalError::ExitAlternateScreen); }

    ALT_SCREEN_ACTIVE.store(false, Ordering::Release);
    Ok(())
}
