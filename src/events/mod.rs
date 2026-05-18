#![allow(clippy::needless_borrow, clippy::collapsible_match, clippy::manual_checked_ops)]

use crate::app::{
    App, AppEvent, AppMode, ContextMenuTarget, CurrentView, DropTarget, SidebarTarget,
};
use dracon_terminal_engine::contracts::{
    InputEvent as Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use std::collections::HashSet;
use tokio::sync::mpsc;

pub mod editor;
pub mod editor_modals;
pub mod file_manager;
pub mod git;
pub mod input;
pub mod modal_mouse;
pub mod modals;
pub mod monitor;
pub mod mouse_helpers;
pub mod settings_handlers;

/// Top-level event dispatcher for keyboard and mouse input.
///
/// Priority order:
/// 1. Input shield (cooldown after mode changes)
/// 2. Resize events
/// 3. Modal/overlay handling (if non-Normal mode)
/// 4. View-specific keyboard dispatch
/// 5. General mouse dispatch (sidebar, tabs, panes)
///
/// Returns `true` if the event was consumed.
pub fn handle_event(
    evt: Event,
    app: &mut App,
    event_tx: mpsc::Sender<AppEvent>,
    panes_needing_refresh: &mut HashSet<usize>,
) -> bool {
    // 1. Input Shield / Cooldown
    if let Some(until) = app.output.input_shield_until {
        if std::time::Instant::now() < until {
            if let Event::Resize(w, h) = evt {
                app.core.terminal_size = (w, h);
            }
            return true;
        }
    }

    // 2. Global Resize
    if let Event::Resize(w, h) = evt {
        app.core.terminal_size = (w, h);
        return true;
    }

    // 3. Mode-specific logic (Modals, Overlays)
    if !matches!(app.core.mode, AppMode::Normal)
        && crate::events::modals::handle_modal_events(&evt, app, &event_tx) {
            return true;
        }

    // 4. View-specific logic (Keyboard)
    match &evt {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return false;
            }

            let has_control = key.modifiers.contains(KeyModifiers::CONTROL);

            // Global Quit
            if (key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q')) && has_control {
                crate::config::save_state_quiet(app);
                app.core.running = false;
                return true;
            }

            // Plain Escape should exit non-files views.
            if key.code == KeyCode::Esc
                && matches!(
                    app.core.current_view,
                    CurrentView::Git
                        | CurrentView::Processes
                        | CurrentView::Editor
                        | CurrentView::Commit
                        | CurrentView::Debug
                )
            {
                return handle_global_escape(app, &event_tx);
            }

            // Global Escape (Ctrl+[)
            if has_control && key.code == KeyCode::Char('[') {
                return handle_global_escape(app, &event_tx);
            }

            // --- GLOBAL OVERRIDES (High Priority) ---
            if has_control {
                if key.code == KeyCode::Char('d') || key.code == KeyCode::Char('D') {
                    app.core.current_view = if app.core.current_view == CurrentView::Debug {
                        CurrentView::Files
                    } else {
                        CurrentView::Debug
                    };
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                match key.code {
                    KeyCode::Char('m') | KeyCode::Char('M') if app.core.current_view == CurrentView::Editor => {
                        app.show_main_stage = !app.show_main_stage;
                        return true;
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        app.toggle_split();
                        app.save_current_view_prefs();
                        crate::config::save_state_quiet(app);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(0));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(1));
                        return true;
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.sidebar.show_sidebar = !app.sidebar.show_sidebar;
                            app.save_current_view_prefs();
                        }
                        return true;
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Editor);
                        return true;
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::GitHistory);
                        return true;
                    }
                    _ => {}
                }
            }

            match app.core.current_view {
                CurrentView::Editor => {
                    if editor::handle_editor_events(&evt, app, &event_tx) {
                        return true;
                    }
                }
                CurrentView::Commit => {
                    if editor::handle_editor_events(&evt, app, &event_tx) {
                        return true;
                    }
                }
                CurrentView::Processes => {
                    if monitor::handle_monitor_events(&evt, app, &event_tx) {
                        return true;
                    }
                }
                CurrentView::Git => {
                    if git::handle_git_events(&evt, app, &event_tx) {
                        return true;
                    }
                }
                CurrentView::Debug => {
                    return true;
                }
                CurrentView::Files => {
                    if file_manager::handle_file_events(&evt, app, &event_tx) {
                        return true;
                    }
                }
            }
        }
        Event::Mouse(me) => {
            return handle_general_mouse(me, app, &event_tx, panes_needing_refresh);
        }
        Event::Paste(text) => {
            if let AppMode::Editor = app.core.mode {
                if let Some(preview) = &mut app.editor_global.editor_state {
                    if let Some(editor) = &mut preview.editor {
                        editor.insert_string(text);
                        if app.settings.auto_save {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::SaveFile(
                                preview.path.clone(),
                                editor.get_content(),
                            ));
                            editor.modified = false;
                        }
                        return true;
                    }
                }
            }
        }
        _ => {}
    }

    false
}

