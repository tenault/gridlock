// ╭───────────────────────────────────────────────────────────────io/error.rs─╮
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
use std::os::unix::io::RawFd;


// ╭──────────────────────╮
// │    TERMINAL ERROR    │
// ╰──────────────────────╯

/// Errors that can arise during terminal state acquisition and management.
#[derive(Debug)]
pub enum TerminalError {
    /// Failed to open the controlling terminal (`/dev/tty`).
    ///
    /// This is likely because:
    /// - No controlling terminal exists (e.g. daemon process)
    /// - Permission denied accessing the TTY device
    /// - The system has no TTY device available
    OpenTTY { source: io::Error },

    /// Failed to close the controlling terminal.
    ///
    /// This is likely due to:
    /// - `EINTR`: Interrupted by signal, fd state is unspecified
    /// - `EIO`: Device-level I/O error (very rare)
    CloseTTY { fd: RawFd, source: io::Error },

    /// Failed to retrieve current terminal settings.
    ///
    /// Occurs when `tcgetattr()` fails on the terminal file descriptor.
    GetTermios { fd: RawFd, source: io::Error },

    /// Failed to apply terminal settings.
    ///
    /// Occurs when `tcsetattr()` fails on the terminal file descriptor. May leave the terminal in
    /// an inconsistent state.
    SetTermios { fd: RawFd, source: io::Error },

    /// Failed to query terminal dimensions.
    ///
    /// Occurs when `ioctl(TIOCGWINSZ)` fails on the terminal file descriptor.
    WinSize { fd: RawFd, source: io::Error },

    /// Failed to read bytes from the terminal file descriptor.
    ///
    /// `EINTR` is handled internally, so this captures all other read errors.
    Read { fd: RawFd, read: usize, source: io::Error },

    /// Failed to write bytes to the terminal file descriptor.
    ///
    /// `EINTR` is handled internally, so this captures all other write errors.
    Write { fd: RawFd, written: usize, total: usize, source: io::Error },

    /// Failed to install handler for a specific signal.
    ///
    /// Occurs during guard acquisition when installing `SIGINT`/`SIGTERM`/`SIGABRT` handlers.
    InstallSignal { signal: i32, source: io::Error },

    /// Failed to uninstall handler for a specific signal.
    ///
    /// Occurs during guard restoration when uninstalling `SIGINT`/`SIGTERM`/`SIGABRT` handlers.
    UninstallSignal { signal: i32, source: io::Error },

    /// Failed to enter the alternate screen buffer.
    ///
    /// The escape sequence was emitted, but the terminal may not have responded correctly.
    EnterAlternateScreen,

    /// Failed to exit the alternate screen buffer.
    ///
    /// The escape sequence was emitted, but the terminal may not have responded correctly.
    ExitAlternateScreen,

    /// Failed to setup new guard due to singleton violation.
    ///
    /// [`TerminalGuard::acquire()`] can only succeed once per process. Call
    /// [`TerminalGuard::release()`] first, or redesign your architecture to avoid multiple guards.
    IllegalGuard,

    /// Attempted double-restore of the terminal.
    ///
    /// Indicates a non-fatal logic error in the guard's lifecycle management.
    AlreadyRestored,

    /// Attempted an operation on a closed terminal file descriptor.
    ///
    /// Occurs when the controlling terminal was closed (e.g. via `guard.release()`), but an
    /// operation was still attempted which requires an open fd.
    InvalidFd,
}

impl TerminalError {
    /// Returns whether the error is likely recoverable by retrying.
    ///
    /// Generally, transient errors (resource exhaustion, temporary unavailability) are
    /// recoverable. Configuration and permission errors are not.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::OpenTTY           { source }
            | Self::CloseTTY        { source, .. }
            | Self::GetTermios      { source, .. }
            | Self::SetTermios      { source, .. }
            | Self::WinSize         { source, .. }
            | Self::Read            { source, .. }
            | Self::Write           { source, .. }
            | Self::InstallSignal   { source, .. }
            | Self::UninstallSignal { source, .. } => {
                matches!(
                    source.kind(),
                    io::ErrorKind::WouldBlock
                    | io::ErrorKind::TimedOut
                    | io::ErrorKind::Interrupted
                    | io::ErrorKind::OutOfMemory
                )
            },

