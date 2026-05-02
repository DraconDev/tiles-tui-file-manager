use std::io::{self, Write};

/// copy text to the system clipboard using OSC 52.
/// Format: \x1b]52;c;BASE64_TEXT\x07
pub fn copy_to_clipboard<W: Write>(writer: &mut W, text: &str) -> io::Result<()> {
    // Basic base64 encoding (manual implementing to avoid deps or use a crate if allowed?
    // Plan said "not heavy", so let's use a simple implementation or assume user has `base64` crate?
    // The user said "not heavy", adding `base64` is probably acceptable, BUT
    // I can implement a tiny encoder here since we only need encode.

    let encoded = simple_base64_encode(text.as_bytes());
    write!(writer, "\x1b]52;c;{}\x07", encoded)
}

/// Create a hyperlink using OSC 8.
/// Format: \x1b]8;;URL\x1b\\TEXT\x1b]8;;\x1b\\
pub fn write_hyperlink<W: Write>(writer: &mut W, text: &str, url: &str) -> io::Result<()> {
    write!(writer, "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}

pub fn simple_base64_encode(input: &[u8]) -> String {
    // ... (existing implementation)
    const SET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len() * 4 / 3 + 4);
    let mut val = 0;
    let mut bits = 0;
    for &byte in input {
        val = (val << 8) | byte as u32;
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(SET[((val >> bits) & 0x3F) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(SET[((val << (6 - bits)) & 0x3F) as usize] as char);
    }
    // Padding
    while !out.len().is_multiple_of(4) {
        out.push('=');
    }
    out
}

/// Triggers the system bell (BEL).
pub fn bell<W: Write>(writer: &mut W) -> io::Result<()> {
    write!(writer, "\x07")
}

/// Send a desktop notification using OSC 777 (standard in some terms).
pub fn notify<W: Write>(writer: &mut W, title: &str, body: &str) -> io::Result<()> {
    write!(writer, "\x1b]777;notify;{};{}\x1b\\", title, body)
}