fn handle_global_escape(app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> bool {
    if app.drag.is_dragging {
        app.drag.is_dragging = false;
        app.drag.drag_source = None;
        app.drag.drag_start_pos = None;
        app.drag.hovered_drop_target = None;
        return true;
    }
    // Cancel marquee selection on Escape
    if app.drag.is_marquee {
        app.drag.clear_marquee();
        return true;
    }
    if app.core.current_view == CurrentView::Commit {
        app.core.current_view = CurrentView::Git;
        app.core.mode = AppMode::Normal;
        app.editor_global.editor_state = None;
        app.sidebar.sidebar_focus = false;
        app.core.input.clear();
        app.set_input_shield(60);
        for pane in &mut app.panes {
            for fs in &mut pane.tabs {
                if let Some(preview) = &fs.view.preview {
                    let p = preview.path.to_string_lossy();
                    if p.starts_with("git://") || p.starts_with("git-diff://") {
                        fs.view.preview = None;
                    }
                }
            }
        }
        return true;
    }

    if matches!(app.core.mode, AppMode::Normal) {
        match app.core.current_view {
            CurrentView::Git | CurrentView::Processes | CurrentView::Debug => {
                if let Some(fs) = app.current_file_state_mut() {
                    fs.nav.search_filter.clear();
                    fs.git.git_pending_state.select(None);
                    fs.git.git_history_state.select(None);
                }
                for pane in &mut app.panes {
                    for fs in &mut pane.tabs {
                        if let Some(preview) = &fs.view.preview {
                            let p = preview.path.to_string_lossy();
                            if p.starts_with("git://") || p.starts_with("git-diff://") {
                                fs.view.preview = None;
                            }
                        }
                    }
                }
                app.core.mode = AppMode::Normal;
                app.core.input.clear();
                app.core.current_view = CurrentView::Files;
                app.set_input_shield(150);
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                return true;
            }
            CurrentView::Editor => {
                if let Some(preview) = &app.editor_global.editor_state {
                    if let Some(editor) = &preview.editor {
                        if editor.modified {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::SaveFile(
                                preview.path.clone(),
                                editor.get_content(),
                            ));
                        }
                    }
                }
                for pane in &mut app.panes {
                    for fs in &mut pane.tabs {
                        if let Some(preview) = &fs.view.preview {
                            if let Some(editor) = &preview.editor {
                                if editor.modified {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::SaveFile(
                                        preview.path.clone(),
                                        editor.get_content(),
                                    ));
                                }
                            }
                        }
                        fs.view.preview = None;
                    }
                }

                app.save_current_view_prefs();
                app.core.current_view = CurrentView::Files;
                app.load_view_prefs(CurrentView::Files);
                app.editor_global.editor_state = None;
                app.core.input.clear(); // Ensure no stray inputs remain
                                   // Increase shield to catch escape sequences
                app.set_input_shield(150);
                // Force a refresh to prevent "path display" glitches or empty lists
                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                return true;
            }
            _ => {}
        }
    } else {
        app.core.mode = AppMode::Normal;
        app.core.input.clear();
        app.selection.rename_selected = false;
        return true;
    }
    false
}

