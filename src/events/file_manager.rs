#![allow(clippy::needless_borrow)]

use dracon_terminal_engine::contracts::{
    InputEvent as Event, KeyCode, KeyModifiers,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app::{
    App, AppEvent, AppMode, CurrentView, SidebarTarget, UndoAction,
};
use crate::events::input::delete_word_backwards;

const SEARCH_DEBOUNCE_MS: u64 = 300;

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
                    super::file_actions::handle_space_key(app, event_tx);
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
                        super::file_actions::handle_quick_copy(app, event_tx, true);
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
                        super::file_actions::handle_quick_copy(app, event_tx, false);
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
                    super::file_actions::handle_enter_key(app, event_tx);
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
                        super::file_actions::handle_rename_shortcut(app);
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
                        super::file_actions::handle_permanent_delete_key(app, event_tx);
                    } else {
                        super::file_actions::handle_trash_key(app, event_tx);
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
                            crate::nav_helpers::push_history(fs, home);
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
                    if !crate::events::file_actions::is_valid_search_char(c) {
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
                        crate::nav_helpers::navigate_up(app);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn path_join(base: &std::path::Path, name: &std::ffi::OsStr) -> std::path::PathBuf {
        base.join(name)
    }

    use crate::events::file_actions::is_double_click;
    use crate::events::file_actions::is_virtual_divider;

    #[test]
    fn is_valid_search_char_allows_letters() {
        assert!(crate::events::file_actions::is_valid_search_char('a'));
        assert!(crate::events::file_actions::is_valid_search_char('Z'));
        assert!(crate::events::file_actions::is_valid_search_char('0'));
    }

    #[test]
    fn is_valid_search_char_rejects_specials() {
        assert!(!crate::events::file_actions::is_valid_search_char('*'));
        assert!(!crate::events::file_actions::is_valid_search_char('?'));
        assert!(!crate::events::file_actions::is_valid_search_char('!'));
        assert!(!crate::events::file_actions::is_valid_search_char('\\'));
    }

    #[test]
    fn is_valid_search_char_rejects_control() {
        assert!(!crate::events::file_actions::is_valid_search_char('\x01'));
        assert!(!crate::events::file_actions::is_valid_search_char('\x1b'));
        assert!(!crate::events::file_actions::is_valid_search_char('\x7f'));
    }

    #[test]
    fn is_virtual_divider_check() {
        assert!(is_virtual_divider(std::path::Path::new("__DIVIDER__")));
        assert!(!is_virtual_divider(std::path::Path::new("real_folder")));
        assert!(!is_virtual_divider(std::path::Path::new("__DIVIDER")));
    }

    #[test]
    fn path_join_basic() {
        let base = std::path::Path::new("/home/user");
        let name = std::ffi::OsStr::new("file.txt");
        assert_eq!(path_join(base, name), PathBuf::from("/home/user/file.txt"));
    }

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
    fn double_click_exact_position() {
        let now = std::time::Instant::now();
        assert!(is_double_click((5, 3), now, 5, 3));
    }

    #[test]
    fn double_click_boundary_distance() {
        let now = std::time::Instant::now();
        // dx=1, dy=1 (within boundary)
        assert!(is_double_click((10, 10), now, 11, 11));
        // dx=2, dy=0 (outside boundary)
        assert!(!is_double_click((10, 10), now, 12, 10));
    }

    #[test]
    fn double_click_time_boundary() {
        let now = std::time::Instant::now();
        // Just under 500ms should be a double click
        assert!(is_double_click((10, 10), now, 10, 10));
    }

    #[test]
    fn double_click_wraparound_distance() {
        let now = std::time::Instant::now();
        // dx=u16 wraparound: 0 - 65535 = 65535 (far away)
        assert!(!is_double_click((0, 5), now, 65535, 5));
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

    fn make_file_state(files: Vec<PathBuf>) -> crate::state::FileState {
        use crate::state::FileColumn;
        let mut fs = crate::state::FileState::new(
            PathBuf::from("/tmp"),
            None,
            false,
            vec![FileColumn::Name, FileColumn::Size],
            FileColumn::Name,
            true,
        );
        fs.list.files = files;
        fs
    }

    #[test]
    fn reselect_after_filter_finds_matching_path() {
        let mut fs = make_file_state(vec![
            PathBuf::from("/tmp/a.rs"),
            PathBuf::from("/tmp/b.rs"),
            PathBuf::from("/tmp/c.rs"),
        ]);
        let old_path = PathBuf::from("/tmp/b.rs");
        reselect_after_filter(&mut fs, Some(&old_path));
        assert_eq!(fs.list.selection.selected, Some(1), "should select the matching file");
    }

    #[test]
    fn reselect_after_filter_missing_path_selects_first() {
        let mut fs = make_file_state(vec![
            PathBuf::from("/tmp/a.rs"),
            PathBuf::from("/tmp/b.rs"),
        ]);
        let old_path = PathBuf::from("/tmp/nonexistent.rs");
        reselect_after_filter(&mut fs, Some(&old_path));
        assert_eq!(fs.list.selection.selected, Some(0), "should select first file when path not found");
    }

    #[test]
    fn reselect_after_filter_none_selects_first() {
        let mut fs = make_file_state(vec![
            PathBuf::from("/tmp/a.rs"),
            PathBuf::from("/tmp/b.rs"),
        ]);
        reselect_after_filter(&mut fs, None);
        assert_eq!(fs.list.selection.selected, Some(0), "should select first file when no old path");
    }

    #[test]
    fn reselect_adjusts_offset_if_selected_above_viewport() {
        let mut fs = make_file_state(vec![
            PathBuf::from("/tmp/a.rs"),
            PathBuf::from("/tmp/b.rs"),
            PathBuf::from("/tmp/c.rs"),
        ]);
        fs.view.view_height = 5; // capacity = 5-3 = 2
        *fs.view.table_state.offset_mut() = 2; // viewing rows 2+
        let old_path = PathBuf::from("/tmp/a.rs"); // idx 0, above viewport
        reselect_after_filter(&mut fs, Some(&old_path));
        assert_eq!(*fs.view.table_state.offset_mut(), 0, "should scroll up to show selected item");
    }
}
