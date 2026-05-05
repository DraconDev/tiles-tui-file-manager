use crate::app::{App, AppEvent, AppMode, MonitorSubview};
use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, MouseButton, MouseEventKind};
use tokio::sync::mpsc;

/// Helper to set process selection consistently.
/// Updates both `process_table_state` and `process_selected_idx` atomically.
fn set_process_selection(app: &mut App, idx: Option<usize>) {
    app.process_table_state.select(idx);
    app.process_selected_idx = idx;
}

pub fn handle_monitor_events(
    evt: &Event,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    // Handle process search input mode
    if matches!(app.mode, AppMode::ProcessSearch) {
        return handle_process_search_input(evt, app);
    }

    // Handle kill confirmation modal
    if matches!(app.mode, AppMode::KillProcessConfirm(_, _)) {
        return handle_kill_confirm_input(evt, app, event_tx);
    }

    if let Event::Key(key) = evt {
        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Char('1') => {
                    app.monitor_subview = MonitorSubview::Overview;
                    return true;
                }
                KeyCode::Char('2') => {
                    app.monitor_subview = MonitorSubview::Processes;
                    return true;
                }
                KeyCode::Char('3') => {
                    app.monitor_subview = MonitorSubview::Cpu;
                    return true;
                }
                KeyCode::Char('4') => {
                    app.monitor_subview = MonitorSubview::Memory;
                    return true;
                }
                KeyCode::Char('5') => {
                    app.monitor_subview = MonitorSubview::Disk;
                    return true;
                }
                KeyCode::Char('6') => {
                    app.monitor_subview = MonitorSubview::Network;
                    return true;
                }
                KeyCode::Up => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        let new_idx = app.process_table_state
                            .selected()
                            .map(|s| s.saturating_sub(1))
                            .or(Some(0));
                        set_process_selection(app, new_idx);
                        return true;
                    }
                }
                KeyCode::Down => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        let len = app.system_state.processes.len();
                        let new_idx = app.process_table_state
                            .selected()
                            .map(|s| (s + 1).min(len.saturating_sub(1)))
                            .or(Some(0));
                        set_process_selection(app, new_idx);
                        return true;
                    }
                }
                KeyCode::Char('k') => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        if let Some(idx) = app.process_selected_idx {
                            if let Some(p) = app.system_state.processes.get(idx) {
                                app.mode = AppMode::KillProcessConfirm(p.pid, p.name.clone());
                            }
                        }
                        return true;
                    }
                }
                KeyCode::Char('c') => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        if let Some(idx) = app.process_selected_idx {
                            if let Some(p) = app.system_state.processes.get(idx) {
                                let pid_str = p.pid.to_string();
                                crate::event_helpers::copy_text_to_clipboard_async(pid_str.clone());
                                let _ = crate::app::try_send_event(
                                    event_tx,
                                    AppEvent::StatusMsg(format!("Copied PID {} to clipboard", pid_str)),
                                );
                            }
                        }
                        return true;
                    }
                }
                KeyCode::Char('/') => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        app.mode = AppMode::ProcessSearch;
                        app.input.clear();
                        app.input.value.clone_from(&app.process_search_filter);
                        return true;
                    }
                }
                KeyCode::Enter => {
                    if app.monitor_subview == MonitorSubview::Processes {
                        // Could open process details here in future
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

fn handle_process_search_input(evt: &Event, app: &mut App) -> bool {
    match evt {
        Event::Key(key) => match key.code {
            KeyCode::Esc => {
                app.mode = AppMode::Normal;
                return true;
            }
            KeyCode::Enter => {
                app.process_search_filter = app.input.value.clone();
                app.mode = AppMode::Normal;
                return true;
            }
            KeyCode::Backspace => {
                app.input.handle_event(&crossterm::event::Event::Key(
                    crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Backspace),
                ));
                app.process_search_filter = app.input.value.clone();
                return true;
            }
            KeyCode::Char(c) => {
                app.input.handle_event(&crossterm::event::Event::Key(
                    crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char(c)),
                ));
                app.process_search_filter = app.input.value.clone();
                return true;
            }
            _ => return true,
        },
        _ => true,
    }
}

