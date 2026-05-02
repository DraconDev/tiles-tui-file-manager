use std::io::{self, Write};

/// Mode 2026: Synchronized Output
/// When enabled, the terminal will batch all rendering commands until `disable` is called.
/// This prevents screen tearing and flickering during complex frame updates.
///
/// Ref: https://gist.github.com/christianparpart/d8a62cc4942b327ed587
pub struct SyncGuard;

impl SyncGuard {
    /// Start a synchronized update block.
    pub fn begin<W: Write>(writer: &mut W) -> io::Result<()> {
        write!(writer, "\x1b[?2026h")
    }

    /// End a synchronized update block, triggering the frame render.
    pub fn end<W: Write>(writer: &mut W) -> io::Result<()> {
        write!(writer, "\x1b[?2026l")
    }
}
