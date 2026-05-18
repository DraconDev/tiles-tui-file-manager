//! Clipboard utilities — extracted from event_helpers.rs.
//!
//! Provides system clipboard copy (wayland/X11/macOS) with OSC 52 fallback.

use std::process::{Command, Stdio};
use base64::Engine;

/// Copy text to the system clipboard.
///
/// Tries: wl-copy, xclip, xsel, pbcopy (in order).
/// Falls back to OSC 52 terminal escape sequence.
pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let attempts: [(&str, &[&str]); 4] = [
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("pbcopy", &[]),
    ];

    let mut last_err = None;
    for (cmd, args) in attempts {
        match Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    if stdin.write_all(text.as_bytes()).is_err() {
                        last_err = Some(format!("{} rejected clipboard data", cmd));
                        let _ = child.kill();
                        continue;
                    }
                }

                match child.wait() {
                    Ok(status) if status.success() => return Ok(()),
                    Ok(_) => last_err = Some(format!("{} exited unsuccessfully", cmd)),
                    Err(err) => last_err = Some(format!("{} failed: {}", cmd, err)),
                }
            }
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    last_err = Some(format!("{} failed: {}", cmd, err));
                }
            }
        }
    }

    copy_text_to_clipboard_via_osc52(text).map_err(|osc_err| {
        let fallback = last_err.unwrap_or_else(|| {
            "No clipboard helper found (tried wl-copy, xclip, xsel, pbcopy)".to_string()
        });
        format!("{}; OSC 52 fallback failed: {}", fallback, osc_err)
    })
}

/// Copy text to clipboard in a background thread.
pub fn copy_text_to_clipboard_async(text: String) {
    std::thread::spawn(move || {
        let _ = copy_text_to_clipboard(&text);
    });
}

/// OSC 52 terminal escape sequence clipboard copy (fallback).
fn copy_text_to_clipboard_via_osc52(text: &str) -> Result<(), String> {
    use std::io::Write;

    let term = std::env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return Err("terminal does not support clipboard escape sequences".to_string());
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    let sequence = format!("\u{1b}]52;c;{}\u{07}", encoded);

    let mut stdout = std::io::stdout();
    stdout
        .write_all(sequence.as_bytes())
        .map_err(|err| format!("write failed: {}", err))?;
    stdout
        .flush()
        .map_err(|err| format!("flush failed: {}", err))?;
    Ok(())
}
