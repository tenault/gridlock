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
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

use super::signal;
use super::termios;
use super::tty::{self, TerminalError, TerminalSnapshot};


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

/// Idempotency flag, enforces guard singleton and ensures a single restore across all paths.
static GUARD_ALIVE: AtomicBool = AtomicBool::new(false);


// ╭───────────────────────────╮
// │    TERMINAL RAII GUARD    │
// ╰───────────────────────────╯

/// Owns the terminal snapshot and guarantees state restoration on drop.
///
/// Construct with [`TerminalGuard::acquire`]. While the guard is alive, you can call
/// [`TerminalGuard::snapshot`] to inspect the original state, or [`TerminalGuard::tty_fd`] to get
/// the file descriptor for subsequent `tcsetattr` calls.
pub struct TerminalGuard {
    snapshot: Option<TerminalSnapshot>,
    _marker: PhantomData<*mut ()>, // constrains !Send + !Sync
}

impl TerminalGuard {

    // ───── constructor ─────

    /// Acquires the current terminal state.
    ///
    /// Opens `/dev/tty` instead of assuming `fd 0` is a terminal, so this succeeds even when
    /// `stdin` is redirected.
    pub fn acquire() -> Result<Self, TerminalError> {
        // enforce singleton
        if GUARD_ALIVE.load(Ordering::Acquire) { return Err(TerminalError::ExistingGuard); }

        let tty_fd = tty::open_tty()?;

        let termios = match termios::get_termios(tty_fd) {
            Ok(t) => t,
            Err(e) => {
                unsafe { libc::close(tty_fd); }
                return Err(e);
            }
        };

        let (rows, cols) = match tty::query_winsize(tty_fd) {
            Ok(size) => size,
            Err(e) => {
                unsafe { libc::close(tty_fd); }
                return Err(e);
            }
        };

        // setup handlers for SIGINT, SIGTERM, etc
        signal::install_handlers()?;

        // all potential errors thrown, guard can be built
        GUARD_ALIVE.store(true, Ordering::Release);

        let snapshot = TerminalSnapshot {
            termios,
            ws_rows: rows,
            ws_cols: cols,
            tty_fd
        };

        // store the snapshot pointer globally for signal handlers
        //
        // we intentionally leak a clone so that the pointer remains valid even if the guard is
        // dropped mid-panic before the signal fires
        signal::store_snapshot(Box::into_raw(Box::new(snapshot.clone())));

        Ok(Self {
            snapshot: Some(snapshot),
            _marker: PhantomData,
        })
    }

    // ───── utility ─────

    /// Borrows the snapshot, panics if the guard was already released.
    pub fn snapshot(&self) -> &TerminalSnapshot {
        self.snapshot
            .as_ref()
            .expect("guard.snapshot() called after release!")
    }

    /// Gets the controlling tty fd from the snapshot, useful for subsequent `tcsetattr` calls.
    pub fn tty_fd(&self) -> RawFd { self.snapshot().tty_fd }

    // ───── cleanup ─────

    /// Explicitly restores the terminal and releases the guard early.
    pub fn release(mut self) -> Result<(), TerminalError> {
        let err = self.restore();
        std::mem::forget(self); // prevents double-free since we took ownership
        return err;
    }

    // ╶╶╶╶╶ internal ╴╴╴╴╴

    fn restore(&mut self) -> Result<(), TerminalError> {
        // check if already restored (idempotent)
        if !GUARD_ALIVE.swap(false, Ordering::AcqRel) { return Ok(()); }

        // restore terminal state
        let restore_err = if let Some(snapshot) = self.snapshot.take() {
            let err = termios::set_termios(snapshot.tty_fd, &snapshot.termios);
            unsafe { libc::close(snapshot.tty_fd); }
            err
        } else {
            Ok(())
        };

        // cleanup global snapshot pointer and reclaim leak
        let snapshot_ptr = signal::clear_snapshot();
        if !snapshot_ptr.is_null() {
            // if signal handler claimed snapshot, this is a safe no-op
            // otherwise, we free what we leaked during ::acquire()
            unsafe { let _ = Box::from_raw(snapshot_ptr); }
        }

        // restore default signal dispositions
        signal::uninstall_handlers();

        return restore_err;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) { let _ = self.restore(); }
}


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Explicitly marks `GUARD_ALIVE = false` for external idempotency (used by `signal_handler()`).
///
/// Does __not__ release the guard itself. Caller must perform manual cleanup, or accept unsafe loss
/// of singleton enforcement.
pub(crate) fn kill() -> bool { GUARD_ALIVE.swap(false, Ordering::AcqRel) }
