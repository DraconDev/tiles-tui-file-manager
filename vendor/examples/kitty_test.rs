use dracon_terminal_engine::visuals::image::{ImageOptions, ImageProtocol};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();

    // Clear screen and home cursor
    write!(stdout, "\x1b[2J\x1b[H")?;

    println!("Testing Kitty Graphics Protocol Support...");
    println!("If supported, you should see a red square below this line:");
    println!();

    // Generate a 100x100 red square (RGBA)
    let width = 100;
    let height = 100;
    let mut data = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        data.push(255); // R
        data.push(0); // G
        data.push(0); // B
        data.push(255); // A
    }

    // Direct display (Action 'T')
    let opts = ImageOptions::default();
    ImageProtocol::display_rgba(&mut stdout, &data, width as u32, height as u32, 999, opts)?;

    stdout.flush()?;

    println!();
    println!();
    println!("If you see the square, the protocol works.");
    println!("If not, your terminal might not support Kitty Graphics or SGR 1006.");

    Ok(())
}
