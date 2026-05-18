//! Navigation helpers — extracted from event_helpers.rs.
//!
//! Directory navigation functions: back/forward/up, history tracking,
//! path input dialog.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::app::{App, AppEvent, AppMode, FileState};
use crate::config::MAX_HISTORY;

/// Navigate backward in the folder history.
pub fn navigate_back(app: &mut App) {
    if let Some(fs) = app.current_file_state_mut() {
        if fs.nav.history_index > 0 {
            fs.nav.history_index -= 1;
            fs.nav.current_path = fs.nav.history[fs.nav.history_index].clone();
            fs.list.files.clear();
            fs.list.tree_file_depths.clear();
            fs.list.metadata.clear();
            fs.list.local_count = 0;
        }
    }
}

/// Navigate forward in the folder history.
pub fn navigate_forward(app: &mut App) {
    if let Some(fs) = app.current_file_state_mut() {
        if fs.nav.history_index + 1 < fs.nav.history.len() {
            fs.nav.history_index += 1;
            fs.nav.current_path = fs.nav.history[fs.nav.history_index].clone();
            fs.list.files.clear();
            fs.list.tree_file_depths.clear();
            fs.list.metadata.clear();
            fs.list.local_count = 0;
        }
    }
}

/// Push a path onto the navigation history, trimming entries beyond MAX_HISTORY.
pub fn push_history(fs: &mut FileState, path: PathBuf) {
    if fs.nav.history_index + 1 < fs.nav.history.len() {
        fs.nav.history.truncate(fs.nav.history_index + 1);
    }
    if fs.nav.history.last() != Some(&path) {
        fs.nav.history.push(path);
        fs.nav.history_index = fs.nav.history.len() - 1;
    }
    if fs.nav.history.len() > MAX_HISTORY {
        let excess = fs.nav.history.len() - MAX_HISTORY;
        fs.nav.history.drain(0..excess);
        fs.nav.history_index = fs.nav.history_index.saturating_sub(excess);
    }
}

/// Navigate up to the parent directory of the current path.
pub fn navigate_up(app: &mut App) {
    let (old_folder, old_idx, old_scroll, new_path) = {
        let fs = match app.current_file_state_mut() {
            Some(fs) => fs,
            None => return,
        };
        let parent = match fs.nav.current_path.parent() {
            Some(p) => p,
            None => return,
        };
        let old_folder = fs.nav.current_path.clone();
        let old_idx = fs.list.selection.selected.unwrap_or(0);
        let old_scroll = fs.view.table_state.offset();
        let new_path = parent.to_path_buf();
        (old_folder, old_idx, old_scroll, new_path)
    };
    app.selection.folder_selections.insert(old_folder.clone(), (old_idx, old_scroll));
    if let Some(fs) = app.current_file_state_mut() {
        fs.nav.current_path = new_path.clone();
        fs.list.files.clear();
        fs.list.tree_file_depths.clear();
        fs.list.metadata.clear();
        fs.list.local_count = 0;
        fs.view.pending_select_path = Some((old_folder, old_scroll));
        fs.git.git_cache_until = None;
        push_history(fs, new_path);
    }
}

/// Open the path input dialog, pre-filled with the current directory.
pub fn open_path_input(app: &mut App) {
    let value = app.current_file_state()
        .map(|fs| fs.nav.current_path.to_string_lossy().to_string())
        .unwrap_or_default();
    app.core.input.set_value(value);
    app.core.input.cursor_position = app.core.input.value.len();
    app.core.input.style = ratatui::style::Style::default()
        .fg(crate::ui::theme::accent_secondary())
        .add_modifier(ratatui::style::Modifier::BOLD);
    app.core.input.cursor_style = ratatui::style::Style::default()
        .bg(crate::ui::theme::accent_secondary())
        .fg(ratatui::style::Color::Black);
    app.core.mode = AppMode::PathInput;
}

