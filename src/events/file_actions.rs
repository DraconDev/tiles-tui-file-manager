//! File action handlers — extracted from file_manager.rs.
//!
//! Handles keyboard shortcuts for file operations: space (toggle expand),
//! enter (open/navigate), rename, trash, permanent delete, quick copy.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::app::{App, AppEvent, CurrentView};
use crate::event_helpers;

/// Handle Space key: toggle tree expand/collapse in sidebar, or preview in file list.
pub fn handle_space_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    // If sidebar is focused and selected item is a folder, toggle expand/collapse
    if app.sidebar.sidebar_focus {
        if let Some(bound) = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.idx == app.sidebar.sidebar_selection)
        {
            let path = bound.path.clone();
            if app.layout.expanded_folders.contains(&path) {
                app.layout.expanded_folders.retain(|p| p != &path);
            } else if path.is_dir() || app.sidebar.sidebar_favorites.iter().any(|f| f == &path) {
                app.layout.expanded_folders.push(path);
            }
        }
        return;
    }

    // File list space: preview selected file
    let (selected_path, pane_idx) = {
        if let Some(fs) = app.current_file_state() {
            let idx = match fs.list.selection.selected {
                Some(i) => i,
                None => return,
            };
            match fs.list.files.get(idx).cloned() {
                Some(p) => (p, app.focused_pane_index),
                None => return,
            }
        } else {
            return;
        }
    };

    let _ = crate::app::try_send_event(event_tx, AppEvent::PreviewRequested(pane_idx, selected_path));
}

/// Handle Enter key: navigate into folder or open file.
pub fn handle_enter_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    // Sidebar enter: navigate to folder (Dolphin-style, no auto-expand)
    if app.sidebar.sidebar_focus {
        if let Some(bound) = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.idx == app.sidebar.sidebar_selection)
        {
            if bound.path.is_dir() {
                let path = bound.path.clone();
                if let Some(fs) = app.current_file_state_mut() {
                    fs.nav.current_path = path.clone();
                    fs.list.files.clear();
                    fs.list.tree_file_depths.clear();
                    fs.list.metadata.clear();
                    fs.list.local_count = 0;
                    crate::event_helpers::push_history(fs, path);
                }
                let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
            }
        }
        return;
    }

    // File list enter: navigate or open
    if let Some(fs) = app.current_file_state() {
        let idx = match fs.list.selection.selected {
            Some(i) => i,
            None => return,
        };
        if let Some(path) = fs.list.files.get(idx).cloned() {
            if path.is_dir() {
                // Navigate into directory
                let mut app_guard = app;
                if let Some(fs) = app_guard.current_file_state_mut() {
                    fs.nav.current_path = path.clone();
                    fs.list.files.clear();
                    fs.list.tree_file_depths.clear();
                    fs.list.metadata.clear();
                    fs.list.local_count = 0;
                    crate::event_helpers::push_history(fs, path);
                }
                let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app_guard.focused_pane_index));
            } else {
                // Open file with default application
                let _ = dracon_terminal_engine::utils::spawn_detached(
                    "xdg-open",
                    vec![path.to_string_lossy().to_string()],
                );
            }
        }
    }
}

/// Handle Rename shortcut (F2 or Ctrl+R).
pub fn handle_rename_shortcut(app: &mut App) {
    if let Some(fs) = app.current_file_state() {
        let idx = match fs.list.selection.selected {
            Some(i) => i,
            None => return,
        };
        if let Some(path) = fs.list.files.get(idx).cloned() {
            app.core.rename_target = Some(path);
            app.core.input.value = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            app.core.input.cursor = app.core.input.value.len();
            app.core.app_mode = crate::app::AppMode::Rename;
        }
    }
}

/// Handle Trash shortcut (Delete key): move to trash.
pub fn handle_trash_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if let Some(fs) = app.current_file_state() {
        let idx = match fs.list.selection.selected {
            Some(i) => i,
            None => return,
        };
        if let Some(path) = fs.list.files.get(idx).cloned() {
            let _ = crate::app::try_send_event(event_tx, AppEvent::TrashFile(path));
        }
    }
}

/// Handle permanent delete (Shift+Delete).
pub fn handle_permanent_delete_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if let Some(fs) = app.current_file_state() {
        let idx = match fs.list.selection.selected {
            Some(i) => i,
            None => return,
        };
        if let Some(path) = fs.list.files.get(idx).cloned() {
            let _ = crate::app::try_send_event(event_tx, AppEvent::Delete(path));
        }
    }
}

/// Handle quick copy (Ctrl+C on file): copy selected file path to clipboard.
pub fn handle_quick_copy(app: &mut App, event_tx: &mpsc::Sender<AppEvent>, _to_left: bool) {
    if app.core.current_view == CurrentView::Git || app.core.current_view == CurrentView::Commit {
        return;
    }
    if app.sidebar.sidebar_focus {
        // Copy sidebar path
        if let Some(bound) = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.idx == app.sidebar.sidebar_selection)
        {
            let path_str = bound.path.to_string_lossy().to_string();
            let _ = crate::clipboard::copy_text_to_clipboard(&path_str);
            let _ = crate::app::try_send_event(
                event_tx,
                AppEvent::StatusMsg(format!("Copied: {}", path_str)),
            );
        }
        return;
    }

    if let Some(fs) = app.current_file_state() {
        let idx = match fs.list.selection.selected {
            Some(i) => i,
            None => return,
        };
        if let Some(path) = fs.list.files.get(idx) {
            let path_str = path.to_string_lossy().to_string();
            let _ = crate::clipboard::copy_text_to_clipboard(&path_str);
            let _ = crate::app::try_send_event(
                event_tx,
                AppEvent::StatusMsg(format!("Copied: {}", path_str)),
            );
        }
    }
}
