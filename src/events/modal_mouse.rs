#![allow(unused_imports)]

//! Modal mouse event handling.
//! Extracted from events/modals.rs (Phase 4).

use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind, MouseButton};

use crate::app::{App, AppEvent, AppMode, ContextMenuAction, ContextMenuTarget, CurrentView, SettingsSection};
use crate::events::settings_handlers::{cycle_preview_max_mb, open_style_color_input, style_preset_for_index, STYLE_MAX_INDEX};
use crate::state::IconMode;
use crate::ui::theme as theme;
use serde::Deserialize;
use tokio::sync::mpsc;

pub fn handle_modal_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let (w, h) = app.core.terminal_size;
    let column = me.column;
    let row = me.row;

    // Middle-click paste for input modals
    if let MouseEventKind::Down(MouseButton::Middle) = me.kind {
        if matches!(
            app.core.mode,
            AppMode::Rename
                | AppMode::NewFile
                | AppMode::NewFolder
                | AppMode::AddRemote(_)
                | AppMode::ImportServers
                | AppMode::EditorSearch
                | AppMode::EditorReplace
                | AppMode::EditorGoToLine
                | AppMode::StyleColorInput
                | AppMode::ResetSettingsConfirm
        ) {
            if let Some(text) = dracon_terminal_engine::utils::get_primary_selection_text() {
                let pos = app.core.input.cursor_position;
                if pos >= app.core.input.value.len() {
                    app.core.input.value.push_str(&text);
                } else {
                    app.core.input.value.insert_str(pos, &text);
                }
                app.core.input.cursor_position += text.len();
                app.selection.rename_selected = false;
                return true;
            }
        }
    }

    match app.core.mode.clone() {
        AppMode::Highlight => {
            if let MouseEventKind::Down(_) = me.kind {
                let area_w = 34;
                let area_h = 5;
                let area_x = (w.saturating_sub(area_w)) / 2;
                let area_y = (h.saturating_sub(area_h)) / 2;
                if column >= area_x
                    && column < area_x + area_w
                    && row >= area_y
                    && row < area_y + area_h
                {
                    let rel_x = column.saturating_sub(area_x + 3);
                    if row >= area_y + 2 && row <= area_y + 3 {
                        let colors = [1, 2, 3, 4, 5, 6, 0];
                        if let Some(&color_code) = colors.get((rel_x / 4) as usize) {
                            let color = if color_code == 0 {
                                None
                            } else {
                                Some(color_code as u8)
                            };
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
                                    if let Some(col) = color {
                                        app.selection.path_colors.insert(p, col);
                                    } else {
                                        app.selection.path_colors.remove(&p);
                                    }
                                }
                                crate::config::save_state_quiet(app);
                            }
                            app.core.mode = AppMode::Normal;
                        }
                    }
                } else {
                    app.core.mode = AppMode::Normal;
                }
                return true;
            }
        }
        AppMode::ContextMenu {
            x,
            y,
            ref actions,
            ref target,
            ..
        } => {
            let (mw, mh) = (25, actions.len() as u16 + 2);
            let (mut dx, mut dy) = (x, y);
            if dx + mw > w {
                dx = w.saturating_sub(mw);
            }
            if dy + mh > h {
                dy = h.saturating_sub(mh);
            }

            if let MouseEventKind::Down(_) = me.kind {
                if column >= dx && column < dx + mw && row >= dy && row < dy + mh {
                    let rel_y = row.saturating_sub(dy + 1);
                    if row > dy && row < dy + mh - 1 {
                        if let Some(action) = actions.get(rel_y as usize) {
                            if *action != ContextMenuAction::Separator {
                                let prev_mode = app.core.mode.clone();
                                crate::event_helpers::handle_context_menu_action(
                                    action,
                                    target,
                                    app,
                                    event_tx.clone(),
                                );
                                if matches!(app.core.mode, AppMode::Normal) {
                                    // Menu was closed, check if action changed mode
                                } else if matches!(prev_mode, AppMode::ContextMenu { .. }) {
                                    // Action changed mode (like NewFile/NewFolder), keep it
                                } else {
                                    app.core.mode = AppMode::Normal;
                                }
                            }
                        }
                    }
                } else {
                    app.core.mode = AppMode::Normal;
                }
                return true;
            }
            return true; // Consume all mouse events while context menu is open
        }
        AppMode::StyleColorInput => {
            if let MouseEventKind::Down(_) = me.kind {
                let area_w = 64;
                let area_h = 9;
                let area_x = (w.saturating_sub(area_w)) / 2;
                let area_y = (h.saturating_sub(area_h)) / 2;
                let inside = column >= area_x
                    && column < area_x + area_w
                    && row >= area_y
                    && row < area_y + area_h;
                if !inside {
                    app.core.mode = AppMode::Settings;
                    app.core.input.clear();
                }
                return true;
            }
        }
        AppMode::ResetSettingsConfirm => {
            if let MouseEventKind::Down(_) = me.kind {
                let area_w = 56;
                let area_h = 12;
                let area_x = (w.saturating_sub(area_w)) / 2;
                let area_y = (h.saturating_sub(area_h)) / 2;
                let inside = column >= area_x
                    && column < area_x + area_w
                    && row >= area_y
                    && row < area_y + area_h;
                if !inside {
                    app.core.mode = AppMode::Settings;
                    app.core.input.clear();
                }
                return true;
            }
        }
        AppMode::Settings => {
            if let MouseEventKind::Down(_) = me.kind {
                if row == 0 && column >= w.saturating_sub(10) {
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                let inner_x = 1;
                let inner_y = 1;
                if column < inner_x + 20 {
                    let rel_y = row.saturating_sub(inner_y);
                    match rel_y {
                        0 => app.settings.settings_section = SettingsSection::Columns,
                        1 => app.settings.settings_section = SettingsSection::Tabs,
                        2 => app.settings.settings_section = SettingsSection::General,
                        3 => app.settings.settings_section = SettingsSection::Style,
                        4 => app.settings.settings_section = SettingsSection::Remotes,
                        5 => app.settings.settings_section = SettingsSection::Shortcuts,
                        _ => {}
                    }
                    app.settings.settings_index = 0;
                } else {
                    // Right Side Interactions
                    let rel_col = column.saturating_sub(inner_x + 20);
                    let rel_y = row.saturating_sub(inner_y + 1); // +1 assuming block top border

                    match app.settings.settings_section {
                        SettingsSection::General => {
                            // Indices match the drawn list: 0=Version (read-only),
                            // 1=hidden, ..., 8=separator, 9-13=sidebar, 14=Reset.
                            // The table may be scrolled on small terminals; the
                            // persisted offset keeps clicks on the rendered row.
                            let idx = rel_y as usize + app.settings.settings_scroll as usize;
                            if idx < 15 {
                                app.settings.settings_index = idx;
                                match app.settings.settings_index {
                                    0 => {} // Version is read-only
                                    1 => app.settings.default_show_hidden = !app.settings.default_show_hidden,
                                    2 => app.settings.confirm_delete = !app.settings.confirm_delete,
                                    3 => app.settings.smart_date = !app.settings.smart_date,
                                    4 => app.settings.semantic_coloring = !app.settings.semantic_coloring,
                                    5 => app.settings.auto_save = !app.settings.auto_save,
                                    6 => {
                                        app.preview_max_mb =
                                            cycle_preview_max_mb(app.preview_max_mb)
                                    }
                                    7 => {
                                        app.core.icon_mode = match app.core.icon_mode {
                                            IconMode::Nerd => IconMode::Unicode,
                                            IconMode::Unicode => IconMode::ASCII,
                                            IconMode::ASCII => IconMode::Nerd,
                                        }
                                    }
                                    8 => {} // separator, do nothing
                                    9 => app.sidebar.sidebar_folders = !app.sidebar.sidebar_folders,
                                    10 => app.sidebar.sidebar_favorites = !app.sidebar.sidebar_favorites,
                                    11 => app.sidebar.sidebar_recent = !app.sidebar.sidebar_recent,
                                    12 => app.sidebar.sidebar_storage = !app.sidebar.sidebar_storage,
                                    13 => app.sidebar.sidebar_remotes = !app.sidebar.sidebar_remotes,
                                    14 => {
                                        app.core.mode = AppMode::ResetSettingsConfirm;
                                        app.core.input.clear();
                                    }
                                    _ => {}
                                }
                                if app.settings.settings_index != 8 && app.settings.settings_index != 14 && app.settings.settings_index != 0 {
                                    crate::config::save_state_quiet(app);
                                }
                            }
                        }
                        SettingsSection::Columns => {
                            if rel_y < 3 {
                                // Toggle Mode Tabs
                                if rel_col < 15 {
                                    app.settings.settings_target = crate::app::SettingsTarget::SingleMode;
                                } else {
                                    app.settings.settings_target = crate::app::SettingsTarget::SplitMode;
                                }
                            } else {
                                let idx = rel_y.saturating_sub(3) as usize + app.settings.settings_scroll as usize;
                                if idx < 4 {
                                    app.settings.settings_index = idx;
                                    let col = match idx {
                                        0 => crate::app::FileColumn::Size,
                                        1 => crate::app::FileColumn::Modified,
                                        2 => crate::app::FileColumn::Created,
                                        3 => crate::app::FileColumn::Permissions,
                                        _ => crate::app::FileColumn::Size,
                                    };
                                    let target_set = match app.settings.settings_target {
                                        crate::app::SettingsTarget::SingleMode => {
                                            &mut app.layout.single_columns
                                        }
                                        crate::app::SettingsTarget::SplitMode => {
                                            &mut app.layout.split_columns
                                        }
                                    };
                                    if let Some(pos) = target_set.iter().position(|c| c == &col) {
                                        target_set.remove(pos);
                                    } else {
                                        target_set.push(col);
                                    }
                                    crate::config::save_state_quiet(app);
                                }
                            }
                        }
                        SettingsSection::Style => {
                            let idx = rel_y as usize + app.settings.settings_scroll as usize;
                            if idx < STYLE_MAX_INDEX + 1 {
                                app.settings.settings_index = idx;
                                if app.settings.settings_index == 0 {
                                    crate::ui::theme::set_style_settings(
                                        crate::ui::theme::ThemeStyle::preset_legacy_red(),
                                    );
                                    crate::config::save_state_quiet(app);
                                } else if let Some(preset) =
                                    style_preset_for_index(app.settings.settings_index)
                                {
                                    crate::ui::theme::set_style_settings(preset);
                                    crate::config::save_state_quiet(app);
                                } else {
                                    open_style_color_input(app);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                return true;
            }
        }
        AppMode::Delete(_) | AppMode::DeleteFile(_) => {
            if let MouseEventKind::Down(MouseButton::Left) = me.kind {
                let area_w = 40;
                let area_h = 10;
                let area_x = (w.saturating_sub(area_w)) / 2;
                let area_y = (h.saturating_sub(area_h)) / 2;

                let inner_x = area_x + 1;
                let inner_y = area_y + 1;
                let inner_h = area_h - 2;
                let button_y = inner_y + inner_h.saturating_sub(2);

                let is_hit = |bx: u16, len: u16| {
                    column >= inner_x + bx && column < inner_x + bx + len && row == button_y
                };

                if is_hit(5, 9) {
                    // Collect paths to delete
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
                        // Honor trash mode: this click could otherwise
                        // permanently delete when the user chose Trash.
                        let trash = matches!(&app.core.mode, AppMode::Delete(m) if m == "trash");
                        for p in paths {
                            if trash {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::TrashFile(p));
                            } else {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::Delete(p));
                            }
                        }
                    }
                    app.core.mode = AppMode::Normal;
                    app.core.input.clear();
                    return true;
                }

                if is_hit(25, 8) {
                    app.core.mode = AppMode::Normal;
                    app.core.input.clear();
                    return true;
                }
            }
        }
        AppMode::DragDropMenu {
            ref sources,
            ref target,
        } => {
            if let MouseEventKind::Down(MouseButton::Left) = me.kind {
                // Hit-testing must use the same geometry the modal renders
                // with: a fixed 60x9 cell box (see draw_drag_drop_modal), not
                // the 60x20 percentages it used to assume.
                let area = crate::ui::centered_modal(60, 9, ratatui::layout::Rect::new(0, 0, w, h));

                // Block borders take 1 cell each side
                let inner_x = area.x + 1;
                let inner_y = area.y + 1;

                let button_y_offset = if sources.len() == 1 {
                    3
                } else {
                    let display_count = std::cmp::min(sources.len(), 3);
                    let mut offset = 1 + display_count;
                    if sources.len() > 3 {
                        offset += 1;
                    }
                    offset + 2
                };
                let button_y = inner_y + button_y_offset as u16;

                let is_hit = |bx: u16, len: u16| {
                    column >= inner_x + bx && column < inner_x + bx + len && row == button_y
                };

                let sources = sources.clone();
                let target = target.to_path_buf();

                // Button layout: " [C] Copy " (10) + "  " (2) + " [M] Move " (10) + "  " (2) + " [L] Link " (10) + "  " (2) + " [Esc] Cancel " (14)
                if is_hit(0, 10) {
                    for src in &sources {
                        let dest = target.join(src.file_name().unwrap_or_default());
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Copy(src.clone(), dest));
                    }
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                if is_hit(12, 10) {
                    for src in &sources {
                        let dest = target.join(src.file_name().unwrap_or_default());
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Rename(src.clone(), dest));
                    }
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                if is_hit(24, 10) {
                    for src in &sources {
                        let dest = target.join(src.file_name().unwrap_or_default());
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::Symlink(src.clone(), dest));
                    }
                    app.core.mode = AppMode::Normal;
                    return true;
                }
                if is_hit(36, 14) {
                    app.core.mode = AppMode::Normal;
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppMode};
    use crate::events::modals::{handle_settings_keys, STYLE_COLOR_START_INDEX};
    use crate::events::settings_handlers::handle_style_color_input_keys;
    use dracon_terminal_engine::contracts::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use dracon_terminal_engine::compositor::engine::TilePlacement;
    use tokio::sync::mpsc;

    fn test_app() -> App {
        App::new()
    }

    #[test]
    fn drag_drop_link_click_emits_symlink_and_closes_modal() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.terminal_size = (100, 40);

        let src = PathBuf::from("/tmp/src-file.txt");
        let target = PathBuf::from("/tmp/dest-dir");
        app.core.mode = AppMode::DragDropMenu {
            sources: vec![src.clone()],
            target: target.clone(),
        };

        let handled = handle_modal_mouse(
            &MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 46, // Link button region for 100x40 terminal.
                row: 20,
                modifiers: KeyModifiers::empty(),
            },
            &mut app,
            &tx,
        );

        assert!(handled);
        assert!(matches!(app.core.mode, AppMode::Normal));

        match rx.try_recv() {
            Ok(AppEvent::Symlink(from, to)) => {
                assert_eq!(from, src);
                assert_eq!(to, target.join("src-file.txt"));
            }
            other => panic!("expected Symlink event, got {:?}", other),
        }
    }

    #[test]
    fn style_preset_and_custom_color_apply() {
        // Global theme style is shared across tests: serialize.
        let _guard = crate::ui::theme::STYLE_TEST_LOCK.lock().unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut app = test_app();
        app.core.mode = AppMode::Settings;
        app.settings.settings_section = SettingsSection::Style;

        // Apply Cool preset row (index 3 in new order: 1=Legacy Red, 2=Warm, 3=Cool).
        app.settings.settings_index = 3;
        let _ = handle_settings_keys(
            &KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            },
            &mut app,
            &tx,
        );
        let style_after_preset = crate::ui::theme::style_settings();
        assert_eq!(style_after_preset.accent_primary.r, 160);

        // Open first custom color row and apply explicit color.
        app.settings.settings_index = STYLE_COLOR_START_INDEX;
        let _ = handle_settings_keys(
            &KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            },
            &mut app,
            &tx,
        );
        assert!(matches!(app.core.mode, AppMode::StyleColorInput));
        app.core.input.set_value("#112233".to_string());
        let _ = handle_style_color_input_keys(
            &KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            },
            &mut app,
        );
        let style_after_custom = crate::ui::theme::style_settings();
        assert_eq!(style_after_custom.accent_primary.r, 0x11);
        assert_eq!(style_after_custom.accent_primary.g, 0x22);
        assert_eq!(style_after_custom.accent_primary.b, 0x33);
    }
}
