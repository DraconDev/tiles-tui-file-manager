//! File action handlers — extracted from file_manager.rs.
//!
//! Handles keyboard shortcuts for file operations: space (toggle expand/preview),
//! enter (navigate/open), rename, trash, permanent delete, quick copy.

use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use crate::app::{App, AppEvent, AppMode, CurrentView, SidebarTarget};

/// Double-click time threshold in milliseconds.
pub const DOUBLE_CLICK_MS: u64 = 500;

/// Check if two clicks at (last_x, last_y) and (column, row) within DOUBLE_CLICK_MS
/// constitute a double-click (position within 1px, time within threshold).
pub fn is_double_click(
    last_click_pos: (u16, u16),
    last_click_time: std::time::Instant,
    column: u16,
    row: u16,
) -> bool {
    let (last_x, last_y) = last_click_pos;
    let close_enough = last_x.abs_diff(column) <= 1 && last_y.abs_diff(row) <= 1;
    close_enough && last_click_time.elapsed() < std::time::Duration::from_millis(DOUBLE_CLICK_MS)
}

/// Check if a character is valid for search input (not a control or special char).
pub fn is_valid_search_char(c: char) -> bool {
    let bad = (c as u32) < 32 || c == '\x7f' || c == '\x1b'
        || matches!(c, '[' | ']' | '~' | '^' | '_' | '=' | '+' | '<' | '>' | '*' | '?' | '!' | '$'
            | '%' | '&' | '@' | '#' | '{' | '}' | '\\' | '|' | '`');
    !bad
}

/// Handle Space key: toggle tree expand/collapse in sidebar, or preview in file list.
pub fn handle_space_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    // If sidebar is focused and selected item is a folder, toggle expand/collapse
    if app.sidebar.sidebar_focus {
        if let Some(bound) = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.index == app.sidebar.sidebar_index)
        {
            if let SidebarTarget::Project(path) = &bound.target {
                if path.is_dir() {
                    if app.sidebar.tree_expanded_folders.contains(path) {
                        app.sidebar.tree_expanded_folders.remove(path);
                    } else {
                        app.sidebar.tree_expanded_folders.insert(path.clone());
                    }
                    return;
                }
            }
        }
    }

    if let Some(fs) = app.current_file_state_mut() {
        if fs.list.selection.selected.is_none() && !fs.list.files.is_empty() {
            fs.list.selection.selected = Some(0);
            fs.view.table_state.select(Some(0));
            fs.list.selection.anchor = Some(0);
        }

        if let Some(idx) = fs.list.selection.selected {
            if let Some(path) = fs.list.files.get(idx).cloned() {
                if is_virtual_divider(&path) {
                    return;
                }
                if path.is_dir() {
                    let was_expanded = app.layout.expanded_folders.contains(&path);
                    if was_expanded {
                        app.layout.expanded_folders.remove(&path);
                    } else {
                        app.layout.expanded_folders.insert(path.clone());
                    }
                    let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                } else {
                    let (is_binary, _, _) = crate::modules::files::check_file_suitability(&path, u64::MAX);
                    if is_binary {
                        return;
                    }
                    let target_pane = app.focused_pane_index
                        .min(app.panes.len().saturating_sub(1));
                    let _ = crate::app::try_send_event(event_tx, AppEvent::PreviewRequested(target_pane, path));
                    app.save_current_view_prefs();
                    app.core.current_view = CurrentView::Editor;
                    app.load_view_prefs(CurrentView::Editor);
                    app.sidebar.show_sidebar = true;
                    if app.panes.len() == 1 {
                        app.focused_pane_index = 0;
                    } else {
                        app.focused_pane_index = target_pane;
                    }
                    app.sidebar.sidebar_focus = false;
                }
            }
        }
    }
}

