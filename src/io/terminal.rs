// ╭────────────────────────────────────────────────────────────io/terminal.rs─╮
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
use std::marker::PhantomData;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use super::error::TerminalError;
use super::escapes;
use super::signal;
use super::tty::TTY;


// ╭────────────────╮
// │    CONTROLS    │
// ╰────────────────╯

/// Idempotency flag, enforces instance singleton and ensures single restore across all exit paths.
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Raw pointer to the active snapshot, null when no guard is active.
static SNAPSHOT_PTR: AtomicPtr<TerminalSnapshot> = AtomicPtr::new(std::ptr::null_mut());

/// Packed cache of terminal dimensions (high 16 bits is rows, low 16 is cols).
///
/// Zero means "uninitialized" (no successful query has been made yet). This is purely a
/// performance hint; callers needing guaranteed dimensions should use [`TTY::query_winsize()`],
/// which performs `ioctl(TIOCGWINSZ)`.
///
/// `SIGWINCH` handlers should call [`invalidate_winsize_cache()`] to mark this stale, prompting the
/// next hot-loop consumer to re-query.
static WINSIZE_CACHE: AtomicU32 = AtomicU32::new(0);


// ╭────────────────╮
// │    TERMINAL    │
// ╰────────────────╯

/// Owns the terminal snapshot and guarantees state restoration on drop.
pub struct Terminal {
    tty: TTY,
    snapshot: TerminalSnapshot,
    _marker: PhantomData<*mut ()>, // constrains !Send + !Sync

    pub rows: u16,
    pub cols: u16,
}

impl Terminal {

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    constructor    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    // ───── EXTERNAL ─────

    /// Acquires the current terminal and initializes it for client control.
    pub fn acquire() -> Result<Self, TerminalError> {

        // ╶╶╶╶╶ enforce instance singleton ╴╴╴╴╴
        
        if TERMINAL_ACTIVE.swap(true, Ordering::AcqRel) { return Err(TerminalError::IllegalGuard); }

        // ╶╶╶╶╶ construct or error ╴╴╴╴╴

        match Self::construct() {
            Ok(terminal) => Ok(terminal),
            Err(e) => {
                TERMINAL_ACTIVE.store(false, Ordering::Release);
                Err(e)
            },
        }
    }

    // ───── INTERNAL ─────

