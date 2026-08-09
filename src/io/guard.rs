// ╭───────────────────────────────────────────────────────────────io/guard.rs─╮
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

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

use super::screen;
use super::signal;
use super::tty::{TTY, TerminalError, TerminalSnapshot};


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

/// Idempotency flag, enforces guard singleton and ensures a single restore across all paths.
static GUARD_ALIVE: AtomicBool = AtomicBool::new(false);


// ╭──────────────────────╮
// │    TERMINAL GUARD    │
// ╰──────────────────────╯

/// Owns the terminal snapshot and guarantees state restoration on drop.
///
/// Construct with [`TerminalGuard::acquire`]. While the guard is alive, you can call
/// [`TerminalGuard::snapshot`] to inspect the original state, or [`TerminalGuard::tty_fd`] to get
/// the file descriptor for subsequent `tcsetattr` calls.
pub struct TerminalGuard {
    tty: TTY,
    snapshot: TerminalSnapshot,
    _marker: PhantomData<*mut ()>, // constrains !Send + !Sync
}

impl TerminalGuard {

    // ───── constructor ─────

    /// Acquires the current terminal and initializes it for client control.
    ///
    /// Opens `/dev/tty` instead of assuming `fd 0` is a terminal, so this succeeds even when
    /// `stdin` is redirected.
    pub fn acquire() -> Result<Self, TerminalError> {
        // enforce singleton
        if GUARD_ALIVE.load(Ordering::Acquire) { return Err(TerminalError::ExistingGuard); }

        let tty = TTY::open()?;

        let termios = match tty.get_termios() {
            Ok(t) => t,
            Err(e) => {
                unsafe { libc::close(tty.fd); }
                return Err(e);
            }
        };

        let (rows, cols) = match tty.query_winsize() {
            Ok(size) => size,
            Err(e) => {
                unsafe { libc::close(tty.fd); }
                return Err(e);
            }
        };

        // setup handlers for SIGINT, SIGTERM, etc
        signal::install_handlers()?;

        // enter asb with known raw mode
        tty.uncook()?;
        screen::enter_asb(tty.fd);

        // all potential errors thrown, guard can now be built
        GUARD_ALIVE.store(true, Ordering::Release);

        let snapshot = TerminalSnapshot {
            termios,
            ws_rows: rows,
            ws_cols: cols,
            tty_fd: tty.fd,
        };

        // store the snapshot pointer globally for signal handlers
        //
        // we intentionally leak a clone so that the pointer remains valid even if the guard is
        // dropped mid-panic before the signal fires
        signal::store_snapshot(Box::into_raw(Box::new(snapshot.clone())));

        Ok(Self {
            tty,
            snapshot,
            _marker: PhantomData,
        })
    }

    // ───── cleanup ─────

    /// Explicitly restores the terminal and releases the guard early.
    pub fn release(mut self) -> Result<(), TerminalError> {
        let err = self.restore();
        std::mem::forget(self); // prevents double-drop since we took ownership
        return err;
    }

    // ╶╶╶╶╶ internal ╴╴╴╴╴

    fn restore(&mut self) -> Result<(), TerminalError> {
        // check if already restored (idempotent)
        if !GUARD_ALIVE.swap(false, Ordering::AcqRel) { return Ok(()); }

        // cleanup global snapshot pointer and reclaim leak
        //
        // if signal handler claimed snapshot, this is a safe no-op
        // otherwise, we free what we leaked during ::acquire()
        let ptr = signal::clear_snapshot();
        if !ptr.is_null() {
            unsafe { let _ = Box::from_raw(ptr); }
        }

        // restore default signal dispositions
        signal::uninstall_handlers();

        screen::exit_asb(self.tty.fd);
        self.tty.set_termios(&self.snapshot.termios)?;

        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) { let _ = self.restore(); } // swallow errors, just restore
}


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Explicitly marks `GUARD_ALIVE = false` for external idempotency (used by `signal_handler()`).
///
/// Does __not__ release the guard itself. Caller must perform manual cleanup, or accept unsafe loss
/// of singleton enforcement.
pub(crate) fn kill() -> bool { GUARD_ALIVE.swap(false, Ordering::AcqRel) }
