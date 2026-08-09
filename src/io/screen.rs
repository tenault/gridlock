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


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

const ENTER_ASB: &[u8] = b"\x1b[?1049h";
const EXIT_ASB:  &[u8] = b"\x1b[?1049l";

/// Idempotency flag to protect entry/exits of the alternate screen buffer.
static ASB_ACTIVE: AtomicBool = AtomicBool::new(false);


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Enters the alternate screen buffer via `libc::write` (idempotent).
pub(crate) fn enter_asb(fd: RawFd) {
    if ASB_ACTIVE.swap(true, Ordering::AcqRel) { return; }
    unsafe { libc::write(fd, ENTER_ASB.as_ptr() as *const _, ENTER_ASB.len()); }
}

/// Exits the alternate screen buffer via `libc::write` (idempotent).
pub(crate) fn exit_asb(fd: RawFd) {
    if !ASB_ACTIVE.swap(false, Ordering::AcqRel) { return; }
    unsafe { libc::write(fd, EXIT_ASB.as_ptr() as *const _, EXIT_ASB.len()); }
}
