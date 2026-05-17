#![allow(unused_imports)]

//! Editor modal key handlers — replace, search, goto.
//! Extracted from events/modals.rs (Phase 4).

use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppEvent, AppMode};
use crate::ui::theme as theme;
use tokio::sync::mpsc;

pub fn handle_editor_replace_keys(
    key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.core.mode = app.core.previous_mode.clone();
            app.core.input.clear();
            app.editor_global.replace_buffer.clear();
            true
        }
        KeyCode::Tab | KeyCode::Enter => {
            if app.editor_global.replace_buffer.is_empty() {
                app.editor_global.replace_buffer = app.core.input.value.clone();
                app.core.input.clear();
                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                    "Replace '{}' with: (Enter: next, ^Enter: all)",
                    app.editor_global.replace_buffer
                )));
            } else {
                let replace_term = app.core.input.value.clone();
                let find_term = app.editor_global.replace_buffer.clone();
                let is_all = key.modifiers.contains(KeyModifiers::CONTROL);

                if let Some(preview) = &mut app.editor_global.editor_state {
                    if let Some(editor) = &mut preview.editor {
                        editor.push_history();
                        if is_all {
                            editor.replace_all(&find_term, &replace_term);
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Replaced all '{}' with '{}'",
                                find_term, replace_term
                            )));
                        } else {
                            editor.replace_next(&find_term, &replace_term);
                            let (w, h) = app.core.terminal_size;
                            editor.ensure_cursor_centered(ratatui::layout::Rect::new(
                                1,
                                1,
                                w.saturating_sub(2),
                                h.saturating_sub(2),
                            ));
                        }
                    }
                }
                let focused_idx = app.focused_pane_index;
                if let Some(pane) = app.panes.get_mut(focused_idx) {
                    if let Some(fs) = pane.current_state_mut() {
                        if let Some(preview) = &mut fs.view.preview {
                            if let Some(editor) = &mut preview.editor {
                                editor.push_history();
                                if is_all {
                                    editor.replace_all(&find_term, &replace_term);
                                } else {
                                    editor.replace_next(&find_term, &replace_term);
                                }
                            }
                        }
                    }
                }
                app.core.mode = app.core.previous_mode.clone();
                app.core.input.clear();
                app.editor_global.replace_buffer.clear();
            }
            true
        }
        _ => {
            let res = app.core.input
                .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt));
            if res && app.editor_global.replace_buffer.is_empty() && app.core.input.value.is_empty() {
                app.core.mode = app.core.previous_mode.clone();
                app.core.input.clear();
                app.editor_global.replace_buffer.clear();
            }
            res
        }
    }
}

pub fn handle_editor_search_keys(
    _key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    match _key.code {
        KeyCode::Esc | KeyCode::Enter => {
            let clear_filter = |ed: &mut dracon_terminal_engine::widgets::TextEditor| ed.set_filter("");
            if let Some(preview) = &mut app.editor_global.editor_state {
                if let Some(editor) = &mut preview.editor {
                    clear_filter(editor);
                }
            }
            if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                if let Some(fs) = pane.current_state_mut() {
                    if let Some(preview) = &mut fs.view.preview {
                        if let Some(editor) = &mut preview.editor {
                            clear_filter(editor);
                        }
                    }
                }
            }
            if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                if let Some(fs) = pane.current_state_mut() {
                    if let Some(preview) = &mut fs.view.preview {
                        if let Some(editor) = &mut preview.editor {
                            editor.handle_event(
                                &dracon_terminal_engine::input::mapping::to_runtime_event(evt),
                                ratatui::layout::Rect::new(0, 0, 100, 100),
                            );
                        }
                    }
                }
            }
            if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                if let Some(fs) = pane.current_state_mut() {
                    if let Some(preview) = &mut fs.view.preview {
                        if let Some(editor) = &mut preview.editor {
                            editor.set_filter("");
                        }
                    }
                }
            }
            app.core.mode = app.core.previous_mode.clone();
            app.core.input.clear();
            true
        }
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
            if let Some(preview) = &mut app.editor_global.editor_state {
                if let Some(editor) = &mut preview.editor {
                    editor.handle_event(
                        &dracon_terminal_engine::input::mapping::to_runtime_event(evt),
                        ratatui::layout::Rect::new(
                            1,
                            1,
                            app.core.terminal_size.0.saturating_sub(2),
                            app.core.terminal_size.1.saturating_sub(2),
                        ),
                    );
                }
            }
            if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                if let Some(fs) = pane.current_state_mut() {
                    if let Some(preview) = &mut fs.view.preview {
                        if let Some(editor) = &mut preview.editor {
                            editor.handle_event(
                                &dracon_terminal_engine::input::mapping::to_runtime_event(evt),
                                ratatui::layout::Rect::new(0, 0, 100, 100),
                            );
                        }
                    }
                }
            }
            true
        }
        _ => {
            let handled = app.core.input
                .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt));
            if handled {
                let filter = app.core.input.value.clone();
                if filter.is_empty() {
                    app.core.mode = app.core.previous_mode.clone();
                    app.core.input.clear();
                    return true;
                }
                if let Some(preview) = &mut app.editor_global.editor_state {
                    if let Some(editor) = &mut preview.editor {
                        editor.set_filter(&filter);
                    }
                }
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(fs) = pane.current_state_mut() {
                        if let Some(preview) = &mut fs.view.preview {
                            if let Some(editor) = &mut preview.editor {
                                editor.set_filter(&filter);
                            }
                        }
                    }
                }
            }
            handled
        }
    }
}

pub fn handle_editor_goto_keys(
    _key: &dracon_terminal_engine::contracts::KeyEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
    evt: &Event,
) -> bool {
    match _key.code {
        KeyCode::Esc => {
            app.core.mode = app.core.previous_mode.clone();
            app.core.input.clear();
            true
        }
        KeyCode::Enter => {
            if let Ok(line_num) = app.core.input.value.parse::<usize>() {
                let target = line_num.saturating_sub(1);
                if let Some(preview) = &mut app.editor_global.editor_state {
                    if let Some(editor) = &mut preview.editor {
                        editor.cursor_row =
                            std::cmp::min(target, editor.lines.len().saturating_sub(1));
                        editor.cursor_col = 0;
                    }
                }
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(fs) = pane.current_state_mut() {
                        if let Some(preview) = &mut fs.view.preview {
                            if let Some(editor) = &mut preview.editor {
                                editor.cursor_row =
                                    std::cmp::min(target, editor.lines.len().saturating_sub(1));
                                editor.cursor_col = 0;
                            }
                        }
                    }
                }
            }
            app.core.mode = app.core.previous_mode.clone();
            app.core.input.clear();
            true
        }
        _ => app.core.input
            .handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt)),
    }
}
