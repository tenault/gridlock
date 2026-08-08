// ╭──────────────────────────────────────────────────────────────io/signal.rs─╮
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
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use super::guard;
use super::tty::{TerminalError, TerminalSnapshot};


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

const HANDLED_SIGNALS: [libc::c_int; 3] = [
    libc::SIGABRT,
    libc::SIGINT,
    libc::SIGTERM,
];

/// Raw pointer to the active snapshot, null when no guard is active.
static SNAPSHOT_PTR: AtomicPtr<TerminalSnapshot> = AtomicPtr::new(ptr::null_mut());

/// Signal-safe sentinel for whether handlers are installed.
static HANDLERS_INSTALLED: AtomicBool = AtomicBool::new(false);


// ╭───────────────╮
// │    HANDLER    │
// ╰───────────────╯

/// Interrupts normal signal flow to force-restore the terminal to a saved state.
///
/// Because every function call must be `async-signal-safe` (POSIX), we are unable to rely on the
/// guard's built-in `restore()`, and must manually rebuild it.
extern "C" fn signal_handler(signal: libc::c_int) {
    // skip if guard already marked for release (idempotent)
    if !guard::kill() { return; }

    let snapshot_ptr = SNAPSHOT_PTR.load(Ordering::Acquire);

    if !snapshot_ptr.is_null() {
        unsafe {
            let snapshot = &*snapshot_ptr;

            // restore terminal state (async-signal-safe ops only)
            libc::tcsetattr(snapshot.tty_fd, libc::TCSANOW, &snapshot.termios);
            libc::close(snapshot.tty_fd);

            // leak snapshot intentionally since handler context can't safely dealloc
            // process exit will reclaim memory anyway
        }
    }

    // re-raise signal with default disposition so process terminates
    let mut default: libc::sigaction = unsafe { std::mem::zeroed() };
    default.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;
    unsafe {
        libc::sigaction(signal, &default, ptr::null_mut());
        libc::raise(signal);
    }
}


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

// ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
//      handler
// ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

/// Installs signal handler via `sigaction` to capture `SIGINT`, `SIGTERM`, etc.
///
/// Installation runs through `HANDLED_SIGNALS` array. Performs rollback if install fails for any
/// signal, preventing a half-installed state.
pub(crate) fn install_handlers() -> Result<(), TerminalError> {
    // check if handlers are already installed (idempotent)
    if HANDLERS_INSTALLED.load(Ordering::Acquire) { return Ok(()) }

    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = signal_handler as *const () as libc::sighandler_t;
    action.sa_flags = 0;

    // track installs for rollback on error
    let mut installed: Vec<libc::c_int> = Vec::new();

    // install
    for &signal in &HANDLED_SIGNALS {
        unsafe {
            if libc::sigaction(signal, &action, ptr::null_mut()) != 0 {
                // rollback installs to default to prevent an undefined, half-installed state
                let mut default: libc::sigaction = std::mem::zeroed();
                default.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;

                for &prior in &installed { libc::sigaction(prior, &default, ptr::null_mut()); }

                // raise error
                return Err(TerminalError::BadInstallHandler {
                    signal: signal,
                    source: io::Error::last_os_error(),
                });
            }

            installed.push(signal);
        }
    }

    HANDLERS_INSTALLED.store(true, Ordering::Release);

    Ok(())
}

/// Restores default signal dispositions via `sigaction`.
///
/// Uninstallation runs through `HANDLED_SIGNALS` array.
pub(crate) fn uninstall_handlers() {
    // check if handlers are already uninstalled (idempotent)
    if !HANDLERS_INSTALLED.load(Ordering::Acquire) { return; }

    let mut default: libc::sigaction = unsafe { std::mem::zeroed() };
    default.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;

    unsafe {
        for &signal in &HANDLED_SIGNALS { libc::sigaction(signal, &default, ptr::null_mut()); }
    }

    HANDLERS_INSTALLED.store(false, Ordering::Release);
}

// ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
//      snapshot pointer
// ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

/// Exposes access to the snapshot pointer so that `TerminalGuard` may set it.
pub(crate) fn store_snapshot(ptr: *mut TerminalSnapshot) {
    SNAPSHOT_PTR.store(ptr, Ordering::Release);
}

/// Clears the snapshot pointer (and returns it) (idempotent).
pub(crate) fn clear_snapshot() -> *mut TerminalSnapshot {
    SNAPSHOT_PTR.swap(ptr::null_mut(), Ordering::AcqRel)
}
