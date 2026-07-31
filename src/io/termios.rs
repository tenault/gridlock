use std::io;
use std::os::unix::io::RawFd;

use super::tty::TerminalError;

/// Gets the terminal state via `tcgetattr`.
///
/// __Does NOT close the file descriptor on error.__ Caller is responsible for cleanup.
pub(crate) fn get_termios(fd: RawFd) -> Result<libc::termios, TerminalError> {
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::tcgetattr(fd, &mut t) };

    if rc != 0 { return Err(TerminalError::BadGetAttr(io::Error::last_os_error())); }

    Ok(t)
}

/// Sets the terminal state via `tcsetattr`.
///
/// __Does NOT close the file descriptor on error.__ Caller is responsible for cleanup.
pub(crate) fn set_termios(fd: RawFd, t: &libc::termios) -> Result<(), TerminalError> {
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, t) } != 0 {
        return Err(TerminalError::BadSetAttr(io::Error::last_os_error()));
    }

    Ok(())
}
