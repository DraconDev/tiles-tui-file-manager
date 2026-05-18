#![allow(clippy::needless_borrow)]

use dracon_terminal_engine::contracts::{
    InputEvent as Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app::{
    App, AppEvent, AppMode, ContextMenuTarget, CurrentView, FileColumn, SidebarTarget, UndoAction,
};
use crate::events::input::delete_word_backwards;
use crate::state::{DropTarget};

const DOUBLE_CLICK_MS: u64 = 500;
const SEARCH_DEBOUNCE_MS: u64 = 300;

fn is_valid_search_char(c: char) -> bool {
    let bad = (c as u32) < 32 || c == '\x7f' || c == '\x1b'
        || matches!(c, '[' | ']' | '~' | '^' | '_' | '=' | '+' | '<' | '>' | '*' | '?' | '!' | '$'
            | '%' | '&' | '@' | '#' | '{' | '}' | '\\' | '|' | '`');
    !bad
}

fn reselect_after_filter(fs: &mut crate::state::FileState, old_path: Option<&std::path::Path>) {
    if let Some(path) = old_path {
        if let Some(new_idx) = fs.list.files.iter().position(|p| p == path) {
            fs.list.selection.selected = Some(new_idx);
            fs.list.selection.anchor = Some(new_idx);
            fs.view.table_state.select(Some(new_idx));
            let capacity = fs.view.view_height.saturating_sub(3).max(1);
            let offset = fs.view.table_state.offset();
            if new_idx < offset {
                *fs.view.table_state.offset_mut() = new_idx;
            } else if new_idx >= offset + capacity {
                *fs.view.table_state.offset_mut() = new_idx.saturating_sub(capacity - 1);
            }
            return;
        }
    }
    fs.list.selection.selected = Some(0);
    fs.list.selection.anchor = Some(0);
    fs.view.table_state.select(Some(0));
    *fs.view.table_state.offset_mut() = 0;
}

fn is_double_click(
    last_click_pos: (u16, u16),
    last_click_time: std::time::Instant,
    column: u16,
    row: u16,
) -> bool {
    let (last_x, last_y) = last_click_pos;
    let close_enough = last_x.abs_diff(column) <= 1 && last_y.abs_diff(row) <= 1;
    close_enough && last_click_time.elapsed() < Duration::from_millis(DOUBLE_CLICK_MS)
}

fn is_virtual_divider(path: &std::path::Path) -> bool {
    path.to_string_lossy() == "__DIVIDER__"
}

fn open_file_or_navigate(path: &std::path::Path) -> Option<std::path::PathBuf> {
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

fn execute_undo(
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> Option<&'static str> {
    let action = app.undo_state.undo_stack.pop()?;
    let active_remote = app.current_file_state()
        .and_then(|fs| fs.nav.remote_session.clone());
    let success;
    let action_name = match &action {
        UndoAction::Rename(old, new) | UndoAction::Move(old, new) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::rename(remote, new, old).is_ok()
            } else {
                std::fs::rename(new, old).is_ok()
            };
            "move/rename"
        }
        UndoAction::Copy(_src, dest) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::remove_path(remote, dest).is_ok()
            } else if dest.is_dir() {
                std::fs::remove_dir_all(dest).is_ok()
            } else {
                std::fs::remove_file(dest).is_ok()
            };
            "copy"
        }
        UndoAction::Delete(path) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::remove_path(remote, path).is_ok()
            } else {
                std::fs::remove_file(path).is_ok()
            };
            "delete"
        }
    };
    if success {
        app.undo_state.redo_stack.push(action);
    }
    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(if success {
        format!("Undo OK ({})", action_name)
    } else {
        format!("Undo failed ({})", action_name)
    }));
    for pane_idx in 0..app.panes.len() {
        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
    }
    Some(action_name)
}

fn execute_redo(
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> Option<&'static str> {
    let action = app.undo_state.redo_stack.pop()?;
    let active_remote = app.current_file_state()
        .and_then(|fs| fs.nav.remote_session.clone());
    let success;
    let action_name = match &action {
        UndoAction::Rename(old, new) | UndoAction::Move(old, new) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::rename(remote, old, new).is_ok()
            } else {
                std::fs::rename(old, new).is_ok()
            };
            "move/rename"
        }
        UndoAction::Copy(src, dest) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::copy_recursive(remote, src, dest).is_ok()
            } else {
                crate::modules::files::copy_recursive(src, dest).is_ok()
            };
            "copy"
        }
        UndoAction::Delete(path) => {
            success = if let Some(remote) = &active_remote {
                crate::modules::remote::remove_path(remote, path).is_ok()
            } else {
                std::fs::remove_file(path).is_ok()
            };
            "delete"
        }
    };
    if success {
        app.undo_state.undo_stack.push(action);
    }
    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(if success {
        format!("Redo OK ({})", action_name)
    } else {
        format!("Redo failed ({})", action_name)
    }));
    for pane_idx in 0..app.panes.len() {
        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
    }
    Some(action_name)
}

