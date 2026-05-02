use crate::backend::tty::{get_terminal_attr, make_raw, set_terminal_attr, Termios};
use std::io::{self, Write};
use std::os::fd::{AsFd, BorrowedFd};

/// The main RAII wrapper for the terminal.
/// When this struct is dropped, the terminal is restored to its original state.
pub struct Terminal<W: Write + AsFd> {
    original_termios: Termios,
    output: W,
}

impl<W: Write + AsFd> Drop for Terminal<W> {
    fn drop(&mut self) {
        // cleanup: show cursor, disable mouse, leave alt screen, pop kitty keyboard
        let _ = write!(
            self.output,
            "\x1b[<u\x1b[?25h\x1b[?1l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1007h\x1b[?7h\x1b[?1049l"
        );
        let _ = self.output.flush();
        // Restore terminal attributes
        let _ = set_terminal_attr(self.output.as_fd(), &self.original_termios);
    }
}

impl<W: Write + AsFd> Terminal<W> {
    /// Enter "God Mode" (Raw Mode + Alternate Screen).
    pub fn new(mut writer: W) -> io::Result<Self> {
        let fd = writer.as_fd();
        let mut termios = get_terminal_attr(fd)?;
        let original_termios = termios;

        make_raw(&mut termios);
        set_terminal_attr(fd, &termios)?;

        // Safe Capture: Alt Screen, Mouse (Button Event + SGR), Kitty Keyboard, No Alt Scroll, No Wrap, No Cursor
        write!(
            writer,
            "\x1b[>1u\x1b[?1049h\x1b[?1003h\x1b[?1006h\x1b[?1007l\x1b[?7l\x1b[?25l"
        )?;
        write!(writer, "\x1b[2J\x1b[H")?;
        writer.flush()?;

        Ok(Self {
            original_termios,
            output: writer,
        })
    }

    /// Access the underlying writer (e.g., to flush)
    pub fn inner(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: Write + AsFd> Write for Terminal<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<W: Write + AsFd> AsFd for Terminal<W> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.output.as_fd()
    }
}
