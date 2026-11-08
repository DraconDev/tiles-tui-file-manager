//! Clipboard utilities — extracted from event_helpers.rs.
//!
//! Provides system clipboard copy (wayland/X11/macOS) with OSC 52 fallback.

use std::process::{Command, Stdio};
use base64::Engine;

/// Copy text to the system clipboard.
///
/// Tries: wl-copy, xclip, xsel, pbcopy (in order).
/// Falls back to OSC 52 terminal escape sequence.
const CLIPBOARD_HELPERS: [(&str, &[&str]); 4] = [
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("xsel", &["--clipboard", "--input"]),
    ("pbcopy", &[]),
];

pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let term = std::env::var("TERM").unwrap_or_default();
    copy_text_to_clipboard_with(text, &CLIPBOARD_HELPERS, &term)
}

fn copy_text_to_clipboard_with(
    text: &str,
    attempts: &[(&str, &[&str])],
    term: &str,
) -> Result<(), String> {
    let mut last_err = None;
    for &(cmd, args) in attempts {
        match Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    if stdin.write_all(text.as_bytes()).is_err() {
                        last_err = Some(format!("{} rejected clipboard data", cmd));
                        // Release any reader blocked on the pipe, then kill and
                        // reap the child — killing without wait() leaves a
                        // zombie (one per failed attempt).
                        drop(stdin);
                        let _ = child.kill();
                        let _ = child.wait();
                        continue;
                    }
                    // Drop the stdin handle so the child sees EOF before we wait on it;
                    // helpers like wl-copy read until EOF and would otherwise deadlock.
                    drop(stdin);
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

    copy_text_to_clipboard_via_osc52_with_term(text, term).map_err(|osc_err| {
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

fn copy_text_to_clipboard_via_osc52_with_term(text: &str, term: &str) -> Result<(), String> {
    use std::io::Write;

    if term == "dumb" {
        return Err("terminal does not support clipboard escape sequences".to_string());
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    let sequence = format!("\u{1b}]52;c;{}\u{07}", encoded);

    // Hold the stdout lock across the whole sequence so the escape bytes are
    // not interleaved with concurrent output.
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(sequence.as_bytes())
        .map_err(|err| format!("write failed: {}", err))?;
    stdout
        .flush()
        .map_err(|err| format!("flush failed: {}", err))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_no_helpers_and_dumb_term_returns_error() {
        // Hermetic: no helpers to try, and a terminal that cannot do OSC 52.
        let result = copy_text_to_clipboard_with("test", &[], "dumb");
        assert!(result.is_err());
        // Should mention "clipboard helper" or "OSC 52"
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("clipboard") || err_msg.contains("OSC"));
    }

    #[test]
    fn clipboard_dumb_term_rejects_osc52() {
        let result = copy_text_to_clipboard_via_osc52_with_term("test", "dumb");
        assert!(result.is_err());
    }

    #[test]
    fn clipboard_empty_text_is_fine() {
        // Empty string should be processable (no panic).
        let result = copy_text_to_clipboard_via_osc52_with_term("", "xterm");
        let _ = result;
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn clipboard_write_failure_reaps_child() {
        use std::time::{Duration, Instant};

        // Count zombie children of this process via /proc (state Z, ppid == self).
        fn zombie_children(pid: u32) -> usize {
            let mut count = 0;
            let Ok(entries) = std::fs::read_dir("/proc") else { return 0 };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                if !name.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                if let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) {
                    // comm (field 2) may contain spaces/parens; parse from the last ')'.
                    if let Some(close) = stat.rfind(')') {
                        let rest: Vec<&str> = stat[close + 1..].split_whitespace().collect();
                        if rest.len() >= 2 && rest[0] == "Z" && rest[1] == pid.to_string() {
                            count += 1;
                        }
                    }
                }
            }
            count
        }

        // Nothing to exercise without a POSIX shell.
        if Command::new("sh")
            .arg("-c")
            .arg("true")
            .stdin(Stdio::null())
            .status()
            .is_err()
        {
            return;
        }

        let before = zombie_children(std::process::id());
        // `sh` closes its stdin pipe then sleeps, so the parent's write fails
        // with a broken pipe and the helper must be killed and reaped promptly.
        let start = Instant::now();
        let result = copy_text_to_clipboard_with(
            "clipboard payload",
            &[("sh", &["-c", "exec 0<&-; sleep 30"])],
            "dumb",
        );
        let elapsed = start.elapsed();

        assert!(result.is_err(), "broken-pipe helper must not report success");
        assert!(
            elapsed < Duration::from_secs(10),
            "helper must be killed and reaped promptly, took {elapsed:?}"
        );
        assert!(
            zombie_children(std::process::id()) <= before,
            "failed attempt must not leave a zombie child"
        );
    }
}
