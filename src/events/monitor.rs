use crate::app::{App, AppEvent, AppMode, MonitorSubview};
use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, MouseButton, MouseEventKind};
use tokio::sync::mpsc;

pub fn handle_monitor_events(
    evt: &Event,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
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
                        app.process_table_state.select(new_idx);
                        app.process_selected_idx = new_idx;
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
                        app.process_table_state.select(new_idx);
                        app.process_selected_idx = new_idx;
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
                KeyCode::Char('y') => {
                    if matches!(app.mode, AppMode::KillProcessConfirm(_, _)) {
                        if let AppMode::KillProcessConfirm(pid, _) = app.mode.clone() {
                            let _ = crate::app::try_send_event(event_tx, AppEvent::KillProcess(pid));
                            app.mode = AppMode::Normal;
                        }
                        return true;
                    }
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    if matches!(app.mode, AppMode::KillProcessConfirm(_, _)) {
                        app.mode = AppMode::Normal;
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

pub fn handle_monitor_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    _event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let column = me.column;
    let row = me.row;

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
                        app.process_selected_idx = Some(idx);
                        app.process_table_state.select(Some(idx));

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
                app.process_table_state.select(Some(new_idx));
                app.process_selected_idx = Some(new_idx);
                return true;
            }
        }
        MouseEventKind::ScrollUp => {
            if app.monitor_subview == MonitorSubview::Processes {
                let current = app.process_table_state.selected().unwrap_or(0);
                let new_idx = current.saturating_sub(3);
                app.process_table_state.select(Some(new_idx));
                app.process_selected_idx = Some(new_idx);
                return true;
            }
        }
        _ => {}
    }
    false
}