fn handle_general_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    panes_needing_refresh: &mut HashSet<usize>,
) -> bool {
    let column = me.column;
    let row = me.row;
    let (w, _) = app.core.terminal_size;
    app.core.mouse_pos = (column, row);

    if let MouseEventKind::Down(MouseButton::Middle) = me.kind {}

    // 1. Sidebar Resizing
    if app.mouse.is_resizing_sidebar {
        match me.kind {
            MouseEventKind::Drag(_) | MouseEventKind::Moved => {
                app.sidebar.sidebar_width_percent = (column as f32 / w as f32 * 100.0) as u16;
                app.sidebar.sidebar_width_percent = app.sidebar.sidebar_width_percent.clamp(5, 50);
                return true;
            }
            MouseEventKind::Up(_) => {
                app.mouse.is_resizing_sidebar = false;
                crate::config::save_state_quiet(app);
                return true;
            }
            _ => {}
        }
    }

    // 2. View-specific routing
    if app.core.current_view == CurrentView::Processes {
        return monitor::handle_monitor_mouse(me, app, event_tx);
    }
    if app.core.current_view == CurrentView::Git {
        return git::handle_git_mouse(me, app, event_tx);
    }
    if app.core.current_view == CurrentView::Commit {
        return editor::handle_editor_mouse(me, app, event_tx);
    }

    // 3. Header Icons (Row 0)
    if row == 0 {
        if let MouseEventKind::Down(_) = me.kind {
            if let Some((_, action_id)) = app.layout.header_icon_bounds
                .iter()
                .find(|(r, _)| column >= r.x && column < r.x + r.width && row == r.y)
            {
                match action_id.as_str() {
                    "back" => {
                        crate::event_helpers::navigate_back(app);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                    "forward" => {
                        crate::event_helpers::navigate_forward(app);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                    }
                    "split" => {
                        app.toggle_split();
                        app.save_current_view_prefs();
                        crate::config::save_state_quiet(app);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(0));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(1));
                    }
                    "burger" => {
                        app.save_current_view_prefs();
                        app.core.mode = AppMode::Settings;
                        app.settings.settings_scroll = 0;
                    }
                    "monitor" => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::SystemMonitor);
                    }
                    "git" => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::GitHistory);
                    }
                    "project" => {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Editor);
                    }
                    _ => {}
                }
                app.sidebar.sidebar_focus = false;
                return true;
            }
        }
        // Hover
        if let Some((_, id)) = app.layout.header_icon_bounds
            .iter()
            .find(|(r, _)| r.contains(ratatui::layout::Position { x: column, y: row }))
        {
            app.layout.hovered_header_icon = Some(id.clone());
        } else {
            app.layout.hovered_header_icon = None;
        }
    }

    // 4. Tabs
    if let Some((_, p_idx, t_idx)) = app.layout.tab_bounds
        .iter()
        .find(|(r, _, _)| r.contains(ratatui::layout::Position { x: column, y: row }))
        .cloned()
    {
        match me.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(p) = app.panes.get_mut(p_idx) {
                    p.active_tab_index = t_idx;
                    app.focused_pane_index = p_idx;
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(p_idx));
                }
                app.sidebar.sidebar_focus = false;
                return true;
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if let Some(p) = app.panes.get_mut(p_idx) {
                    if p.tabs.len() > 1 {
                        p.tabs.remove(t_idx);
                        if p.active_tab_index >= p.tabs.len() {
                            p.active_tab_index = p.tabs.len() - 1;
                        }
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(p_idx));
                    }
                }
                return true;
            }
            _ => {}
        }
    }

    // 5. Sidebar vs Panes
    let sw = app.sidebar_width();
    if app.core.current_view == CurrentView::Editor
        && matches!(me.kind, MouseEventKind::Down(_))
        && column >= sw
    {
        let pane_count = app.panes.len();
        if pane_count > 0 {
            let content_w = w.saturating_sub(sw);
            let pane_w = content_w / pane_count as u16;
            if pane_w > 0 {
                let mut pane_idx = (column.saturating_sub(sw) / pane_w) as usize;
                if pane_idx >= pane_count {
                    pane_idx = pane_count - 1;
                }
                app.focused_pane_index = pane_idx;
                app.sidebar.sidebar_focus = false;
                app.mouse.mouse_click_pos = (column, row);
            }
        }
    }
    // Sidebar Resizing check (MUST BE LEFT CLICK ONLY)
    // Check this BEFORE routing to sidebar mouse, so clicks on the sidebar's right border start resize
    if let MouseEventKind::Down(MouseButton::Left) = me.kind {
        if column >= sw.saturating_sub(1) && column <= sw {
            app.mouse.is_resizing_sidebar = true;
            return true;
        }
    }

    if column < sw {
        handle_sidebar_mouse(me, app, event_tx)
    } else {

        let is_editor_mode = matches!(
            app.core.mode,
            AppMode::Editor
                | AppMode::Viewer
                | AppMode::EditorSearch
                | AppMode::EditorReplace
                | AppMode::EditorGoToLine
        );
        if app.core.current_view == CurrentView::Editor || is_editor_mode {
            editor::handle_editor_mouse(me, app, event_tx)
        } else {
            file_manager::handle_file_mouse(me, app, event_tx, panes_needing_refresh)
        }
    }
}

