#![allow(clippy::needless_borrow, clippy::collapsible_match)]

use crate::app::{App, AppEvent, MonitorSubview};
use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode};
use tokio::sync::mpsc;

pub fn handle_monitor_events(
    evt: &Event,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    if let Event::Key(key) = evt {
        if key.modifiers.is_empty() {
            let has_table = matches!(
                app.monitor_subview,
                MonitorSubview::Processes | MonitorSubview::Applications
            );
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
                    app.monitor_subview = MonitorSubview::Applications;
                    return true;
                }
                KeyCode::Up if has_table => {
                    app.process_table_state.select(
                        app.process_table_state
                            .selected()
                            .map(|s| s.saturating_sub(1))
                            .or(Some(0)),
                    );
                    return true;
                }
                KeyCode::Down if has_table => {
                    let len = app.system_state.processes.len();
                    app.process_table_state.select(
                        app.process_table_state
                            .selected()
                            .map(|s| (s + 1).min(len.saturating_sub(1)))
                            .or(Some(0)),
                    );
                    return true;
                }
                KeyCode::Up if app.monitor_subview == MonitorSubview::Overview => {
                    app.overview_scroll_offset = app.overview_scroll_offset.saturating_sub(1);
                    return true;
                }
                KeyCode::Down if app.monitor_subview == MonitorSubview::Overview => {
                    app.overview_scroll_offset = app.overview_scroll_offset.saturating_add(1);
                    return true;
                }
                KeyCode::Char('k') if has_table => {
                    if app.process_table_state.selected().is_some() {
                        if let Some(p) = app.system_state.processes.get(
                            app.process_table_state.selected().unwrap(),
                        ) {
                            let _ = crate::app::try_send_event(
                                &event_tx,
                                AppEvent::KillProcess(p.pid),
                            );
                        }
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
    if let dracon_terminal_engine::contracts::MouseEventKind::Down(_) = me.kind {
        for (rect, sv) in &app.monitor_subview_bounds {
            if rect.contains(ratatui::layout::Position {
                x: me.column,
                y: me.row,
            }) {
                app.monitor_subview = *sv;
                return true;
            }
        }
        for (rect, col) in &app.process_column_bounds {
            if rect.contains(ratatui::layout::Position {
                x: me.column,
                y: me.row,
            }) {
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
    false
}
