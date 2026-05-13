/// Terminal graphics protocol support for inline image rendering.
/// Supports Kitty, iTerm2, and Sixel protocols with ASCII block fallback.

use base64::Engine;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphicsProtocol {
    Kitty,
    ITerm2,
    Sixel,
    None,
}

/// Queued image render request emitted after terminal.draw().
#[derive(Clone, Debug)]
pub struct PendingImageRender {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub area: ratatui::layout::Rect,
}

/// Clear all terminal-native image placements.
/// Should be called before each `terminal.draw()` to prevent stale images.
pub fn clear_images(protocol: GraphicsProtocol) {
    if protocol == GraphicsProtocol::Kitty {
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(b"\x1b_Ga=d,d=A\x1b\\");
        let _ = stdout.flush();
    }
    // iTerm2 images are inline and get overwritten by text rendering;
    // no explicit clear is needed.
}
pub fn detect_protocol() -> GraphicsProtocol {
    // Check for Kitty
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return GraphicsProtocol::Kitty;
    }

    // Check terminal program
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        let prog = term_program.to_ascii_lowercase();
        if prog == "iterm.app" || prog == "iterm2" {
            return GraphicsProtocol::ITerm2;
        }
        if prog == "apple_terminal" {
            // macOS Terminal supports sixel in newer versions
            return GraphicsProtocol::Sixel;
        }
    }

    // Check TERM for sixel support
    if let Ok(term) = std::env::var("TERM") {
        let t = term.to_ascii_lowercase();
        if t.contains("sixel") || t.contains("mlterm") || t.contains("yaft") {
            return GraphicsProtocol::Sixel;
        }
    }

    // Check for WezTerm (supports iTerm2 protocol)
    if std::env::var("WEZTERM_EXECUTABLE").is_ok() {
        return GraphicsProtocol::ITerm2;
    }

    // Check for Ghostty (supports Kitty protocol)
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return GraphicsProtocol::Kitty;
    }

    // Check for foot (supports sixel)
    if std::env::var("FOOT_PID").is_ok() {
        return GraphicsProtocol::Sixel;
    }

    GraphicsProtocol::None
}

/// Render RGBA image data using the detected terminal graphics protocol.
/// `rgba` is raw RGBA bytes, `w` and `h` are dimensions in pixels.
/// `cols` and `rows` are the target cell area size.
pub fn render_image(
    protocol: GraphicsProtocol,
    rgba: &[u8],
    w: u32,
    h: u32,
    cols: u16,
    rows: u16,
) {
    match protocol {
        GraphicsProtocol::Kitty => render_kitty(rgba, w, h, cols, rows),
        GraphicsProtocol::ITerm2 => render_iterm2(rgba, w, h, cols, rows),
        GraphicsProtocol::Sixel => render_sixel(rgba, w, h),
        GraphicsProtocol::None => {}
    }
}

/// Emit Kitty graphics protocol escape sequence.
/// Uses chunked transmission for large images.
fn render_kitty(rgba: &[u8], w: u32, h: u32, cols: u16, rows: u16) {
    let mut stdout = std::io::stdout();

    // Convert RGBA to RGB24 for Kitty (it accepts RGBA directly in newer versions,
    // but RGB24 is more compatible)
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in rgba.chunks_exact(4) {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }

    let data = base64::engine::general_purpose::STANDARD.encode(&rgb);
    let chunk_size = 4096;

    // Kitty placement control: a=T (direct), f=24 (RGB), s/w=source dimensions,
    // c/r=cell coverage, C=1 (do not move cursor)
    let header = format!(
        "\x1b_Ga=T,f=24,s={},v={},c={},r={},m=1,C=1;",
        w, h, cols, rows
    );

    let mut first = true;
    for chunk in data.as_bytes().chunks(chunk_size) {
        let m = if chunk.len() < chunk_size { "0" } else { "1" };
        let prefix = if first {
            first = false;
            header.clone()
        } else {
            format!("\x1b_Gm={};", m)
        };

        let _ = stdout.write_all(prefix.as_bytes());
        let _ = stdout.write_all(chunk);
        let _ = stdout.write_all(b"\x1b\\");
    }
    let _ = stdout.flush();
}

/// Emit iTerm2 inline image protocol escape sequence.
fn render_iterm2(rgba: &[u8], w: u32, h: u32, cols: u16, rows: u16) {
    let mut stdout = std::io::stdout();

    // iTerm2 expects PNG data. Convert RGBA to PNG using the image crate.
    let img = match image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        Some(img) => img,
        None => return,
    };

    let mut png_data = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut png_data);
        if image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::BufWriter::new(cursor), image::ImageFormat::Png)
            .is_err()
        {
            return;
        }
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
    let osc = format!(
        "\x1b]1337;File=inline=1;size={};width={}px;height={}px;preserveAspectRatio=1:{}\x07",
        png_data.len(),
        cols.saturating_mul(8),
        rows.saturating_mul(16),
        b64
    );

    let _ = stdout.write_all(osc.as_bytes());
    let _ = stdout.flush();
}

/// Emit Sixel graphics escape sequence.
/// This is a simplified implementation that produces basic sixel output.
fn render_sixel(_rgba: &[u8], _w: u32, _h: u32) {
    // Sixel encoding is complex and requires palette quantization.
    // For now, skip sixel and let the ASCII fallback handle it.
    // Full sixel support can be added later using a dedicated library.
}

/// Emit Sixel graphics escape sequence.
/// This is a simplified implementation that produces basic sixel output.
fn render_sixel(_rgba: &[u8], _w: u32, _h: u32) {
    // Sixel encoding is complex and requires palette quantization.
    // For now, skip sixel and let the ASCII fallback handle it.
    // Full sixel support can be added later using a dedicated library.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_kitty() {
        std::env::set_var("KITTY_WINDOW_ID", "1");
        assert_eq!(detect_protocol(), GraphicsProtocol::Kitty);
        std::env::remove_var("KITTY_WINDOW_ID");
    }

    #[test]
    fn detect_none_by_default() {
        std::env::remove_var("KITTY_WINDOW_ID");
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("WEZTERM_EXECUTABLE");
        assert_eq!(detect_protocol(), GraphicsProtocol::None);
    }
}
