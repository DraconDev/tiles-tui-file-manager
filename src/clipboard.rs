/// System clipboard integration for copying file paths.
use arboard::Clipboard;
use std::path::PathBuf;

/// Copy a file path to the system clipboard.
pub fn copy_path_to_clipboard(path: &PathBuf) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {}", e))?;
    let path_str = path.to_string_lossy().to_string();
    clipboard.set_text(path_str).map_err(|e| format!("Clipboard set failed: {}", e))?;
    Ok(())
}

/// Copy multiple file paths to the system clipboard (newline-separated).
pub fn copy_paths_to_clipboard(paths: &[PathBuf]) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {}", e))?;
    let text = paths.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    clipboard.set_text(text).map_err(|e| format!("Clipboard set failed: {}", e))?;
    Ok(())
}

/// Get text from the system clipboard, attempting to parse as paths.
pub fn get_paths_from_clipboard() -> Result<Vec<PathBuf>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {}", e))?;
    let text = clipboard.get_text().map_err(|e| format!("Clipboard get failed: {}", e))?;
    let paths: Vec<PathBuf> = text.lines()
        .map(|line| PathBuf::from(line.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    Ok(paths)
}
