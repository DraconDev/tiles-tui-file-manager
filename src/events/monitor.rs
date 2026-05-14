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
                KeyCode::Up => match app.monitor_subview {
                    MonitorSubview::Overview => {
                        app.overview_scroll_offset = app.overview_scroll_offset.saturating_sub(1);
                        return true;
                    }
                    MonitorSubview::Processes | MonitorSubview::Applications => {
                        app.process_table_state.select(
                            app.process_table_state
                                .selected()
                                .map(|s| s.saturating_sub(1))
                                .or(Some(0)),
                        );
                        return true;
                    }
                },
                KeyCode::Down => match app.monitor_subview {
                    MonitorSubview::Overview => {
                        app.overview_scroll_offset = app.overview_scroll_offset.saturating_add(1).min(1000);
                        return true;
                    }
                    MonitorSubview::Processes | MonitorSubview::Applications => {
                        let len = app.system_state.processes.len();
                        app.process_table_state.select(
                            app.process_table_state
                                .selected()
                                .map(|s| (s + 1).min(len.saturating_sub(1)))
                                .or(Some(0)),
                        );
                        return true;
                    }
                },
                KeyCode::Char('k') => {
                    if matches!(app.monitor_subview, MonitorSubview::Processes | MonitorSubview::Applications)
                        && app.process_table_state.selected().is_some()
                    {
                        if let Some(p) = app.system_state.processes.get(
                            app.process_table_state.selected().unwrap(),
                        ) {
                            app.mode = crate::app::AppMode::SignalSelect {
                                pid: p.pid,
                                name: p.name.clone(),
                                selected_index: 1,
                            };
                        }
                        return true;
                    }
                }
                KeyCode::Char('t') => {
                    if matches!(app.monitor_subview, MonitorSubview::Processes | MonitorSubview::Applications) {
                        app.process_tree_view = !app.process_tree_view;
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
    }
    false
}
