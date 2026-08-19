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

// ╭───────────────────╮
// │    ENVIRONMENT    │
// ╰───────────────────╯

use std::io;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use super::escape;
use super::terminal;
use super::error::TerminalError;


// ╭───────────────╮
// │    SYMBOLS    │
// ╰───────────────╯

const HANDLED_SIGNALS: [libc::c_int; 3] = [
    libc::SIGABRT,
    libc::SIGINT,
    libc::SIGTERM,
];

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
    let snapshot_ptr = terminal::clear_snapshot();
    if !snapshot_ptr.is_null() {
        unsafe {
            let snapshot = &*snapshot_ptr;

            // attempt exit of the alternate screen buffer
            libc::write(
                snapshot.fd,
                escape::EXIT_ALT_SCREEN.as_ptr() as *const _,
                escape::EXIT_ALT_SCREEN.len()
            );

            // restore terminal state
            libc::tcflush(snapshot.fd, libc::TCIFLUSH); // drop input queue
            libc::tcsetattr(snapshot.fd, libc::TCSANOW, &snapshot.termios);
            libc::close(snapshot.fd);

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

/// Installs signal handler via `sigaction` to capture `SIGINT`, `SIGTERM`, etc.
///
/// Installation runs through `HANDLED_SIGNALS` array. Performs rollback if install fails for any
/// signal, preventing a half-installed state.
pub(crate) fn install_handlers() -> Result<(), TerminalError> {
    // check if handlers are already installed (idempotent)
    if HANDLERS_INSTALLED.load(Ordering::Acquire) { return Ok(()); }

    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = signal_handler as *const () as libc::sighandler_t;
    action.sa_flags = 0;

    // track installs for rollback on error
    let mut installed: Vec<libc::c_int> = Vec::new();

    unsafe { // install handlers
        for &signal in &HANDLED_SIGNALS {
            if libc::sigaction(signal, &action, ptr::null_mut()) != 0 {
                // rollback installs to default to prevent an undefined, half-installed state
                let mut default: libc::sigaction = std::mem::zeroed();
                default.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;

                for &prior in &installed { libc::sigaction(prior, &default, ptr::null_mut()); }

                // raise error
                return Err(TerminalError::InstallSignal {
                    signal,
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
pub(crate) fn uninstall_handlers() -> Result<(), TerminalError> {
    // check if handlers are already uninstalled (idempotent)
    if !HANDLERS_INSTALLED.load(Ordering::Acquire) { return Ok(()); }

    let mut default: libc::sigaction = unsafe { std::mem::zeroed() };
    default.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;

    unsafe { // uninstall handlers
        for &signal in &HANDLED_SIGNALS {
            if libc::sigaction(signal, &default, ptr::null_mut()) != 0 {
                return Err(TerminalError::UninstallSignal {
                    signal,
                    source: io::Error::last_os_error(),
                });
            }
        }
    }

    HANDLERS_INSTALLED.store(false, Ordering::Release);
    Ok(())
}