/// Submit the path input dialog: resolve and navigate to the typed path.
pub fn submit_path_input(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> Result<(), String> {
    let t0 = std::time::Instant::now();
    let input = app.core.input.value.trim().to_string();
    if input.is_empty() {
        return Err("Path is empty".to_string());
    }

    let focused = app.focused_pane_index;
    let Some(fs) = app.current_file_state_mut() else {
        return Err("No active file pane".to_string());
    };

    let remote = fs.nav.remote_session.is_some();
    let target = resolve_path_input(&input, &fs.nav.current_path, remote);

    fs.nav.current_path = target.clone();
    fs.view.pending_select_path = None;
    fs.list.selection.clear();
    fs.nav.search_filter.clear();
    fs.git.git_cache_until = None;
    *fs.view.table_state.offset_mut() = 0;
    push_history(fs, target);

    let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(focused));
    crate::app::log_debug(&format!("submit_path_input: {:?}", t0.elapsed()));
    Ok(())
}

/// Resolve a user-typed path string to an absolute PathBuf.
/// Handles empty/whitespace, ~ expansion, absolute paths, and relative paths.
fn resolve_path_input(input: &str, current_path: &std::path::Path, remote: bool) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return current_path.to_path_buf();
    }

    if !remote && trimmed == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }

    if !remote {
        if let Some(rest) = trimmed.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        }
    }

    let typed = PathBuf::from(trimmed);
    if typed.is_absolute() {
        if !remote {
            std::fs::canonicalize(&typed).unwrap_or(typed)
        } else {
            typed
        }
    } else {
        let joined = current_path.join(&typed);
        if !remote {
            std::fs::canonicalize(&joined).unwrap_or(joined)
        } else {
            joined
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fs(path: &str) -> FileState {
        FileState::new(
            PathBuf::from(path),
            None,
            false,
            vec![crate::state::FileColumn::Name],
            crate::state::FileColumn::Name,
            true,
        )
    }

    // ── resolve_path_input ──────────────────────────────────────

    #[test]
    fn resolve_empty_returns_current() {
        let current = std::path::PathBuf::from("/home/user");
        let result = resolve_path_input("", &current, false);
        assert_eq!(result, current);
    }

    #[test]
    fn resolve_whitespace_returns_current() {
        let current = std::path::PathBuf::from("/home/user");
        let result = resolve_path_input("   ", &current, false);
        assert_eq!(result, current);
    }

    #[test]
    fn resolve_absolute_path() {
        let current = std::path::PathBuf::from("/home/user");
        let result = resolve_path_input("/etc/config", &current, false);
        assert_eq!(result, std::path::PathBuf::from("/etc/config"));
    }

    #[test]
    fn resolve_relative_path() {
        let current = std::path::PathBuf::from("/home/user");
        let result = resolve_path_input("projects/tiles", &current, false);
        assert_eq!(result, std::path::PathBuf::from("/home/user/projects/tiles"));
    }

    #[test]
    fn resolve_absolute_ignores_remote_flag() {
        let current = std::path::PathBuf::from("/home/user");
        let result = resolve_path_input("/var/log", &current, true);
        assert_eq!(result, std::path::PathBuf::from("/var/log"));
    }

    #[test]
    fn resolve_path_tilde_expands_home() {
        let current = std::path::PathBuf::from("/tmp");
        let result = resolve_path_input("~", &current, false);
        if dirs::home_dir().is_some() {
            assert_ne!(result, std::path::PathBuf::from("/tmp"));
        }
    }

    #[test]
    fn resolve_path_tilde_slash_relative() {
        let current = std::path::PathBuf::from("/tmp");
        let result = resolve_path_input("~/Documents", &current, false);
        if let Some(home) = dirs::home_dir() {
            assert_eq!(result, home.join("Documents"));
        }
    }

    // ── push_history ────────────────────────────────────────────

    #[test]
    fn push_history_basic() {
        let mut fs = make_fs("/home");
        push_history(&mut fs, std::path::PathBuf::from("/home"));
        assert_eq!(fs.nav.history.len(), 1);
        assert_eq!(fs.nav.history_index, 0);
    }

    #[test]
    fn push_history_deduplicates_consecutive() {
        let mut fs = make_fs("/home");
        push_history(&mut fs, std::path::PathBuf::from("/home"));
        assert_eq!(fs.nav.history.len(), 1);
    }

    #[test]
    fn push_history_allows_different_paths() {
        let mut fs = make_fs("/home");
        push_history(&mut fs, std::path::PathBuf::from("/etc"));
        push_history(&mut fs, std::path::PathBuf::from("/var"));
        assert_eq!(fs.nav.history.len(), 3);
        assert_eq!(fs.nav.history_index, 2);
    }

    #[test]
    fn push_history_caps_at_50() {
        let mut fs = make_fs("/");
        for i in 0..60 {
            push_history(&mut fs, std::path::PathBuf::from(format!("/dir{}", i)));
        }
        assert_eq!(fs.nav.history.len(), 50);
        assert_eq!(fs.nav.history.last().unwrap(), &std::path::PathBuf::from("/dir59"));
    }

    #[test]
    fn push_history_index_stays_valid_after_cap() {
        let mut fs = make_fs("/");
        for i in 0..55 {
            push_history(&mut fs, std::path::PathBuf::from(format!("/dir{}", i)));
        }
        assert!(fs.nav.history_index < fs.nav.history.len());
    }

    #[test]
    fn push_history_truncates_future_on_new_entry() {
        let mut fs = make_fs("/");
        push_history(&mut fs, std::path::PathBuf::from("/a"));
        push_history(&mut fs, std::path::PathBuf::from("/b"));
        push_history(&mut fs, std::path::PathBuf::from("/c"));
        fs.nav.history_index = 2;
        push_history(&mut fs, std::path::PathBuf::from("/d"));
        assert_eq!(fs.nav.history.len(), 4);
        assert_eq!(fs.nav.history[3], std::path::PathBuf::from("/d"));
    }

    #[test]
    fn push_history_empty_fs() {
        let fs = make_fs("/root");
        assert_eq!(fs.nav.history.len(), 1);
        assert_eq!(fs.nav.history[0], std::path::PathBuf::from("/root"));
    }

    #[test]
    fn push_history_same_path_twice_no_dup() {
        let mut fs = make_fs("/home");
        push_history(&mut fs, std::path::PathBuf::from("/home"));
        assert_eq!(fs.nav.history.len(), 1);
    }

    #[test]
    fn push_history_alternating_paths() {
        let mut fs = make_fs("/a");
        push_history(&mut fs, std::path::PathBuf::from("/b"));
        push_history(&mut fs, std::path::PathBuf::from("/a"));
        push_history(&mut fs, std::path::PathBuf::from("/b"));
        assert_eq!(fs.nav.history.len(), 4);
    }

    // ── navigate_back / navigate_forward ────────────────────────

    #[test]
    fn navigate_back_decrements_index() {
        let mut fs = make_fs("/a");
        push_history(&mut fs, std::path::PathBuf::from("/b"));
        push_history(&mut fs, std::path::PathBuf::from("/c"));
        assert_eq!(fs.nav.history_index, 2);
        fs.nav.history_index = 1;
        fs.nav.current_path = fs.nav.history[1].clone();
        assert_eq!(fs.nav.current_path, std::path::PathBuf::from("/b"));
    }

    #[test]
    fn navigate_forward_increments_index() {
        let mut fs = make_fs("/a");
        push_history(&mut fs, std::path::PathBuf::from("/b"));
        push_history(&mut fs, std::path::PathBuf::from("/c"));
        fs.nav.history_index = 1;
        fs.nav.history_index = 2;
        fs.nav.current_path = fs.nav.history[2].clone();
        assert_eq!(fs.nav.current_path, std::path::PathBuf::from("/c"));
    }
}