/// Handle Enter key: navigate into folder or open file.
pub fn handle_enter_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if app.sidebar.sidebar_focus {
        let target_opt = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.index == app.sidebar.sidebar_index)
            .map(|b| b.target.clone());
        if let Some(target) = target_opt {
            match target {
                SidebarTarget::Favorite(path) | SidebarTarget::Recent(path) => {
                    if let Some(fs) = app.current_file_state_mut() {
                        fs.nav.remote_session = None;
                        fs.nav.current_path = path.clone();
                        fs.list.files.clear();
                        fs.list.tree_file_depths.clear();
                        fs.list.metadata.clear();
                        fs.list.local_count = 0;
                        fs.list.selection.selected = Some(0);
                        fs.list.selection.anchor = Some(0);
                        fs.list.selection.clear_multi();
                        crate::nav_helpers::push_history(fs, path.clone());
                        let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        app.sidebar.sidebar_focus = false;
                    }
                }
                SidebarTarget::Remote(idx) => {
                    let _ =
                        crate::app::try_send_event(event_tx, AppEvent::ConnectToRemote(app.focused_pane_index, idx));
                }
                SidebarTarget::Project(path) => {
                    if path.is_dir() {
                        // Enter on folder = navigate only (Dolphin-style, no auto-expand)
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.nav.remote_session = None;
                            fs.nav.current_path = path.clone();
                            fs.list.files.clear();
                            fs.list.tree_file_depths.clear();
                            fs.list.metadata.clear();
                            fs.list.local_count = 0;
                            fs.list.selection.selected = Some(0);
                            fs.list.selection.anchor = Some(0);
                            fs.list.selection.clear_multi();
                            crate::nav_helpers::push_history(fs, path.clone());
                            let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        }
                        app.sidebar.sidebar_focus = false;
                    } else {
                        let _ = crate::app::try_send_event(event_tx, AppEvent::PreviewRequested(app.focused_pane_index, path));
                        app.sidebar.sidebar_focus = false;
                    }
                }
                _ => {}
            }
        }
        return;
    }

    let mut navigate_to = None;
    if let Some(fs) = app.current_file_state() {
        if let Some(idx) = fs.list.selection.selected {
            if let Some(path) = fs.list.files.get(idx) {
                if is_virtual_divider(path) {
                    return;
                }
                if let Some(nav) = open_file_or_navigate(path) {
                    navigate_to = Some(nav);
                }
            }
        }
    }
    if let Some(p) = navigate_to {
        let restore = app.selection.folder_selections.get(&p).copied();

        if let Some(fs) = app.current_file_state() {
            let path = fs.nav.current_path.clone();
            let idx = fs.list.selection.selected.unwrap_or(0);
            let scroll = fs.view.table_state.offset();
            app.selection.folder_selections.insert(path, (idx, scroll));
        }

        if let Some(fs) = app.current_file_state_mut() {
            fs.nav.current_path = p.clone();
            fs.list.files.clear();
            fs.list.tree_file_depths.clear();
            fs.list.metadata.clear();
            fs.list.local_count = 0;
            if let Some((restore_sel, restore_scroll)) = restore {
                fs.list.selection.selected = Some(restore_sel);
                fs.list.selection.anchor = Some(restore_sel);
                *fs.view.table_state.offset_mut() = restore_scroll;
            } else {
                fs.list.selection.selected = Some(0);
                fs.list.selection.anchor = Some(0);
                *fs.view.table_state.offset_mut() = 0;
            }
            fs.list.selection.clear_multi();
            fs.nav.search_filter.clear();
            fs.nav.search_generation += 1;
            crate::nav_helpers::push_history(fs, p);
            app.layout.expanded_folders.clear();
            let _ = crate::app::try_send_event(event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
        }
    }
}

