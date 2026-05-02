use dracon_terminal_engine::{
    input::event::{Event, KeyCode, KeyEvent}, // Correct imports
    input::parser::Parser,
    visuals::image::{ImageOptions, ImageProtocol},
    Terminal,
};
use std::io::{self, stdout, Read, Write};

fn main() -> io::Result<()> {
    let stdout = stdout();
    let mut term = Terminal::new(stdout)?;

    write!(term, "\x1b[2J\x1b[H")?;
    write!(term, "Generating High-Res Image (Kitty Protocol)...\r\n")?;

    // Generate 256x256 RGBA Gradient
    let width = 256;
    let height = 256;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let r = (x % 256) as u8;
            let g = (y % 256) as u8;
            let b = 128;
            let a = 255;
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(a);
        }
    }

    // Display
    ImageProtocol::display_rgba(&mut term, &data, width, height, 1, ImageOptions::default())?;

    write!(
        term,
        "\r\nImage Displayed. Press 'q' to clear and exit.\r\n"
    )?;
    term.flush()?;

    // Wait for Input
    let mut parser = Parser::new();
    let mut stdin = io::stdin();
    let mut buf = [0u8; 128];
    loop {
        if let Ok(n) = stdin.read(&mut buf) {
            for &byte in &buf[..n] {
                if let Some(event) = parser.advance(byte) {
                    if let Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }) = event
                    {
                        // Cleanup
                        ImageProtocol::delete_all(&mut term)?;
                        return Ok(());
                    }
                }
            }
        }
    }
}
