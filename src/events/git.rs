use crate::app::{App, AppEvent, AppMode, CurrentView};
use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, MouseButton, MouseEventKind};
use tokio::sync::mpsc;

fn trigger_git_diff_fetch(app: &App, event_tx: &mpsc::Sender<AppEvent>) {
    let pane_idx = app.focused_pane_index;
    let Some(pane) = app.panes.get(pane_idx) else { return };
    let tab_idx = pane.active_tab_index;
    let Some(tab) = pane.tabs.get(tab_idx) else { return };
    let Some(pending_idx) = tab.git_pending_state.selected() else { return };
    let Some(change) = tab.git_pending.get(pending_idx) else { return };
    
    let file_path = change.path.clone();
    let current_dir = tab.current_path.clone();
    let remote_session = tab.remote_session.clone();
    let tx = event_tx.clone();
    
    tokio::spawn(async move {
        let diff = if let Some(remote) = &remote_session {
            match crate::modules::remote::show_file_diff(remote, &current_dir, &file_path) {
                Ok(content) => content,
                Err(e) => format!("Error: {}", e),
            }
        } else {
            match tokio::task::spawn_blocking(move || {
                crate::modules::files::show_file_diff(&current_dir, &file_path)
            }).await {
                Ok(Ok(content)) => content,
                Ok(Err(e)) => format!("Error: {}", e),
                Err(_) => "Error: task failed".to_string(),
            }
        };
        let _ = crate::app::try_send_event(&tx, AppEvent::GitDiffFetched(pane_idx, tab_idx, diff));
    });
}

pub fn handle_git_events(evt: &Event, app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> bool {
    if let CurrentView::Git = app.current_view {
        if let Event::Key(key) = evt {
            if key.modifiers.is_empty() {
                match key.code {
                    KeyCode::Up => {
                        if let Some(fs) = app.current_file_state_mut() {
                            // If pending is focused, navigate pending
                            if fs.git_pending_state.selected().is_some() {
                                let idx = fs.git_pending_state.selected().unwrap_or(0);
                                if idx > 0 {
                                    fs.git_pending_state.select(Some(idx - 1));
                                    trigger_git_diff_fetch(app, event_tx);
                                }
                            } else {
                                // Navigate history
                                let idx = fs.git_history_state.selected().unwrap_or(0);
                                if idx > 0 {
                                    fs.git_history_state.select(Some(idx - 1));
                                }
                            }
                        }
                        return true;
                    }
                    KeyCode::Down => {
                        if let Some(fs) = app.current_file_state_mut() {
                            // If pending is focused, navigate pending
                            if fs.git_pending_state.selected().is_some() {
                                let idx = fs.git_pending_state.selected().unwrap_or(0);
                                let len = fs.git_pending.len();
                                if idx + 1 < len {
                                    fs.git_pending_state.select(Some(idx + 1));
                                    trigger_git_diff_fetch(app, event_tx);
                                }
                            } else {
                                // Navigate history
                                let idx = fs.git_history_state.selected().unwrap_or(0);
                                let len = fs.git_history.len();
                                if idx + 1 < len {
                                    fs.git_history_state.select(Some(idx + 1));
                                }
                            }
                        }
                        return true;
                    }
                    KeyCode::Tab => {
                        if let Some(fs) = app.current_file_state_mut() {
                            if fs.git_pending_state.selected().is_some() {
                                // Switch to history
                                fs.git_pending_state.select(None);
                                fs.git_history_state.select(Some(0));
                            } else if fs.git_pending.is_empty() {
                                // No pending, stay in history but cycle to start
                                fs.git_history_state.select(Some(0));
                            } else {
                                // Switch to pending
                                fs.git_history_state.select(None);
                                fs.git_pending_state.select(Some(0));
                                trigger_git_diff_fetch(app, event_tx);
                            }
                        }
                        return true;
                    }
                    KeyCode::Enter if matches!(app.mode, AppMode::Normal) => {
                        let mut open_preview: Option<std::path::PathBuf> = None;
                        let mut open_commit_view = false;
                        if let Some(fs) = app.current_file_state() {
                            // Priority 1: Pending changes (Diff)
                            if let Some(idx) = fs.git_pending_state.selected() {
                                if let Some(change) = fs.git_pending.get(idx) {
                                    open_preview = Some(std::path::PathBuf::from(format!(
                                        "git-diff://{}",
                                        change.path
                                    )));
                                }
                            }
                            // Priority 2: History (Commit)
                            if open_preview.is_none() {
                                if let Some(idx) = fs.git_history_state.selected() {
                                    if let Some(commit) = fs.git_history.get(idx) {
                                        let hash = commit.hash.clone();
                                        open_preview =
                                            Some(std::path::PathBuf::from(format!("git://{}", hash)));
                                        open_commit_view = true;
                                    }
                                }
                            }
                        }
                        if let Some(path) = open_preview {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(app.focused_pane_index, path));
                            if open_commit_view {
                                app.current_view = CurrentView::Commit;
                                app.mode = AppMode::Viewer;
                                app.sidebar_focus = false;
                            }
                            return true;
                        }
                    }
                    KeyCode::Char('/') if matches!(app.mode, AppMode::Normal) => {
                        app.mode = AppMode::Search;
                        app.input.clear();
                        return true;
                    }
                    KeyCode::Esc if matches!(app.mode, AppMode::Search) => {
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.git_search_filter.clear();
                        }
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return true;
                    }
                    KeyCode::Enter if matches!(app.mode, AppMode::Search) => {
                        let query = app.input.value.clone();
                        if let Some(fs) = app.current_file_state_mut() {
                            fs.git_search_filter = query;
                        }
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return true;
                    }
                    _ => {}
                }
            } else if matches!(app.mode, AppMode::Search) {
                // Handle character input for search
                let res = app.input.handle_event(&dracon_terminal_engine::input::mapping::to_runtime_event(evt));
                return res;
            }
        }
    }
    false
}