/// Handle Rename shortcut (F2 or Ctrl+R).
pub fn handle_rename_shortcut(app: &mut App) {
    let mut to_rename = None;
    if let Some(fs) = app.current_file_state() {
        if let Some(p) = fs.list.selection.selected.and_then(|idx| fs.list.files.get(idx)) {
            to_rename = Some(
                p.file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("root"))
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    if let Some(name) = to_rename {
        app.core.mode = AppMode::Rename;
        app.core.input.set_value(name.clone());
        if let Some(idx) = name.rfind('.') {
            app.core.input.cursor_position = if idx > 0 { idx } else { name.len() };
        } else {
            app.core.input.cursor_position = name.len();
        }
        app.selection.rename_selected = true;
    }
}

/// Handle Trash shortcut (Delete key): move to trash.
pub fn handle_trash_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if let Some(fs) = app.current_file_state() {
        if fs.list.selection.selected.is_some() {
            if app.settings.confirm_delete {
                app.core.mode = AppMode::Delete("trash".to_string());
            } else {
                let mut paths = Vec::new();
                if !fs.list.selection.is_empty() {
                    for &idx in fs.list.selection.multi_selected_indices() {
                        if let Some(p) = fs.list.files.get(idx) {
                            paths.push(p.clone());
                        }
                    }
                } else if let Some(idx) = fs.list.selection.selected {
                    if let Some(p) = fs.list.files.get(idx) {
                        paths.push(p.clone());
                    }
                }
                for p in paths {
                    let _ = crate::app::try_send_event(event_tx, AppEvent::TrashFile(p));
                }
            }
        }
    }
}

/// Handle permanent delete (Shift+Delete).
pub fn handle_permanent_delete_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if let Some(fs) = app.current_file_state() {
        if fs.list.selection.selected.is_some() {
            if app.settings.confirm_delete {
                app.core.mode = AppMode::Delete("permanent".to_string());
            } else {
                let mut paths = Vec::new();
                if !fs.list.selection.is_empty() {
                    for &idx in fs.list.selection.multi_selected_indices() {
                        if let Some(p) = fs.list.files.get(idx) {
                            paths.push(p.clone());
                        }
                    }
                } else if let Some(idx) = fs.list.selection.selected {
                    if let Some(p) = fs.list.files.get(idx) {
                        paths.push(p.clone());
                    }
                }
                for p in paths {
                    let _ = crate::app::try_send_event(event_tx, AppEvent::Delete(p));
                }
            }
        }
    }
}

/// Handle quick copy (Ctrl+C on file): copy selected file path to clipboard.
pub fn handle_quick_copy(app: &mut App, event_tx: &mpsc::Sender<AppEvent>, _to_left: bool) {
    let other_pane_idx = if app.focused_pane_index == 0 { 1 } else { 0 };
    if let Some(dest_path) = app.panes
        .get(other_pane_idx)
        .and_then(|p| p.current_state())
        .map(|fs| fs.nav.current_path.clone())
    {
        if let Some(fs) = app.current_file_state() {
            let mut paths = Vec::new();
            if !fs.list.selection.is_empty() {
                for &idx in fs.list.selection.multi_selected_indices() {
                    if let Some(p) = fs.list.files.get(idx) {
                        paths.push(p.clone());
                    }
                }
            } else if let Some(idx) = fs.list.selection.selected {
                if let Some(p) = fs.list.files.get(idx) {
                    paths.push(p.clone());
                }
            }
            for p in paths {
                let dest = path_join(
                    &dest_path,
                    p.file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("root")),
                );
                let _ = crate::app::try_send_event(event_tx, AppEvent::Copy(p, dest));
            }
        }
    }
}

// ── Helper functions (also extracted from file_manager.rs) ──

/// Check if a path is a virtual divider (separator row).
pub fn is_virtual_divider(path: &Path) -> bool {
    path.to_string_lossy() == "__DIVIDER__"
}

pub fn open_file_or_navigate(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        dracon_terminal_engine::utils::spawn_detached(
            "xdg-open",
            vec![path.to_string_lossy().to_string()],
        );
        None
    }
}

fn path_join(base: &Path, name: &std::ffi::OsStr) -> PathBuf {
    base.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn is_virtual_divider_exact_match() {
        assert!(is_virtual_divider(Path::new("__DIVIDER__")));
    }

    #[test]
    fn is_virtual_divider_prefix_not_match() {
        assert!(!is_virtual_divider(Path::new("__DIVIDER")));
        assert!(!is_virtual_divider(Path::new("__DIVIDER__suffix")));
    }

    #[test]
    fn is_virtual_divider_normal_path_false() {
        assert!(!is_virtual_divider(Path::new("real_file.txt")));
        assert!(!is_virtual_divider(Path::new("/home/user")));
    }

    #[test]
    fn path_join_absolute_base() {
        let base = Path::new("/home/user");
        let name = std::ffi::OsStr::new("doc.txt");
        assert_eq!(path_join(base, name), PathBuf::from("/home/user/doc.txt"));
    }

    #[test]
    fn path_join_nested() {
        let base = Path::new("/tmp");
        let name = std::ffi::OsStr::new("subdir/file.rs");
        assert_eq!(path_join(base, name), PathBuf::from("/tmp/subdir/file.rs"));
    }

    #[test]
    fn path_join_root_base() {
        let base = Path::new("/");
        let name = std::ffi::OsStr::new("etc");
        assert_eq!(path_join(base, name), PathBuf::from("/etc"));
    }
}
