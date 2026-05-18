//! File mouse event handler — extracted from file_manager.rs.
//!
//! Handles all mouse interactions on the file list: click, double-click,
//! drag-and-drop, marquee selection, scroll, and context menu.

use std::time::Duration;

use tokio::sync::mpsc;

use dracon_terminal_engine::contracts::{KeyModifiers, MouseButton, MouseEventKind};

use crate::app::{
    App, AppEvent, AppMode, ContextMenuTarget, DropTarget, FileColumn,
};
use crate::events::file_actions::{is_valid_search_char, is_virtual_divider, open_file_or_navigate};

/// Search debounce interval in milliseconds.
const SEARCH_DEBOUNCE_MS: u64 = 300;

/// Handle mouse events on the file list pane.
#[allow(clippy::too_many_arguments)]
pub fn handle_file_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
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
                        fs.list.files.clear();
                        fs.list.tree_file_depths.clear();
                        fs.list.metadata.clear();
                        fs.list.local_count = 0;
                        fs.list.selection.clear();
                        fs.nav.search_filter.clear();
                        *fs.view.table_state.offset_mut() = 0;
                        crate::nav_helpers::push_history(fs, target_path);
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app.focused_pane_index));
                        app.sidebar.sidebar_focus = false;
                        return true;
                    }

                    // Clicked breadcrumb row but not on a segment:
                    // copy path to clipboard and open path input
                    let path = fs.nav.current_path.to_string_lossy().to_string();
                    crate::nav_helpers::open_path_input(app);
                    crate::clipboard::copy_text_to_clipboard_async(path);
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
                let Some(idx) = crate::events::mouse_helpers::fs_mouse_index(row, app) else {
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
                                    crate::nav_helpers::push_history(&mut nfs, path);
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
                    // Always set marquee_start (for vertical-drag heuristic).
                    // Set drag_source only for Name column clicks (file drag-and-drop).
                    // The Drag handler uses a vertical-drag heuristic: if the user drags
                    // primarily vertically from the Name column, marquee takes over.
                    app.drag.marquee_start = Some((column, row));
                    app.drag.marquee_end = Some((column, row));
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
                        && super::file_actions::is_double_click(app.mouse.mouse_click_pos, app.mouse.mouse_last_click, column, row)
                    {
                        if path.is_dir() {
                            if let Some(fs) = app.current_file_state_mut() {
                                fs.nav.current_path = path.clone();
                                // Clear file list immediately to avoid showing stale
                                // parent directory data while RefreshFiles is async
                                fs.list.files.clear();
                                fs.list.tree_file_depths.clear();
                                fs.list.metadata.clear();
                                fs.list.selection.clear();
                                fs.list.local_count = 0;
                                fs.git.git_cache_until = None;
                                crate::nav_helpers::push_history(fs, path);
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
                match app.drag.hovered_drop_target.take() {
                    Some(DropTarget::Folder(target_path)) => {
                        if let Some(source_path) = app.drag.drag_source.take() {
                            if source_path != target_path {
                                app.core.mode = AppMode::DragDropMenu {
                                    sources: vec![source_path],
                                    target: target_path,
                                };
                            }
                        }
                    }
                    Some(DropTarget::CurrentDir(pane_idx)) => {
                        // Drop into the other pane's current directory
                        if let Some(source_path) = app.drag.drag_source.take() {
                            if let Some(pane) = app.panes.get(pane_idx) {
                                if let Some(fs) = pane.current_state() {
                                    let target_path = fs.nav.current_path.clone();
                                    if source_path != target_path {
                                        app.core.mode = AppMode::DragDropMenu {
                                            sources: vec![source_path],
                                            target: target_path,
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
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
                let Some(idx) = crate::events::mouse_helpers::fs_mouse_index(row, app) else {
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
            // Marquee drag: activate if click was outside Name column,
            // OR if dragging vertically from Name column (user wants selection, not file drag).
            if let Some((sx, sy)) = app.drag.marquee_start {
                app.drag.marquee_end = Some((column, row));
                let dx = (column as f32 - sx as f32).abs();
                let dy = (row as f32 - sy as f32).abs();
                let dist_sq = dx * dx + dy * dy;
                let is_vertical_drag = dy > dx * 2.0 && dy >= 2.0;
                if dist_sq >= 4.0 && !app.drag.is_marquee
                    && (app.drag.drag_source.is_none() || is_vertical_drag)
                {
                    app.drag.is_marquee = true;
                    // Cancel file drag if marquee takes over
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
                if dist_sq >= 1.0
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

                    // File row folder targets (same pane).
                    if app.drag.hovered_drop_target.is_none() && row >= 3 {
                        if let Some(idx) = crate::events::mouse_helpers::fs_mouse_index(row, app) {
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

                    // Cross-pane drop: check other pane for folder targets or current dir
                    if app.drag.hovered_drop_target.is_none() && app.panes.len() > 1 {
                        for (pi, rect) in app.layout.pane_rects.iter().enumerate() {
                            if pi == app.focused_pane_index { continue; }
                            if column >= rect.x && column < rect.x + rect.width
                                && row >= rect.y && row < rect.y + rect.height
                            {
                                if row >= 3 {
                                    // Check if hovering over a folder row in the other pane
                                    if let Some(pane) = app.panes.get(pi) {
                                        if let Some(fs) = pane.current_state() {
                                            let offset = fs.view.table_state.offset();
                                            let file_idx = (row as usize).saturating_sub(3).saturating_add(offset);
                                            if let Some(path) = fs.list.files.get(file_idx) {
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
                                // If no folder found, drop into the other pane's current directory
                                if app.drag.hovered_drop_target.is_none() {
                                    if let Some(pane) = app.panes.get(pi) {
                                        if let Some(fs) = pane.current_state() {
                                            if let Some(src) = &app.drag.drag_source {
                                                if src != &fs.nav.current_path {
                                                    app.drag.hovered_drop_target =
                                                        Some(DropTarget::CurrentDir(pi));
                                                }
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
                let Some(idx) = crate::events::mouse_helpers::fs_mouse_index(row, app) else {
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


#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::events::mouse_helpers::fs_mouse_index;
    use crate::state::DragState;

    fn test_app_with_files() -> App {
        let mut app = App::new();
        let fs = app.current_file_state_mut().unwrap();
        fs.list.files = vec![
            std::path::PathBuf::from("/tmp/a.rs"),
            std::path::PathBuf::from("/tmp/b.rs"),
            std::path::PathBuf::from("/tmp/c.rs"),
        ];
        fs.view.view_height = 20;
        app
    }

    #[test]
    fn fs_mouse_index_first_row_returns_first_file() {
        let app = test_app_with_files();
        let idx = fs_mouse_index(3, &app); // row 3 = first content row (header + breadcrumb + col header)
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn fs_mouse_index_out_of_bounds_returns_none() {
        let app = test_app_with_files();
        let idx = fs_mouse_index(100, &app); // way beyond file list
        assert_eq!(idx, None);
    }

    #[test]
    fn drag_state_marquee_rect_when_not_active() {
        let drag = DragState::default();
        assert!(drag.marquee_rect().is_none());
    }

    #[test]
    fn drag_state_marquee_rect_when_active() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((5, 3));
        drag.marquee_end = Some((10, 8));
        let rect = drag.marquee_rect().unwrap();
        assert_eq!(rect.min_col, 5);
        assert_eq!(rect.max_col, 10);
        assert_eq!(rect.min_row, 3);
        assert_eq!(rect.max_row, 8);
    }

    #[test]
    fn drag_state_marquee_rect_normalized_when_end_before_start() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((10, 8));
        drag.marquee_end = Some((5, 3));
        let rect = drag.marquee_rect().unwrap();
        assert_eq!(rect.min_col, 5);
        assert_eq!(rect.max_col, 10);
        assert_eq!(rect.min_row, 3);
        assert_eq!(rect.max_row, 8);
    }

    #[test]
    fn drag_state_clear_marquee_resets_all() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((5, 3));
        drag.marquee_end = Some((10, 8));
        drag.pending_click_idx = Some(5);
        drag.clear_marquee();
        assert!(!drag.is_marquee);
        assert!(drag.marquee_start.is_none());
        assert!(drag.marquee_end.is_none());
        // clear_marquee doesn't reset pending_click_idx
        assert_eq!(drag.pending_click_idx, Some(5));
    }
}