pub fn handle_git_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let _column = me.column;
    let row = me.row;

    if let MouseEventKind::Down(button) = me.kind {
        if let Some(fs) = app.current_file_state() {
            let pending_len = fs.git_pending.len();
            let inner_h = app.terminal_size.1.saturating_sub(2);
            let top_h = if pending_len == 0 {
                0
            } else {
                (pending_len as u16 + 2).min(inner_h / 3)
            };

            let inner_y = 1; // Top border

            // Pending section: inner_y to inner_y + top_h
            if top_h > 0 && row >= inner_y && row < inner_y + top_h {
                // Clicked in pending area
                let rel_row = (row - inner_y) as usize;
                if rel_row > 0 && rel_row <= pending_len { // Account for title row
                    let idx = rel_row - 1;
                    if idx < pending_len {
                        if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                            if let Some(tab) = pane.tabs.get_mut(pane.active_tab_index) {
                                tab.git_pending_state.select(Some(idx));
                                tab.git_history_state.select(None);

                                if button == MouseButton::Left {
                                    if let Some(change) = tab.git_pending.get(idx) {
                                        trigger_git_diff_fetch(app, event_tx);
                                    }
                                }
                                return true;
                            }
                        }
                    }
                }
                return true; // Clicked in pending area but on title or empty space
            }

            // History section calculation
            let history_area_y = inner_y + top_h;
            let table_data_start_y = history_area_y + 2; // Title + header

            if row >= table_data_start_y {
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(tab) = pane.tabs.get_mut(pane.active_tab_index) {
                        let scroll_offset = tab.git_history_state.offset();
                        let rel_row = (row - table_data_start_y) as usize + scroll_offset;
                        if rel_row < tab.git_history.len() {
                            tab.git_history_state.select(Some(rel_row));
                            tab.git_pending_state.select(None);
                            if button == MouseButton::Left {
                                if let Some(commit) = tab.git_history.get(rel_row) {
                                    let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                                        app.focused_pane_index,
                                        std::path::PathBuf::from(format!("git://{}", commit.hash)),
                                    ));
                                    app.current_view = CurrentView::Commit;
                                    app.mode = AppMode::Viewer;
                                    app.sidebar_focus = false;
                                }
                            }
                            return true;
                        }
                    }
                }
            }
        }
    }
    false // Don't trap - event not handled
}
