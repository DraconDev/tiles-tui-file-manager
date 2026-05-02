use dracon_terminal_engine::core::terminal::Terminal;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut term = Terminal::new(stdout)?;

    // We can write standard ANSI codes directly
    write!(term, "\x1b[2J")?; // Clear
    write!(term, "\x1b[H")?; // Home

    write!(term, "Welcome to Dracon (Raw Mode).\r\n")?;
    write!(
        term,
        "Press any key to exit (actually, this just sleeps for 2s).\r\n"
    )?;

    // Demonstrate cursor movement
    write!(term, "\x1b[5;10HHello at 5,10")?;

    term.flush()?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    // When `term` goes out of scope, it drops and restores canonical mode.
    Ok(())
}