fn handle_global_shortcuts(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> Option<bool> {
    let has_control = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char('i') | KeyCode::Char('I') if has_control => {
            app.core.mode = AppMode::Properties;
            Some(true)
        }
        KeyCode::Enter if has_alt => {
            app.core.mode = AppMode::Properties;
            Some(true)
        }
        KeyCode::Char('h') | KeyCode::Char('H') if has_control => {
            let idx = app.toggle_hidden();
            if let Some(fs) = app.panes.get(idx).and_then(|p| p.current_state()) {
                app.settings.default_show_hidden = fs.nav.show_hidden;
            }
            crate::config::save_state_quiet(app);
            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(idx));
            Some(true)
        }
        KeyCode::Backspace if has_control => {
            let idx = app.toggle_hidden();
            if let Some(fs) = app.panes.get(idx).and_then(|p| p.current_state()) {
                app.settings.default_show_hidden = fs.nav.show_hidden;
            }
            crate::config::save_state_quiet(app);
            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(idx));
            Some(true)
        }
        KeyCode::Char('g') | KeyCode::Char('G') if has_control => {
            app.core.mode = AppMode::Settings;
            app.settings.settings_scroll = 0;
            Some(true)
        }
        KeyCode::Char('n') | KeyCode::Char('N') if has_control => {
            if let Some(fs) = app.current_file_state() {
                let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                    path: fs.nav.current_path.clone(),
                    new_tab: true,
                    remote: fs.nav.remote_session.clone(),
                    command: None,
                });
            }
            Some(true)
        }
        KeyCode::Char('k') | KeyCode::Char('K') if has_control => {
            if let Some(fs) = app.current_file_state() {
                let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                    path: fs.nav.current_path.clone(),
                    new_tab: false,
                    remote: fs.nav.remote_session.clone(),
                    command: None,
                });
            }
            Some(true)
        }
        KeyCode::Char('t') | KeyCode::Char('T') if has_control => {
            if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                if let Some(fs) = pane.current_state() {
                    let new_fs = fs.clone();
                    pane.open_tab(new_fs);
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                }
            }
            Some(true)
        }
        // Ctrl+Shift+T — reopen last closed tab
        KeyCode::Char('t') | KeyCode::Char('T')
            if has_control
                && key.modifiers.contains(KeyModifiers::SHIFT)
                && app.core.current_view == CurrentView::Files =>
        {
            if let Some(closed) = app.nav.closed_tabs.pop_back() {
                if let Some(pane) = app.panes.get_mut(closed.pane_index) {
                    if pane.tabs.len() < crate::config::MAX_TABS {
                        let new_fs = crate::state::FileState::new(
                            closed.path.clone(),
                            None,
                            false,
                            pane.current_state()
                                .map(|fs| fs.list.columns.clone())
                                .unwrap_or_else(|| vec![crate::state::FileColumn::Name]),
                            pane.current_state()
                                .map(|fs| fs.nav.sort_column)
                                .unwrap_or(crate::state::FileColumn::Name),
                            pane.current_state()
                                .map(|fs| fs.nav.sort_ascending)
                                .unwrap_or(true),
                        );
                        pane.open_tab(new_fs);
                        app.focused_pane_index = closed.pane_index;
                        let _ = crate::app::try_send_event(
                            &event_tx,
                            AppEvent::RefreshFiles(closed.pane_index),
                        );
                        let _ = crate::app::try_send_event(
                            &event_tx,
                            AppEvent::StatusMsg(format!(
                                "Restored: {}",
                                closed.path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default()
                            )),
                        );
                    }
                }
            }
            Some(true)
        }
        KeyCode::Char('w') if has_control && app.core.current_view == CurrentView::Files => {
            let pane_idx = app.focused_pane_index;
            if let Some(pane) = app.panes.get_mut(pane_idx) {
                if pane.tabs.len() > 1 {
                    // Save tab info for undo before removing
                    if let Some(removed) = pane.tabs.get(pane.active_tab_index) {
                        if app.nav.closed_tabs.len() >= 10 {
                            app.nav.closed_tabs.pop_front();
                        }
                        app.nav.closed_tabs.push_back(crate::state::ClosedTab {
                            path: removed.nav.current_path.clone(),
                            pane_index: pane_idx,
                        });
                    }
                    let removed = pane.tabs.remove(pane.active_tab_index);
                    if pane.active_tab_index >= pane.tabs.len() {
                        pane.active_tab_index = pane.tabs.len() - 1;
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                        "Closed: {}",
                        removed.nav.current_path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    )));
                } else {
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg("Cannot close last tab".to_string()));
                }
            }
            Some(true)
        }
        KeyCode::Left if has_alt => {
            app.resize_sidebar(-2);
            Some(true)
        }
        KeyCode::Right if has_alt => {
            app.resize_sidebar(2);
            Some(true)
        }
        KeyCode::Char(' ') if has_control => {
            app.core.input.clear();
            app.core.mode = AppMode::CommandPalette;
            crate::event_helpers::update_commands(app);
            Some(true)
        }
        KeyCode::Left if has_control => {
            if app.sidebar.sidebar_focus {
                app.resize_sidebar(-2);
            } else {
                app.move_to_other_pane();
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(0));
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(1));
            }
            Some(true)
        }
        KeyCode::Right if has_control => {
            if app.sidebar.sidebar_focus {
                app.resize_sidebar(2);
            } else {
                app.move_to_other_pane();
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(0));
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(1));
            }
            Some(true)
        }
        _ => None,
    }
}

fn handle_clipboard_and_undo(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> Option<bool> {
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Char('c') => {
            if let Some(fs) = app.current_file_state() {
                if let Some(idx) = fs.list.selection.selected {
                    if let Some(path) = fs.list.files.get(idx) {
                        app.selection.clipboard = Some((path.clone(), crate::app::ClipboardOp::Copy));
                    }
                }
            }
            Some(true)
        }
        KeyCode::Char('x') => {
            if let Some(fs) = app.current_file_state() {
                if let Some(idx) = fs.list.selection.selected {
                    if let Some(path) = fs.list.files.get(idx) {
                        app.selection.clipboard = Some((path.clone(), crate::app::ClipboardOp::Cut));
                    }
                }
            }
            Some(true)
        }
        KeyCode::Char('v') => {
            if let Some((src, op)) = app.selection.clipboard.clone() {
                if let Some(fs) = app.current_file_state() {
                    let dest = fs.nav.current_path.join(
                        src.file_name().unwrap_or_else(|| std::ffi::OsStr::new("root")),
                    );
                    match op {
                        crate::app::ClipboardOp::Copy => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(src, dest));
                        }
                        crate::app::ClipboardOp::Cut => {
                            let result = crate::app::try_send_event(&event_tx, AppEvent::Rename(src, dest));
                            if result {
                                app.selection.clipboard = None;
                            }
                        }
                    }
                }
            }
            Some(true)
        }
        KeyCode::Char('a') => {
            if let Some(fs) = app.current_file_state_mut() {
                fs.list.selection.select_all(fs.list.files.len());
            }
            Some(true)
        }
        KeyCode::Char('z') if !has_shift => {
            if execute_undo(app, event_tx).is_none() {
                if app.in_soft_shield() {
                    return Some(true);
                }
                if let Some(fs) = app.current_file_state_mut() {
                    if !fs.nav.search_filter.is_empty() {
                        fs.nav.search_filter.clear();
                        fs.nav.search_generation += 1;
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                }
            }
            Some(true)
        }
        KeyCode::Char('y') | KeyCode::Char('Z') if has_shift => {
            execute_redo(app, event_tx);
            Some(true)
        }
        KeyCode::Char('Z') if !has_shift => {
            execute_redo(app, event_tx);
            Some(true)
        }
        _ => None,
    }
}