fn handle_kill_confirm_input(
    evt: &Event,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    if let Event::Key(key) = evt {
        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    if let AppMode::KillProcessConfirm(pid, _) = app.mode.clone() {
                        let _ = crate::app::try_send_event(event_tx, AppEvent::KillProcess(pid));
                        app.mode = AppMode::Normal;
                    }
                    return true;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    app.mode = AppMode::Normal;
                    return true;
                }
                _ => return true,
            }
        }
    }
    true
}

pub fn handle_monitor_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let column = me.column;
    let row = me.row;

    // Handle kill confirmation modal mouse clicks
    if matches!(app.mode, AppMode::KillProcessConfirm(_, _)) {
        if let MouseEventKind::Down(button) = me.kind {
            if button == MouseButton::Left {
                // Check YES button (at x=5, width=9 relative to modal inner)
                // Modal is centered, 50x12. Need to calculate actual positions.
                let area = crate::ui::modals::centered_rect(50, 12, app.terminal_size.into());
                let inner = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
                let button_y = inner.y + inner.height.saturating_sub(2);
                let yes_x = inner.x + 5;
                let no_x = inner.x + 25;

                if column >= yes_x && column < yes_x + 9 && row == button_y {
                    if let AppMode::KillProcessConfirm(pid, _) = app.mode.clone() {
                        let _ = crate::app::try_send_event(_event_tx, AppEvent::KillProcess(pid));
                        app.mode = AppMode::Normal;
                    }
                    return true;
                }
                if column >= no_x && column < no_x + 8 && row == button_y {
                    app.mode = AppMode::Normal;
                    return true;
                }
            }
        }
        return true; // Block other mouse events while modal is open
    }

    match me.kind {
        MouseEventKind::Down(button) => {
            // Subview tabs
            for (rect, sv) in &app.monitor_subview_bounds {
                if rect.contains(ratatui::layout::Position { x: column, y: row }) {
                    app.monitor_subview = *sv;
                    return true;
                }
            }

            if app.monitor_subview == MonitorSubview::Processes {
                // Column header click to sort
                if row == 1 {
                    for (rect, col) in &app.process_column_bounds {
                        if column >= rect.x && column < rect.x + rect.width {
                            if app.process_sort_col == *col {
                                app.process_sort_asc = !app.process_sort_asc;
                            } else {
                                app.process_sort_col = *col;
                                app.process_sort_asc = false;
                            }
                            app.apply_process_sort();
                            return true;
                        }
                    }
                }

                // Process row click to select
                if row >= 3 {
                    let offset = app.process_table_state.offset();
                    let rel_row = row.saturating_sub(3) as usize;
                    let idx = offset + rel_row;
                    if idx < app.system_state.processes.len() {
                        set_process_selection(app, Some(idx));

                        if button == MouseButton::Right {
                            if let Some(p) = app.system_state.processes.get(idx) {
                                app.mode = AppMode::KillProcessConfirm(p.pid, p.name.clone());
                            }
                            return true;
                        }
                        return true;
                    }
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if app.monitor_subview == MonitorSubview::Processes {
                let len = app.system_state.processes.len();
                let current = app.process_table_state.selected().unwrap_or(0);
                let new_idx = (current + 3).min(len.saturating_sub(1));
                set_process_selection(app, Some(new_idx));
                return true;
            }
        }
        MouseEventKind::ScrollUp => {
            if app.monitor_subview == MonitorSubview::Processes {
                let current = app.process_table_state.selected().unwrap_or(0);
                let new_idx = current.saturating_sub(3);
                set_process_selection(app, Some(new_idx));
                return true;
            }
        }
        _ => {}
    }
    false
}