    fn construct() -> Result<Self, TerminalError> {

        // ╶╶╶╶╶ acquire terminal state ╴╴╴╴╴

        let tty = TTY::open()?;

        let fd           = tty.fd()?;
        let termios      = tty.get_termios()?;
        let (rows, cols) = tty.query_winsize()?;

        // ╶╶╶╶╶ install signal handlers ╴╴╴╴╴

        signal::install_handlers()?; // SIGINT, SIGTERM, etc

        // ╶╶╶╶╶ configure terminal ╴╴╴╴╴

        tty.uncook()?;
        tty.write_raw(escapes::ENTER_ALT_SCREEN)?;

        // ╶╶╶╶╶ save snapshot (for restoration) ╴╴╴╴╴

        let snapshot = TerminalSnapshot { fd, termios };

        // store the snapshot pointer globally for signal handlers
        //
        // we intentionally leak a clone so that the pointer remains valid even if the guard is
        // dropped mid-panic before the signal fires
        SNAPSHOT_PTR.store(Box::into_raw(Box::new(snapshot.clone())), Ordering::Release);

        // ╶╶╶╶╶ deliver instance ╴╴╴╴╴

        Ok(Self {
            tty,
            snapshot,
            _marker: PhantomData,

            rows,
            cols,
        })
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    utility    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    /// Queries the terminal dimensions via `ioctl(TIOCGWINSZ)`.
    pub fn query_dimensions(&mut self) -> Result<(u16, u16), TerminalError> {
        let (rows, cols) = self.tty.query_winsize()?;
        
        WINSIZE_CACHE.store(pack_dimensions(rows, cols), Ordering::Release);
        self.rows = rows;
        self.cols = cols;
        
        Ok((rows, cols))
    }

    /// Returns the cached terminal dimensions, avoiding a syscall.
    pub fn get_cached_dimensions() -> Option<(u16, u16)> {
        unpack_dimensions(WINSIZE_CACHE.load(Ordering::Acquire))
    }

    // ╭╴╴╴╴╴╴╴╴╴╴╴╴╴╴╴╮
    // ·    cleanup    ·
    // ╰╶╶╶╶╶╶╶╶╶╶╶╶╶╶╶╯

    // ───── EXTERNAL ─────

    /// Explicitly restores the terminal and releases the guard early.
    pub fn release(mut self) -> Result<(), TerminalError> {
        let err = self.restore();
        std::mem::forget(self); // prevents double-drop since we took ownership
        return err;
    }

    // ───── INTERNAL ─────

    fn restore(&mut self) -> Result<(), TerminalError> {

        // ╶╶╶╶╶ check if already restored ╴╴╴╴╴

        if !TERMINAL_ACTIVE.swap(false, Ordering::AcqRel) {
            return Err(TerminalError::AlreadyRestored);
        }

        // ╶╶╶╶╶ cleanup snapshot leak ╴╴╴╴╴

        // if signal handler claimed snapshot, this is a safe no-op...
        let ptr = clear_snapshot();
        
        // ...otherwise, we free what we leaked during ::acquire()
        if !ptr.is_null() {
            unsafe { let _ = Box::from_raw(ptr); }
        }

        // ╶╶╶╶╶ restore default signal dispositions ╴╴╴╴╴

        signal::uninstall_handlers()?;

        // ╶╶╶╶╶ restore terminal ╴╴╴╴╴

        self.tty.write_raw(escapes::EXIT_ALT_SCREEN)?;
        self.tty.set_termios(&self.snapshot.termios)?;
        self.tty.close()?;

        Ok(())
    }

}


// ╭──────────────────╮
// │    EXTENSIONS    │
// ╰──────────────────╯

impl Drop for Terminal {
    fn drop(&mut self) { let _ = self.restore(); } // swallow errors, we just wanna restore
}


// ╭─────────────────────╮
// │    SUPPORT TYPES    │
// ╰─────────────────────╯

/// An immutable snapshot of the terminal state at acquisition
#[derive(Debug, Clone)]
pub(crate) struct TerminalSnapshot {
    pub(crate) fd: RawFd,
    pub(crate) termios: libc::termios,
}

impl TerminalSnapshot {
    /// Pretty-prints the saved termios to a writer (for logging, etc)
    pub(crate) fn dump_termios<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        let t = &self.termios;
        writeln!(w, "----[ terminal snapshot ]----------------")?;
        writeln!(w, " -> tty_fd  : {}", self.fd)?;
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


// ╭─────────────────╮
// │    ACCESSORS    │
// ╰─────────────────╯

// ───── ATOMIC: SNAPSHOT_PTR ─────

/// Clears the snapshot pointer (and returns it) (idempotent).
pub(crate) fn clear_snapshot() -> *mut TerminalSnapshot {
    SNAPSHOT_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel)
}

// ───── ATOMIC: WINSIZE_CACHE ─────

/// Invalidates the winsize cache, forcing the next consumer to re-query.
pub(crate) fn invalidate_winsize_cache() { WINSIZE_CACHE.store(0, Ordering::Release); }


// ╭───────────────╮
// │    UTILITY    │
// ╰───────────────╯

/// Packs (rows, cols) into one `u32` for atomic storage.
fn pack_dimensions(rows: u16, cols: u16) -> u32 { ((rows as u32) << 16) | (cols as u32) }

/// Unpacks a `u32` into (rows, cols), or returns `None` if zeroed.
fn unpack_dimensions(pack: u32) -> Option<(u16, u16)> {
    if pack == 0 { return None; }
    Some(((pack >> 16) as u16, pack as u16))
}