pub fn handle_file_events(evt: &Event, app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> bool {
    if let Event::Key(key) = evt {
        let has_control = key.modifiers.contains(KeyModifiers::CONTROL);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);

        if app.core.mode == AppMode::Normal {
            if let Some(handled) = handle_global_shortcuts(key, app, event_tx) {
                return handled;
            }

            // Standard Navigation
            if key.code == KeyCode::Esc {
                if app.drag.is_dragging {
                    app.drag.is_dragging = false;
                    app.drag.drag_source = None;
                    app.drag.drag_start_pos = None;
                    app.drag.hovered_drop_target = None;
                    return true;
                }
                if app.sidebar.sidebar_focus {
                    app.sidebar.sidebar_focus = false;
                    return true;
                }

                if let Some(fs) = app.current_file_state_mut() {
                    fs.list.selection.clear_multi();
                    fs.list.selection.anchor = None;
                    if !fs.nav.search_filter.is_empty() {
                        fs.nav.search_filter.clear();
                        fs.nav.search_generation += 1;
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                }
                return true;
            }

            match key.code {
                KeyCode::Char('c') | KeyCode::Char('x') | KeyCode::Char('v') | KeyCode::Char('a') | KeyCode::Char('z') | KeyCode::Char('y') | KeyCode::Char('Z')
                    if has_control =>
                {
                    if let Some(handled) = handle_clipboard_and_undo(key, app, event_tx) {
                        return handled;
                    }
                }
                KeyCode::Char('f') if has_control => {
                    app.core.mode = AppMode::Search;
                    return true;
                }
                KeyCode::Insert => {
                    let mut should_save = false;
                    if let Some(fs) = app.current_file_state_mut() {
                        if let Some(idx) = fs.list.selection.selected {
                            fs.list.selection.toggle(idx);
                            should_save = true;
                            // Move down after toggle
                            if idx < fs.list.files.len().saturating_sub(1) {
                                let next_idx = idx + 1;
                                fs.list.selection.selected = Some(next_idx);
                                fs.list.selection.anchor = Some(next_idx);
                                fs.view.table_state.select(Some(next_idx));
                                if next_idx >= fs.view.table_state.offset() + fs.view.view_height {
                                    *fs.view.table_state.offset_mut() =
                                        next_idx.saturating_sub(fs.view.view_height - 1);
                                }
                            }
                        }
                    }
                    if should_save {
                        crate::config::save_state_quiet(app);
                    }
                    return true;
                }
                KeyCode::Char(' ') => {
                    handle_space_key(app, event_tx);
                    return true;
                }
                KeyCode::Char('C') if app.sidebar.sidebar_focus => {
                    app.sidebar.tree_expanded_folders.clear();
                    return true;
                }
                KeyCode::Char('/') => {
                    if let Some(fs) = app.current_file_state_mut() {
                        fs.nav.search_filter.clear();
                        fs.nav.search_generation += 1;
                        fs.list.selection.selected = Some(0);
                        *fs.view.table_state.offset_mut() = 0;
                    }
                    app.sidebar.sidebar_focus = false;
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                KeyCode::Up => {
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    if has_alt && app.sidebar.sidebar_focus {
                        // Reorder Favorites: Find actual starred index from sidebar_bounds
                        if let Some(bound) = app.sidebar.sidebar_bounds
                            .iter()
                            .find(|b| b.index == app.sidebar.sidebar_index)
                        {
                            if let SidebarTarget::Favorite(ref path) = bound.target {
                                if let Some(starred_idx) =
                                    app.nav.starred.iter().position(|p| p == path)
                                {
                                    if starred_idx > 0 {
                                        app.nav.starred.swap(starred_idx, starred_idx - 1);
                                        app.sidebar.sidebar_index = app.sidebar.sidebar_index.saturating_sub(1);
                                        crate::config::save_state_quiet(app);
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(
                                            app.focused_pane_index,
                                        ));
                                    }
                                }
                            }
                        }
                        return true;
                    }
                    if app.sidebar.sidebar_focus && !has_alt {
                        // Navigate sidebar items with Up/Down
                        app.sidebar_move_up();
                        return true;
                    }
                    app.move_up(shift);
                    return true;
                }
                KeyCode::Down => {
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    if has_alt && app.sidebar.sidebar_focus {
                        // Reorder Favorites: Find actual starred index from sidebar_bounds
                        if let Some(bound) = app.sidebar.sidebar_bounds
                            .iter()
                            .find(|b| b.index == app.sidebar.sidebar_index)
                        {
                            if let SidebarTarget::Favorite(ref path) = bound.target {
                                if let Some(starred_idx) =
                                    app.nav.starred.iter().position(|p| p == path)
                                {
                                    if starred_idx < app.nav.starred.len().saturating_sub(1) {
                                        app.nav.starred.swap(starred_idx, starred_idx + 1);
                                        app.sidebar.sidebar_index += 1;
                                        crate::config::save_state_quiet(app);
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(
                                            app.focused_pane_index,
                                        ));
                                    }
                                }
                            }
                        }
                        return true;
                    }
                    if app.sidebar.sidebar_focus && !has_alt {
                        // Navigate sidebar items with Up/Down
                        let max_items = app.sidebar.sidebar_bounds.len();
                        app.sidebar_move_down(max_items);
                        return true;
                    }
                    app.move_down(shift);
                    return true;
                }
                KeyCode::Home => {
                    if let Some(fs) = app.current_file_state_mut() {
                        if !fs.list.files.is_empty() {
                            let mut idx = 0usize;
                            while idx < fs.list.files.len()
                                && fs.list.files[idx].to_string_lossy() == "__DIVIDER__"
                            {
                                idx += 1;
                            }
                            if idx < fs.list.files.len() {
                                fs.list.selection.selected = Some(idx);
                                fs.list.selection.anchor = Some(idx);
                                fs.view.table_state.select(Some(idx));
                                *fs.view.table_state.offset_mut() = idx;
                            }
                        }
                    }
                    return true;
                }
                KeyCode::End => {
                    if let Some(fs) = app.current_file_state_mut() {
                        if !fs.list.files.is_empty() {
                            let mut idx = fs.list.files.len().saturating_sub(1);
                            while idx > 0 && fs.list.files[idx].to_string_lossy() == "__DIVIDER__" {
                                idx = idx.saturating_sub(1);
                            }
                            if fs.list.files[idx].to_string_lossy() != "__DIVIDER__" {
                                fs.list.selection.selected = Some(idx);
                                fs.list.selection.anchor = Some(idx);
                                fs.view.table_state.select(Some(idx));
                                let page = fs.view.view_height.saturating_sub(3).max(1);
                                *fs.view.table_state.offset_mut() = idx.saturating_sub(page - 1);
                            }
                        }
                    }
                    return true;
                }
                KeyCode::PageDown => {
                    if let Some(fs) = app.current_file_state_mut() {
                        if !fs.list.files.is_empty() {
                            let page = fs.view.view_height.saturating_sub(3).max(1);
                            let cur = fs.list.selection.selected.unwrap_or(0);
                            let mut idx = (cur + page).min(fs.list.files.len().saturating_sub(1));
                            while idx + 1 < fs.list.files.len()
                                && fs.list.files[idx].to_string_lossy() == "__DIVIDER__"
                            {
                                idx += 1;
                            }
                            while idx > 0 && fs.list.files[idx].to_string_lossy() == "__DIVIDER__" {
                                idx = idx.saturating_sub(1);
                            }
                            if fs.list.files[idx].to_string_lossy() != "__DIVIDER__" {
                                fs.list.selection.selected = Some(idx);
                                fs.list.selection.anchor = Some(idx);
                                fs.view.table_state.select(Some(idx));
                                if idx >= fs.view.table_state.offset() + page {
                                    *fs.view.table_state.offset_mut() = idx.saturating_sub(page - 1);
                                }
                            }
                        }
                    }
                    return true;
                }
                KeyCode::PageUp => {
                    if let Some(fs) = app.current_file_state_mut() {
                        if !fs.list.files.is_empty() {
                            let page = fs.view.view_height.saturating_sub(3).max(1);
                            let cur = fs.list.selection.selected.unwrap_or(0);
                            let mut idx = cur.saturating_sub(page);
                            while idx > 0 && fs.list.files[idx].to_string_lossy() == "__DIVIDER__" {
                                idx = idx.saturating_sub(1);
                            }
                            if fs.list.files[idx].to_string_lossy() != "__DIVIDER__" {
                                fs.list.selection.selected = Some(idx);
                                fs.list.selection.anchor = Some(idx);
                                fs.view.table_state.select(Some(idx));
                                if idx < fs.view.table_state.offset() {
                                    *fs.view.table_state.offset_mut() = idx;
                                }
                            }
                        }
                    }
                    return true;
                }

                KeyCode::Left => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) && !app.sidebar.sidebar_focus {
                        handle_quick_copy(app, event_tx, true);
                        return true;
                    }
                    if app.panes.len() > 1 && app.focused_pane_index > 0 {
                        app.focused_pane_index -= 1;
                    } else {
                        app.sidebar.sidebar_focus = true;
                    }
                    return true;
                }
                KeyCode::Right => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) && !app.sidebar.sidebar_focus {
                        handle_quick_copy(app, event_tx, false);
                        return true;
                    }
                    if app.sidebar.sidebar_focus {
                        app.sidebar.sidebar_focus = false;
                    } else if app.panes.len() > 1 && app.focused_pane_index < app.panes.len() - 1 {
                        app.focused_pane_index += 1;
                    }
                    return true;
                }

                KeyCode::Enter => {
                    handle_enter_key(app, event_tx);
                    return true;
                }

                KeyCode::Char('r') | KeyCode::Char('R') if has_control => {
                    // Ctrl+R: run the currently selected file
                    if let Some(fs) = app.current_file_state() {
                        if let Some(idx) = fs.list.selection.selected {
                            if let Some(path) = fs.list.files.get(idx) {
                                if !path.is_dir() {
                                    if let Some((work_dir, program, args)) =
                                        crate::modules::files::get_run_command(path)
                                    {
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::SpawnTerminal {
                                            path: work_dir,
                                            new_tab: true,
                                            remote: fs.nav.remote_session.clone(),
                                            command: Some(format!("{} {}", program, args.join(" "))),
                                        });
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                            "Running: {} {}",
                                            program,
                                            args.join(" ")
                                        )));
                                    } else {
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                            "No run command for: {}",
                                            path.extension()
                                                .and_then(|e| e.to_str())
                                                .map(|e| format!(".{e}"))
                                                .unwrap_or_else(|| "unknown".to_string())
                                        )));
                                    }
                                }
                            }
                        }
                    }
                    return true;
                }
                KeyCode::F(2) => {
                    let selected_count = app.current_file_state()
                        .map(|fs| {
                            if !fs.list.selection.is_empty() {
                                fs.list.selection.multi_selected_indices().len()
                            } else if fs.list.selection.selected.is_some() { 1 } else { 0 }
                        })
                        .unwrap_or(0);
                    if selected_count > 1 {
                        // Bulk rename - collect selected files
                        let files: Vec<PathBuf> = app.current_file_state()
                            .map(|fs| {
                                let mut paths = Vec::new();
                                if !fs.list.selection.is_empty() {
                                    for &idx in fs.list.selection.multi_selected_indices() {
                                        if let Some(p) = fs.list.files.get(idx) {
                                            paths.push(p.clone());
                                        }
                                    }
                                }
                                paths
                            })
                            .unwrap_or_default();
                        if !files.is_empty() {
                            app.core.mode = AppMode::BulkRename {
                                files,
                                pattern: String::new(),
                                replacement: String::new(),
                                matched_indices: Vec::new(),
                                selected_index: None,
                            };
                            app.core.input.clear();
                        }
                    } else {
                        handle_rename_shortcut(app);
                    }
                    return true;
                }
                KeyCode::F(3) => {
                    app.selection.selection_mode = !app.selection.selection_mode;
                    if !app.selection.selection_mode {
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.list.selection.clear_multi();
                        }
                    }
                    return true;
                }
                KeyCode::Delete => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        handle_permanent_delete_key(app, event_tx);
                    } else {
                        handle_trash_key(app, event_tx);
                    }
                    return true;
                }
                KeyCode::Char('~') => {
                    if let Some(fs) = app.current_file_state_mut() {
                        if let Some(home) = dirs::home_dir() {
                            fs.nav.current_path = home.clone();
                            fs.list.selection.selected = Some(0);
                            fs.list.selection.anchor = Some(0);
                            fs.list.selection.clear_multi();
                            *fs.view.table_state.offset_mut() = 0;
                            crate::event_helpers::push_history(fs, home);
                            let _ =
                                crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                            return true;
                        }
                    }
                    return false;
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    if !is_valid_search_char(c) {
                        return false;
                    }

                    if app.in_soft_shield() {
                        return true;
                    }

                    let is_sidebar = app.sidebar.sidebar_focus;
                    let mut needs_refresh = false;
                    let old_path = if !is_sidebar {
                        app.current_file_state()
                            .and_then(|fs| {
                                fs.list.selection.selected.and_then(|idx| fs.list.files.get(idx).cloned())
                            })
                    } else {
                        None
                    };
                    if let Some(fs) = app.current_file_state_mut() {
                        let now = std::time::Instant::now();
                        let should_refresh = fs.nav.search_debounce_until
                            .map(|until| now >= until)
                            .unwrap_or(true);

                        fs.nav.search_filter.push(c);
                        fs.nav.search_generation += 1;
                        if !is_sidebar {
                            reselect_after_filter(fs, old_path.as_deref());
                            needs_refresh = should_refresh;
                        }

                        fs.nav.search_debounce_until = Some(now + Duration::from_millis(SEARCH_DEBOUNCE_MS));
                    }
                    if is_sidebar {
                        app.sidebar.sidebar_index = 0;
                    }
                    if needs_refresh {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                    return true;
                }
                KeyCode::Backspace if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let mut handled_search = false;
                    let is_sidebar = app.sidebar.sidebar_focus;
                    let old_path = if !is_sidebar {
                        app.current_file_state()
                            .and_then(|fs| {
                                fs.list.selection.selected.and_then(|idx| fs.list.files.get(idx).cloned())
                            })
                    } else {
                        None
                    };
                    if let Some(fs) = app.current_file_state_mut() {
                        if !fs.nav.search_filter.is_empty() {
                            fs.nav.search_filter.pop();
                            if !is_sidebar {
                                reselect_after_filter(fs, old_path.as_deref());
                            }
                            fs.nav.search_debounce_until = Some(std::time::Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
                            handled_search = true;
                        }
                    }
                    if is_sidebar && handled_search {
                        app.sidebar.sidebar_index = 0;
                    }

                    if handled_search {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    } else {
                        crate::event_helpers::navigate_up(app);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                    return true;
                }
                KeyCode::Backspace
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    let is_sidebar = app.sidebar.sidebar_focus;
                    let old_path = if !is_sidebar {
                        app.current_file_state()
                            .and_then(|fs| {
                                fs.list.selection.selected.and_then(|idx| fs.list.files.get(idx).cloned())
                            })
                    } else {
                        None
                    };
                    if let Some(fs) = app.current_file_state_mut() {
                        delete_word_backwards(&mut fs.nav.search_filter);
                        if !is_sidebar {
                            reselect_after_filter(fs, old_path.as_deref());
                        }
                        fs.nav.search_debounce_until = Some(std::time::Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
                    }
                    if is_sidebar {
                        app.sidebar.sidebar_index = 0;
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    return true;
                }
                KeyCode::Char('w') if has_control => {
                    let is_sidebar = app.sidebar.sidebar_focus;
                    let old_path = if !is_sidebar {
                        app.current_file_state()
                            .and_then(|fs| {
                                fs.list.selection.selected.and_then(|idx| fs.list.files.get(idx).cloned())
                            })
                    } else {
                        None
                    };
                    if let Some(fs) = app.current_file_state_mut() {
                        delete_word_backwards(&mut fs.nav.search_filter);
                        if !is_sidebar {
                            reselect_after_filter(fs, old_path.as_deref());
                        }
                        fs.nav.search_debounce_until = Some(std::time::Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
                    }
                    if is_sidebar {
                        app.sidebar.sidebar_index = 0;
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    return true;
                }
                KeyCode::Char('u') if has_control => {
                    let is_sidebar = app.sidebar.sidebar_focus;
                    if let Some(fs) = app.current_file_state_mut() {
                        fs.nav.search_filter.clear();
                        if !is_sidebar {
                            fs.list.selection.selected = Some(0);
                            fs.list.selection.anchor = Some(0);
                            *fs.view.table_state.offset_mut() = 0;
                        } else {
                            app.sidebar.sidebar_index = 0;
                        }
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                    return true;
                }
                _ => {}
            }
        }
    }
    false
}

pub fn handle_file_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    _panes_needing_refresh: &mut HashSet<usize>,
) -> bool {
    let column = me.column;
    let row = me.row;
    let (w, _h) = app.core.terminal_size;
    let sw = app.sidebar_width();

    if let MouseEventKind::Down(_) = me.kind {
        if column >= sw {
            let cw = w.saturating_sub(sw);
            let pc = app.panes.len();
            if pc == 0 {
                return false;
            }
            let pw = cw / pc as u16;
            if pw == 0 {
                return false;
            }
            let cp = (column.saturating_sub(sw) / pw) as usize;
            if cp < pc {
                app.focused_pane_index = cp;
                app.sidebar.sidebar_focus = false;
            }
        }
    }

    match me.kind {
        MouseEventKind::Down(button) => {
            // Reset marquee on new down-click
            app.drag.clear_marquee();

            if matches!(app.core.mode, AppMode::PathInput) {
                let keep_open = app.current_file_state()
                    .and_then(|fs| fs.view.breadcrumb_header_bounds)
                    .map(|rect| rect.contains(ratatui::layout::Position { x: column, y: row }))
                    .unwrap_or(false);
                if keep_open {
                    return true;
                }
                app.core.mode = AppMode::Normal;
                app.core.input.clear();
                app.core.input.style = ratatui::style::Style::default().fg(ratatui::style::Color::White);
                app.core.input.cursor_style = ratatui::style::Style::default()
                    .bg(ratatui::style::Color::White)
                    .fg(ratatui::style::Color::Black);
            }

            // 1. Breadcrumb Click
            if let Some(fs) = app.current_file_state_mut() {
                let in_breadcrumb_row = fs
                    .view.breadcrumb_header_bounds
                    .map(|r| r.contains(ratatui::layout::Position { x: column, y: row }))
                    .unwrap_or(false);

                if in_breadcrumb_row {
                    // Check breadcrumb segments first: clicking a segment navigates
                    let clicked_segment = fs
                        .view.breadcrumb_bounds
                        .iter()
                        .find(|(r, _)| r.contains(ratatui::layout::Position { x: column, y: row }))
                        .map(|(_, p)| p.clone());

                    if let Some(target_path) = clicked_segment {
                        let current_path = fs.nav.current_path.clone();

                        // Smart Selection
                        if current_path.starts_with(&target_path) && current_path != target_path {
                            if let Ok(prefix) = current_path.strip_prefix(&target_path) {
                                if let Some(component) = prefix.components().next() {
                                    let child_name = component.as_os_str();
                                    fs.view.pending_select_path = Some((target_path.join(child_name), 0));
                                }
                            }
                        }

                        fs.nav.current_path = target_path.clone();
                        fs.list.selection.clear();
                        fs.nav.search_filter.clear();
                        *fs.view.table_state.offset_mut() = 0;
                        crate::event_helpers::push_history(fs, target_path);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        app.sidebar.sidebar_focus = false;
                        return true;
                    }

                    // Clicked breadcrumb row but not on a segment:
                    // copy path to clipboard and open path input
                    let path = fs.nav.current_path.to_string_lossy().to_string();
                    crate::event_helpers::open_path_input(app);
                    crate::event_helpers::copy_text_to_clipboard_async(path);
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                        "Copied current path to clipboard".to_string(),
                    ));
                    return true;
                }
            }

            // 2. Sorting (Header Clicks)
            if row == 1 || row == 2 {
                if let MouseEventKind::Down(MouseButton::Left) = me.kind {
                    if column >= sw {
                        let cw = w.saturating_sub(sw);
                        let pc = app.panes.len();
                        if pc == 0 {
                            return false;
                        }
                        let pw = cw / pc as u16;
                        if pw == 0 {
                            return false;
                        }
                        let cp = (column.saturating_sub(sw) / pw) as usize;
                        if let Some(fs) = app.panes.get_mut(cp).and_then(|p| p.current_state_mut())
                        {
                            for (r, col) in &fs.view.column_bounds {
                                if column >= r.x && column < r.x.saturating_add(r.width) {
                                    if fs.nav.sort_column == *col {
                                        fs.nav.sort_ascending = !fs.nav.sort_ascending;
                                    } else {
                                        fs.nav.sort_column = *col;
                                        fs.nav.sort_ascending = true;
                                    }
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(cp));
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            // 3. File Row Interaction
            if row >= 3 {
                let Some(idx) = crate::event_helpers::fs_mouse_index(row, app) else {
                    // Empty space click — deselect all and start marquee tracking
                    if button == MouseButton::Left && column >= sw {
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.list.selection.clear_multi();
                            fs.list.selection.selected = None;
                        }
                        app.drag.marquee_start = Some((column, row));
                        app.drag.marquee_end = Some((column, row));
                    }
                    if button == MouseButton::Right && column >= sw {
                        let target = ContextMenuTarget::EmptySpace;
                        let actions = crate::event_helpers::get_context_menu_actions(&target, app);
                        app.core.mode = AppMode::ContextMenu {
                            x: column,
                            y: row,
                            target,
                            actions,
                            selected_index: None,
                        };
                        return true;
                    }
                    return true;
                };
                let mut sp = None;
                let mut is_dir = false;
                let mut pending_click: Option<usize> = None;
                let is_shift = me.modifiers.contains(KeyModifiers::SHIFT)
                    || me.modifiers.contains(KeyModifiers::ALT);
                let is_ctrl = me.modifiers.contains(KeyModifiers::CONTROL);
                let has_mods = is_shift || is_ctrl;
                app.selection.prevent_mouse_up_selection_cleanup = has_mods;

                let sel_mode = app.selection.selection_mode;
                if let Some(fs) = app.current_file_state_mut() {
                    let is_divider = is_virtual_divider(&fs.list.files[idx]);
                    if is_divider {
                        return true;
                    }

                    if button == MouseButton::Left {
                        // Defer handle_click for plain clicks — resolve on mouseUp.
                        // If the user drags, it becomes marquee instead.
                        // Ctrl/Shift clicks always fire immediately (they're selection ops).
                        if is_ctrl || is_shift {
                            fs.list.selection.handle_click(
                                idx,
                                is_shift,
                                is_ctrl,
                                sel_mode && !is_shift,
                            );
                            fs.view.table_state.select(fs.list.selection.selected);
                        } else {
                            // For plain clicks, record the target but don't change selection yet.
                            // mouseUp will call handle_click if no drag/marquee occurred.
                            pending_click = Some(idx);
                        }
                    }

                    let p = fs.list.files[idx].clone();
                    is_dir = fs.list.metadata.get(&p).map(|m| m.is_dir).unwrap_or(false);
                    sp = Some(p.clone());

                    // Arrow click on folder: toggle expand/collapse only
                    if is_dir && button == MouseButton::Left {
                        if let Some((name_rect, _)) = fs.view.column_bounds.iter().find(|(_, ct)| *ct == FileColumn::Name) {
                            if column >= name_rect.x && column < name_rect.x + name_rect.width {
                                let clicked_arrow = fs.view.file_row_bounds.iter()
                                    .find(|b| b.file_idx == idx)
                                    .is_some_and(|b| b.arrow_end_x > 0 && column < b.arrow_end_x);
                                if clicked_arrow {
                                    let folder_path = p.clone();
                                    let _ = fs;
                                    let was_expanded = app.layout.expanded_folders.contains(&folder_path);
                                    if was_expanded {
                                        app.layout.expanded_folders.remove(&folder_path);
                                    } else {
                                        app.layout.expanded_folders.insert(folder_path.clone());
                                    }
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                                    return true;
                                }
                            }
                        }
                    }
                }

                if let Some(path) = sp {
                    if button == MouseButton::Right {
                        let target = if is_dir {
                            ContextMenuTarget::Folder(idx)
                        } else {
                            ContextMenuTarget::File(idx)
                        };
                        let actions = crate::event_helpers::get_context_menu_actions(&target, app);
                        app.core.mode = AppMode::ContextMenu {
                            x: column,
                            y: row,
                            target,
                            actions,
                            selected_index: None,
                        };
                        return true;
                    }
                    if button == MouseButton::Middle {
                        if is_dir {
                            if let Some(p) = app.panes.get_mut(app.focused_pane_index) {
                                if let Some(fs) = p.current_state() {
                                    let mut nfs = fs.clone();
                                    nfs.nav.current_path = path.clone();
                                    nfs.list.selection.clear();
                                    crate::event_helpers::push_history(&mut nfs, path);
                                    p.open_tab(nfs);
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                                }
                            }
                        } else {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                                if app.focused_pane_index == 0 { 1 } else { 0 },
                                path,
                            ));
                        }
                        return true;
                    }
                    // Set marquee_start for ALL left-clicks on file rows —
                    // marquee can coexist with drag_source; the Drag handler decides which wins.
                    app.drag.marquee_start = Some((column, row));
                    app.drag.marquee_end = Some((column, row));
                    // Only set drag_source for Name column clicks (file drag-and-drop)
                    let in_name_column = app.current_file_state()
                        .and_then(|fs| fs.view.column_bounds.iter()
                            .find(|(_, ct)| *ct == FileColumn::Name)
                            .map(|(name_rect, _)| {
                                column >= name_rect.x && column < name_rect.x + name_rect.width
                            }))
                        .unwrap_or(true);
                    if in_name_column {
                        app.drag.drag_source = Some(path.clone());
                        app.drag.drag_start_pos = Some((column, row));
                    }

                    // Double Click
                    if button == MouseButton::Left
                        && is_double_click(app.mouse.mouse_click_pos, app.mouse.mouse_last_click, column, row)
                    {
                        if path.is_dir() {
                            if let Some(fs) = app.current_file_state_mut() {
                                fs.nav.current_path = path.clone();
                                fs.list.selection.clear();
                                fs.git.git_cache_until = None;
                                crate::event_helpers::push_history(fs, path);
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                            }
                        } else {
                            let _ = open_file_or_navigate(&path);
                        }
                    }
                    app.mouse.mouse_last_click = std::time::Instant::now();
                    app.mouse.mouse_click_pos = (column, row);
                }
                // Apply deferred pending_click after all borrows are released
                app.drag.pending_click_idx = pending_click;
            }

            if button == MouseButton::Middle {
                if let Some(text) = dracon_terminal_engine::utils::get_primary_selection_text() {
                    if let Some(fs) = app.current_file_state_mut() {
                        let sanitized: String = text.chars().filter(|&c| is_valid_search_char(c)).collect();
                        fs.nav.search_filter.push_str(&sanitized);
                        fs.nav.search_generation += 1;
                        fs.nav.search_debounce_until = Some(std::time::Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                }
            }
            true
        }
        MouseEventKind::Up(_) => {
            // Commit marquee selection if active
            if app.drag.is_marquee {
                if let Some(rect) = app.drag.marquee_rect() {
                    let is_ctrl = me.modifiers.contains(KeyModifiers::CONTROL);
                    if let Some(fs) = app.current_file_state_mut() {
                        if !is_ctrl {
                            fs.list.selection.clear_multi();
                        }
                        // Select all file indices whose rows fall within the marquee rect
                        for bound in &fs.view.file_row_bounds {
                            if bound.file_idx >= fs.list.files.len() {
                                continue;
                            }
                            // Skip virtual dividers
                            if is_virtual_divider(&fs.list.files[bound.file_idx]) {
                                continue;
                            }
                            let offset = fs.view.table_state.offset();
                            let file_screen_row = 3 + bound.file_idx.saturating_sub(offset);
                            if file_screen_row >= rect.min_row as usize
                                && file_screen_row <= rect.max_row as usize
                            {
                                if is_ctrl {
                                    fs.list.selection.toggle(bound.file_idx);
                                } else {
                                    fs.list.selection.add(bound.file_idx);
                                }
                            }
                        }
                        // Set primary selected to the first selected item
                        fs.list.selection.selected = fs.list.selection.multi_selected_indices().iter().min().copied();
                        if let Some(s) = fs.list.selection.selected {
                            fs.view.table_state.select(Some(s));
                        }
                    }
                }
                app.drag.clear_marquee();
                app.drag.drag_start_pos = None;
                app.drag.drag_source = None;
                app.drag.hovered_drop_target = None;
                app.drag.pending_click_idx = None;
                return true;
            }

            if app.drag.is_dragging {
                // Drop Logic
                if let Some(DropTarget::Folder(target_path)) = app.drag.hovered_drop_target.take() {
                    if let Some(source_path) = app.drag.drag_source.take() {
                        if source_path != target_path {
                            app.core.mode = AppMode::DragDropMenu {
                                sources: vec![source_path],
                                target: target_path,
                            };
                        }
                    }
                }
                app.drag.is_dragging = false;
            }
            let sel_mode = app.selection.selection_mode;
            // Resolve deferred pending_click — this is a plain click (no drag/marquee)
            if let Some(idx) = app.drag.pending_click_idx.take() {
                if let Some(fs) = app.current_file_state_mut() {
                    if idx < fs.list.files.len() && !is_virtual_divider(&fs.list.files[idx]) {
                        fs.list.selection.handle_click(idx, false, false, false);
                        fs.view.table_state.select(fs.list.selection.selected);
                    }
                }
            } else if row >= 3
                && !app.selection.prevent_mouse_up_selection_cleanup
                && !sel_mode
                && !me.modifiers.contains(KeyModifiers::SHIFT)
            {
                // Fallback: no pending_click, no marquee, no modifiers → select clicked item
                let Some(idx) = crate::event_helpers::fs_mouse_index(row, app) else {
                    return true;
                };
                if let Some(fs) = app.current_file_state_mut() {
                    if is_virtual_divider(&fs.list.files[idx]) {
                        return true;
                    }
                    fs.list.selection.clear();
                    fs.list.selection.selected = Some(idx);
                    fs.view.table_state.select(Some(idx));
                }
            }
            app.drag.drag_start_pos = None;
            app.drag.drag_source = None;
            app.drag.hovered_drop_target = None;
            app.drag.pending_click_idx = None;
            app.drag.clear_marquee();
            true
        }
        MouseEventKind::Moved => {
            if app.drag.is_dragging {
                app.drag.is_dragging = false;
                app.drag.drag_source = None;
                app.drag.drag_start_pos = None;
                app.drag.hovered_drop_target = None;
            }
            if app.drag.is_marquee {
                app.drag.clear_marquee();
            }
            app.drag.drag_start_pos = None;
            true
        }
        MouseEventKind::Drag(_) => {
            // Marquee drag: prefer marquee over file drag when drag hasn't started yet.
            // If drag_source is set but is_dragging is false, let marquee take over
            // once distance threshold is met (user is selecting, not dragging files).
            if let Some((sx, sy)) = app.drag.marquee_start {
                app.drag.marquee_end = Some((column, row));
                let dist_sq =
                    (column as f32 - sx as f32).powi(2) + (row as f32 - sy as f32).powi(2);
                // Activate marquee if: distance threshold met AND file drag hasn't started
                if dist_sq >= 4.0 && !app.drag.is_marquee && !app.drag.is_dragging {
                    app.drag.is_marquee = true;
                    // Cancel file drag — marquee takes priority
                    app.drag.drag_source = None;
                    app.drag.drag_start_pos = None;
                }
                if app.drag.is_marquee {
                    return true; // consume drag event — no file drag-drop while marquee-ing
                }
            }

            let mut changed = false;
            if let Some((sx, sy)) = app.drag.drag_start_pos {
                let dist_sq =
                    (column as f32 - sx as f32).powi(2) + (row as f32 - sy as f32).powi(2);
                // File drag threshold: 3px (dist_sq >= 9.0) — higher than marquee (2px)
                // so marquee can activate first for selection drags
                if dist_sq >= 9.0
                    && !me.modifiers.contains(KeyModifiers::SHIFT)
                    && !app.selection.selection_mode
                    && !app.drag.is_dragging
                {
                    app.drag.is_dragging = true;
                    changed = true;
                }
            }

            // Update drop target if dragging
            if app.drag.is_dragging {
                let prev_target = app.drag.hovered_drop_target.clone();
                app.drag.hovered_drop_target = None;
                if column >= sw {
                    if let Some(fs) = app.current_file_state() {
                        // Breadcrumb drop target (e.g., move to parent path quickly).
                        if let Some((_, crumb_path)) = fs.view.breadcrumb_bounds.iter().find(|(r, _)| {
                            r.contains(ratatui::layout::Position { x: column, y: row })
                        }) {
                            if let Some(src) = &app.drag.drag_source {
                                if src != crumb_path {
                                    app.drag.hovered_drop_target =
                                        Some(DropTarget::Folder(crumb_path.clone()));
                                }
                            }
                        }
                    }

                    // File row folder targets.
                    if app.drag.hovered_drop_target.is_none() && row >= 3 {
                        if let Some(idx) = crate::event_helpers::fs_mouse_index(row, app) {
                            if let Some(fs) = app.current_file_state() {
                                if let Some(path) = fs.list.files.get(idx) {
                                    if path.is_dir() {
                                        if let Some(src) = &app.drag.drag_source {
                                            if src != path {
                                                app.drag.hovered_drop_target =
                                                    Some(DropTarget::Folder(path.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if app.drag.hovered_drop_target != prev_target {
                    changed = true;
                }
            }

            // Selection extension
            let sel_mode = app.selection.selection_mode;
            if row >= 3
                && column >= sw
                && (me.modifiers.contains(KeyModifiers::SHIFT) || sel_mode)
                && !app.drag.is_dragging
            {
                let Some(idx) = crate::event_helpers::fs_mouse_index(row, app) else {
                    return true;
                };
                if let Some(fs) = app.current_file_state_mut() {
                    if !fs.list.files.is_empty() {
                        let idx = idx.min(fs.list.files.len().saturating_sub(1));
                        let anchor = fs
                            .list.selection
                            .anchor
                            .unwrap_or(fs.list.selection.selected.unwrap_or(0));
                        fs.list.selection.clear_multi();
                        for i in std::cmp::min(anchor, idx)..=std::cmp::max(anchor, idx) {
                            fs.list.selection.add(i);
                        }
                        fs.list.selection.selected = Some(idx);
                        fs.view.table_state.select(Some(idx));
                        changed = true;
                    }
                }
            }

            if app.drag.is_dragging {
                // Keep repainting while dragging to move drag ghost with cursor.
                true
            } else {
                changed
            }
        }
        MouseEventKind::ScrollUp => {
            if let Some(fs) = app.current_file_state_mut() {
                let new_offset = fs.view.table_state.offset().saturating_sub(1);
                *fs.view.table_state.offset_mut() = new_offset;
            }
            true
        }
        MouseEventKind::ScrollDown => {
            if let Some(fs) = app.current_file_state_mut() {
                let max_offset = fs
                    .list.files
                    .len()
                    .saturating_sub(fs.view.view_height.saturating_sub(3));
                let new_offset = fs.view.table_state.offset().saturating_add(1).min(max_offset);
                *fs.view.table_state.offset_mut() = new_offset;
            }
            true
        }
        _ => false,
    }
}

fn handle_space_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
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
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                } else {
                    let (is_binary, _, _) = crate::modules::files::check_file_suitability(&path, u64::MAX);
                    if is_binary {
                        return;
                    }
                    let target_pane = app.focused_pane_index
                        .min(app.panes.len().saturating_sub(1));
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(target_pane, path));
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

fn handle_enter_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
    if app.sidebar.sidebar_focus {
        let target_opt = app.sidebar.sidebar_bounds
            .iter()
            .find(|b| b.index == app.sidebar.sidebar_index)
            .map(|b| b.target.clone());
        if let Some(target) = target_opt {
            match target {
                SidebarTarget::Favorite(path) | SidebarTarget::Recent(path) => {
                    if let Some(fs) = app.current_file_state_mut() {
                        fs.nav.current_path = path.clone();
                        fs.list.selection.selected = Some(0);
                        fs.list.selection.anchor = Some(0);
                        fs.list.selection.clear_multi();
                        crate::event_helpers::push_history(fs, path.clone());
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        app.sidebar.sidebar_focus = false;
                    }
                }
                SidebarTarget::Remote(idx) => {
                    let _ =
                        crate::app::try_send_event(&event_tx, AppEvent::ConnectToRemote(app.focused_pane_index, idx));
                }
                SidebarTarget::Project(path) => {
                    if path.is_dir() {
                        // Enter on folder = navigate + expand (consistent with name click)
                        let path_ref = path.clone();
                        let was_expanded = app.sidebar.tree_expanded_folders.contains(&path_ref);
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.nav.current_path = path.clone();
                            fs.list.selection.selected = Some(0);
                            fs.list.selection.anchor = Some(0);
                            fs.list.selection.clear_multi();
                            crate::event_helpers::push_history(fs, path.clone());
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        }
                        if !was_expanded {
                            app.sidebar.tree_expanded_folders.insert(path_ref);
                        }
                        app.sidebar.sidebar_focus = false;
                    } else {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(app.focused_pane_index, path));
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
            crate::event_helpers::push_history(fs, p);
            // Clear expanded folders when entering a new directory — start fresh
            app.layout.expanded_folders.clear();
            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
        }
    }
}

fn handle_rename_shortcut(app: &mut App) {
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

fn handle_trash_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
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
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::TrashFile(p));
                }
            }
        }
    }
}

fn handle_permanent_delete_key(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) {
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
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::Delete(p));
                }
            }
        }
    }
}

fn handle_quick_copy(app: &mut App, event_tx: &mpsc::Sender<AppEvent>, _to_left: bool) {
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
                let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(p, dest));
            }
        }
    }
}