fn handle_sidebar_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let column = me.column;
    let row = me.row;

    match me.kind {
        MouseEventKind::Down(button) => {
            app.drag.is_dragging = false;
            app.drag.hovered_drop_target = None;
            app.drag.drag_source = None;
            app.sidebar.sidebar_focus = true;
            if button == MouseButton::Left {
                app.drag.drag_start_pos = Some((column, row));
            }
            if let Some(b) = app.sidebar.sidebar_bounds.iter().find(|b| b.y == row).cloned() {
                app.sidebar.sidebar_index = b.index;
                match button {
                    MouseButton::Left => match &b.target {
                        SidebarTarget::Header(name) if name == "REMOTES" => {
                            app.core.mode = AppMode::ImportServers;
                            app.core.input.clear();
                        }
                        SidebarTarget::Favorite(path) => {
                            if let Some(fs) = app.current_file_state_mut() {
                                fs.nav.current_path = path.clone();
                                fs.list.selection.clear();
                                crate::event_helpers::push_history(fs, path.clone());
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                            }
                        }
                        SidebarTarget::Remote(idx) => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::ConnectToRemote(app.focused_pane_index, *idx));
                        }
                        SidebarTarget::Project(path) => {
                            if path.is_dir() {
                                let path_ref = path.clone();
                                let clicked_arrow = b.arrow_end_x > 0 && column < b.arrow_end_x;
                                let was_expanded = app.sidebar.tree_expanded_folders.contains(&path_ref);

                                if clicked_arrow {
                                    // Arrow click: toggle expand/collapse only
                                    if was_expanded {
                                        app.sidebar.tree_expanded_folders.remove(&path_ref);
                                    } else {
                                        app.sidebar.tree_expanded_folders.insert(path_ref.clone());
                                    }
                                } else {
                                    // Name click: navigate to folder only (Dolphin-style — no auto-expand)
                                    if let Some(fs) = app.current_file_state_mut() {
                                        fs.nav.current_path = path_ref.clone();
                                        fs.list.files.clear();
                                        fs.list.tree_file_depths.clear();
                                        fs.list.metadata.clear();
                                        fs.list.local_count = 0;
                                        fs.list.selection.selected = Some(0);
                                        fs.list.selection.anchor = Some(0);
                                        fs.list.selection.clear_multi();
                                        crate::event_helpers::push_history(fs, path_ref.clone());
                                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(
                                            app.focused_pane_index,
                                        ));
                                    }
                                }
                                app.sidebar.sidebar_focus = false;
                            } else {
                                let target_pane = {
                                    let pane_count = app.panes.len();
                                    if pane_count <= 1 {
                                        0
                                    } else {
                                        let sidebar_w = app.sidebar_width();
                                        let content_w =
                                            app.core.terminal_size.0.saturating_sub(sidebar_w);
                                        let pane_w = content_w / pane_count as u16;
                                        if pane_w == 0 {
                                            app.focused_pane_index.min(pane_count - 1)
                                        } else if app.mouse.mouse_click_pos.0 >= sidebar_w {
                                            ((app.mouse.mouse_click_pos.0.saturating_sub(sidebar_w)
                                                / pane_w)
                                                as usize)
                                                .min(pane_count - 1)
                                        } else {
                                            app.focused_pane_index.min(pane_count - 1)
                                        }
                                    }
                                };
                                app.focused_pane_index = target_pane;
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                                    target_pane,
                                    path.clone(),
                                ));
                                app.sidebar.sidebar_focus = false;
                            }
                        }
                        _ => {}
                    },
                    MouseButton::Right => {
                        if let SidebarTarget::Favorite(path) = &b.target {
                            let target = ContextMenuTarget::SidebarFavorite(path.clone());
                            let actions =
                                crate::event_helpers::get_context_menu_actions(&target, app);
                            app.core.mode = AppMode::ContextMenu {
                                x: column,
                                y: row,
                                target,
                                actions,
                                selected_index: None,
                            };
                        }
                    }
                    _ => {}
                }
                if let SidebarTarget::Favorite(ref p) = b.target {
                    if button == MouseButton::Left {
                        app.drag.drag_source = Some(p.clone());
                    }
                }
            }
            true
        }
        MouseEventKind::Up(_) => {
            if let Some(target) = app.drag.hovered_drop_target.take() {
                if let Some(source_path) = app.drag.drag_source.take() {
                    match target {
                        DropTarget::ReorderFavorite(target_idx) => {
                            // Find source index
                            if let Some(source_idx) =
                                app.nav.starred.iter().position(|p| p == &source_path)
                            {
                                // Bounds check to prevent crash
                                if target_idx < app.nav.starred.len() && source_idx != target_idx {
                                    let item = app.nav.starred.remove(source_idx);
                                    // After removal, if source was before target, indices shift down
                                    // Fix: When dragging DOWN (source < target), we want to insert at target_idx
                                    // because items shifted. That places it AFTER the original target item (which moved up).
                                    // When dragging UP (source > target), we want to insert at target_idx
                                    // which places it BEFORE the target item.
                                    let insert_idx = target_idx;

                                    // Ensure we don't exceed bounds
                                    let insert_idx = insert_idx.min(app.nav.starred.len());
                                    app.nav.starred.insert(insert_idx, item);
                                    crate::config::save_state_quiet(app);
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                                }
                            }
                        }
                        DropTarget::Favorites => {
                            // Add folder to favorites when dropped on FAVORITES header
                            if source_path.is_dir() && !app.nav.starred.contains(&source_path) {
                                app.nav.starred.push(source_path.clone());
                                crate::config::save_state_quiet(app);
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                    "Added to favorites: {}",
                                    source_path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                )));
                            }
                        }
                        _ => {} // Handle other DropTarget variants
                    }
                }
            }
            app.drag.is_dragging = false;
            app.drag.drag_source = None;
            app.drag.hovered_drop_target = None;
            true
        }
        MouseEventKind::Drag(_) => {
            if let Some((sx, sy)) = app.drag.drag_start_pos {
                let dist_sq =
                    (column as f32 - sx as f32).powi(2) + (row as f32 - sy as f32).powi(2);
                if dist_sq >= 4.0
                    && !app.drag.is_dragging {
                        app.drag.is_dragging = true;
                    }
            }
            // Update hovered drop target during drag for visual feedback
            if app.drag.is_dragging {
                let prev_target = app.drag.hovered_drop_target.clone();
                app.drag.hovered_drop_target = None;
                // Find what sidebar item we're hovering over
                for bound in &app.sidebar.sidebar_bounds {
                    if bound.y == row {
                        match &bound.target {
                            SidebarTarget::Favorite(ref _path) => {
                                // Find the favorite index from its position in starred
                                if let Some(fav_idx) = app.nav.starred.iter().position(|p| {
                                    if let SidebarTarget::Favorite(ref bp) = bound.target {
                                        p == bp
                                    } else {
                                        false
                                    }
                                }) {
                                    app.drag.hovered_drop_target =
                                        Some(DropTarget::ReorderFavorite(fav_idx));
                                }
                            }
                            SidebarTarget::Header(name) if name == "FAVORITES" => {
                                // Dragging over FAVORITES header - allow adding to favorites
                                app.drag.hovered_drop_target = Some(DropTarget::Favorites);
                            }
                            _ => {}
                        }
                        break;
                    }
                }
                if app.drag.hovered_drop_target != prev_target {
                    return true;
                }
                // Keep repainting while dragging to move drag ghost with cursor.
                return true;
            }
            false
        }
        MouseEventKind::ScrollUp => {
            if app.sidebar.sidebar_scroll_offset > 0 {
                app.sidebar.sidebar_scroll_offset -= 1;
                return true;
            }
            false
        }
        MouseEventKind::ScrollDown => {
            // Allow scrolling even beyond current bounds; draw_sidebar will clamp
            app.sidebar.sidebar_scroll_offset += 1;
            true
        }
        MouseEventKind::Moved => {
            if app.drag.is_dragging {
                app.drag.is_dragging = false;
                app.drag.drag_source = None;
                app.drag.drag_start_pos = None;
                app.drag.hovered_drop_target = None;
            }
            app.drag.drag_start_pos = None;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{CurrentView, SidebarBounds, SidebarTarget};
    use dracon_terminal_engine::contracts::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use dracon_terminal_engine::compositor::engine::TilePlacement;
    use tokio::sync::mpsc;

    fn test_app() -> App {
        let queue: Arc<Mutex<Vec<TilePlacement>>> = Arc::new(Mutex::new(Vec::new()));
        App::new(queue)
    }

    #[test]
    fn esc_exits_git_view_to_files() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.current_view = CurrentView::Git;
        app.core.mode = AppMode::Normal;

        let mut refresh = HashSet::new();
        let changed = handle_event(
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }),
            &mut app,
            tx,
            &mut refresh,
        );

        assert!(changed);
        assert_eq!(app.core.current_view, CurrentView::Files);
        match rx.try_recv() {
            Ok(AppEvent::RefreshFiles(_)) => {}
            other => panic!("expected RefreshFiles event, got {:?}", other),
        }
    }

    #[test]
    fn editor_sidebar_open_targets_last_clicked_pane() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.current_view = CurrentView::Editor;
        app.core.mode = AppMode::Normal;
        app.core.terminal_size = (120, 40);
        app.apply_split_mode(true);
        app.focused_pane_index = 0;
        app.mouse.mouse_click_pos = (90, 10); // right pane side
        let test_path = PathBuf::from("/tmp/tiles_editor_sidebar_target.txt");
        app.sidebar.sidebar_bounds.push(SidebarBounds {
            y: 5,
            index: 0,
            target: SidebarTarget::Project(test_path.clone()),
            ..Default::default()
        });

        let handled = handle_sidebar_mouse(
            &dracon_terminal_engine::contracts::MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 2,
                row: 5,
                modifiers: KeyModifiers::empty(),
            },
            &mut app,
            &tx,
        );

        assert!(handled);
        assert_eq!(app.focused_pane_index, 1);

        match rx.try_recv() {
            Ok(AppEvent::PreviewRequested(pane_idx, path)) => {
                assert_eq!(pane_idx, 1);
                assert_eq!(path, test_path);
            }
            other => panic!("expected PreviewRequested event, got {:?}", other),
        }
    }

    #[test]
    fn esc_from_commit_view_returns_to_git() {
        let (tx, _rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.current_view = CurrentView::Commit;
        app.core.mode = AppMode::Viewer;

        let mut refresh = HashSet::new();
        let changed = handle_event(
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }),
            &mut app,
            tx,
            &mut refresh,
        );

        assert!(changed);
        assert_eq!(app.core.current_view, CurrentView::Git);
    }

    #[test]
    fn sidebar_project_directory_click_opens_folder() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.current_view = CurrentView::Files;
        app.core.mode = AppMode::Normal;
        app.core.terminal_size = (120, 40);
        app.sidebar.show_sidebar = true;
        app.sidebar.sidebar_focus = true;
        let project_dir = std::env::temp_dir().join("tiles-sidebar-open-test");
        let _ = std::fs::create_dir_all(&project_dir);
        app.sidebar.sidebar_bounds.push(SidebarBounds {
            y: 4,
            index: 0,
            target: SidebarTarget::Project(project_dir.clone()),
            ..Default::default()
        });

        let handled = handle_sidebar_mouse(
            &dracon_terminal_engine::contracts::MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 2,
                row: 4,
                modifiers: KeyModifiers::empty(),
            },
            &mut app,
            &tx,
        );

        assert!(handled);
        assert_eq!(
            app.current_file_state().map(|fs| fs.nav.current_path.clone()),
            Some(project_dir.clone())
        );

        match rx.try_recv() {
            Ok(AppEvent::RefreshFiles(_)) => {}
            other => panic!("expected RefreshFiles event, got {:?}", other),
        }
    }
}