            // state violations are never recoverable. code better.
            Self::IllegalGuard
            | Self::AlreadyRestored
            | Self::EnterAlternateScreen
            | Self::ExitAlternateScreen
            | Self::InvalidFd => false,
        }
    }

    /// Returns the underlying OS error code (if it exists).
    ///
    /// Useful for matching against specific codes (e.g. `ENOTTY`, `ENOENT`, etc).
    pub fn get_error_code(&self) -> Option<i32> {
        match self {
            Self::OpenTTY           { source }
            | Self::CloseTTY        { source, .. }
            | Self::GetTermios      { source, .. }
            | Self::SetTermios      { source, .. }
            | Self::WinSize         { source, .. }
            | Self::Read            { source, .. }
            | Self::Write           { source, .. }
            | Self::InstallSignal   { source, .. }
            | Self::UninstallSignal { source, .. } => source.raw_os_error(),
            _ => None,
        }
    }

    /// Returns a human-readable summary of the error.
    pub fn summary(&self) -> &'static str {
        match self {
            Self::OpenTTY    { .. } => "Failed to open the controlling terminal",
            Self::CloseTTY   { .. } => "Failed to close the controlling terminal",
            Self::GetTermios { .. } => "Failed to read terminal settings",
            Self::SetTermios { .. } => "Failed to apply terminal settings",
            Self::WinSize    { .. } => "Failed to query terminal dimensions",
            Self::Read       { .. } => "Failed to read from terminal",
            Self::Write      { .. } => "Failed to write to terminal",

            Self::InstallSignal { signal, .. } => {
                match Self::get_signal_name(*signal) {
                    "SIGINT"  => "Failed to install <Ctrl+C> handler",
                    "SIGTERM" => "Failed to install termination handler",
                    "SIGABRT" => "Failed to install abort handler",
                    _         => "Failed to install signal handler",
                }
            },
            Self::UninstallSignal { signal, .. } => {
                match Self::get_signal_name(*signal) {
                    "SIGINT"  => "Failed to restore default <Ctrl+C> handler",
                    "SIGTERM" => "Failed to restore default termination handler",
                    "SIGABRT" => "Failed to restore default abort handler",
                    _         => "Failed to restore default signal handler",
                }
            },

            Self::EnterAlternateScreen => "Failed to enter alternate screen buffer",
            Self::ExitAlternateScreen  => "Failed to exit alternate screen buffer",
            Self::IllegalGuard         => "Terminal guard already active",
            Self::AlreadyRestored      => "Terminal already restored",
            Self::InvalidFd            => "Terminal file descriptor is closed",
        }
    }

    // ╶╶╶╶╶ internal ╴╴╴╴╴

    /// Returns the name of a given signal, or `unknown` if unmapped.
    fn get_signal_name(signal: i32) -> &'static str {
        match signal {
            libc::SIGABRT => "SIGABRT",
            libc::SIGINT  => "SIGINT",
            libc::SIGTERM => "SIGTERM",
            _             => "unknown",
        }
    }
}


// ╭──────────────────╮
// │    EXTENSIONS    │
// ╰──────────────────╯

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenTTY { source } => {
                write!(f, "Failed to open /dev/tty: {source}")
            },
            Self::CloseTTY { fd, source } => {
                write!(f, "Failed to close controlling fd {fd}: {source}")
            },
            Self::GetTermios { fd, source } => {
                write!(f, "Failed to get termios for fd {fd}: {source}")
            },
            Self::SetTermios { fd, source } => {
                write!(f, "Failed to set termios on fd {fd}: {source}")
            },
            Self::WinSize { fd, source } => {
                write!(f, "Failed to query window size for fd {fd}: {source}")
            },
            Self::Read { fd, read, source } => {
                if *read > 0 {
                    write!(f, "Read failed after {read} bytes on fd {fd}: {source}")
                } else {
                    write!(f, "Read failed on fd {fd}: {source}")
                }
            },
            Self::Write { fd, written, total, source } => {
                if *written < *total {
                    write!(f, "Write incomplete ({written}/{total}) on fd {fd}: {source}")
                } else {
                    write!(f, "Write failed on fd {fd}: {source}")
                }
            },
            Self::InstallSignal { signal, source } => {
                let name = Self::get_signal_name(*signal);
                write!(f, "Failed to install handler for {name} ({signal}): {source}")
            },
            Self::UninstallSignal { signal, source } => {
                let name = Self::get_signal_name(*signal);
                write!(f, "Failed to restore default disposition for {name} ({signal}): {source}")
            },
            Self::EnterAlternateScreen => {
                write!(f, "Failed to enter alternate screen buffer (escape may be unsupported)")
            },
            Self::ExitAlternateScreen => {
                write!(f, "Failed to exit alternate screen buffer (escape may be unsupported)")
            },
            Self::IllegalGuard => {
                write!(f, "Terminal guard already exists")
            },
            Self::AlreadyRestored => {
                write!(f, "Terminal already restored")
            },
            Self::InvalidFd => {
                write!(f, "Attempted operation on a closed terminal file descriptor")
            },
        }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OpenTTY           { source }
            | Self::CloseTTY        { source, .. }
            | Self::GetTermios      { source, .. }
            | Self::SetTermios      { source, .. }
            | Self::WinSize         { source, .. }
            | Self::Read            { source, .. }
            | Self::Write           { source, .. }
            | Self::InstallSignal   { source, .. }
            | Self::UninstallSignal { source, .. } => Some(source),
            _ => None,
        }
    }
}