fn path_join(base: &std::path::Path, name: &std::ffi::OsStr) -> PathBuf {
    base.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_allows_small_pointer_drift() {
        let now = std::time::Instant::now();
        assert!(is_double_click((10, 10), now, 11, 10));
        assert!(is_double_click((10, 10), now, 9, 11));
        assert!(!is_double_click((10, 10), now, 13, 10));
        assert!(!is_double_click(
            (10, 10),
            now - Duration::from_millis(700),
            10,
            10
        ));
    }

    #[test]
    fn sort_toggle_toggles_ascending_on_same_column() {
        use crate::state::FileColumn;
        let mut sort_column = FileColumn::Name;
        let mut sort_ascending = true;

        // Click same column (Name) → toggle descending
        let clicked = FileColumn::Name;
        if sort_column == clicked {
            sort_ascending = !sort_ascending;
        } else {
            sort_column = clicked;
            sort_ascending = true;
        }
        assert!(!sort_ascending, "should be descending after toggle");
        assert_eq!(sort_column, FileColumn::Name);

        // Click different column (Size) → switch to Size, ascending
        let clicked = FileColumn::Size;
        if sort_column == clicked {
            sort_ascending = !sort_ascending;
        } else {
            sort_column = clicked;
            sort_ascending = true;
        }
        assert!(sort_ascending, "new column should start ascending");
        assert_eq!(sort_column, FileColumn::Size);

        // Click Size again → toggle to descending
        let clicked = FileColumn::Size;
        if sort_column == clicked {
            sort_ascending = !sort_ascending;
        } else {
            sort_column = clicked;
            sort_ascending = true;
        }
        assert!(!sort_ascending, "should be descending after second toggle");
        assert_eq!(sort_column, FileColumn::Size);
    }

    #[test]
    fn column_bounds_match_click() {
        use crate::state::FileColumn;
        use ratatui::layout::Rect;

        // Simulate column_bounds as produced by render
        let column_bounds = vec![
            (Rect::new(40, 2, 60, 1), FileColumn::Name),
            (Rect::new(100, 2, 12, 1), FileColumn::Size),
            (Rect::new(112, 2, 20, 1), FileColumn::Modified),
        ];

        // Click at column 50, row 2 → should match Name
        let col = 50u16;
        for (r, c) in &column_bounds {
            if col >= r.x && col < r.x.saturating_add(r.width) {
                assert_eq!(*c, FileColumn::Name);
                return;
            }
        }
        panic!("click at column 50 should match Name column");
    }
}
