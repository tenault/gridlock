use gridlock::TerminalGuard;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let guard = TerminalGuard::acquire()?;
    let snap = guard.snapshot();

    println!("Terminal acquired.");
    println!("tty_fd  :: {}", snap.tty_fd);
    println!("winsize :: {} cols x {} rows", snap.ws_cols, snap.ws_rows);
    println!("c_iflag :: 0x{:08x}", snap.orig_termios.c_iflag);
    println!("c_oflag :: 0x{:08x}", snap.orig_termios.c_oflag);
    println!("c_cflag :: 0x{:08x}", snap.orig_termios.c_cflag);
    println!("c_lflag :: 0x{:08x}", snap.orig_termios.c_lflag);
    
    print!("c_cc    :: [");
    for (i, &cc) in snap.orig_termios.c_cc.iter().take(11).enumerate() {
        if i > 0 { print!(", "); }
        print!("0x{:02x}", cc);
    }
    println!(", ...]");

    Ok(())
}
