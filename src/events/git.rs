#![allow(clippy::needless_borrow)]

use crate::app::{App, AppEvent, AppMode, CurrentView};
use dracon_terminal_engine::contracts::{InputEvent as Event, KeyCode, MouseButton, MouseEventKind};
use tokio::sync::mpsc;

pub fn handle_git_events(evt: &Event, app: &mut App, event_tx: &mpsc::Sender<AppEvent>) -> bool {
    if let CurrentView::Git = app.core.current_view {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Enter if matches!(app.core.mode, AppMode::Normal) => {
                    let mut open_preview: Option<std::path::PathBuf> = None;
                    let mut open_commit_view = false;
                    if let Some(fs) = app.current_file_state() {
                        // Priority 1: Pending changes (Diff)
                        if let Some(idx) = fs.git.git_pending_state.selected() {
                            if let Some(change) = fs.git.git_pending.get(idx) {
                                open_preview = Some(std::path::PathBuf::from(format!(
                                    "git-diff://{}",
                                    change.path
                                )));
                            }
                        }
                        // Priority 2: History (Commit)
                        if open_preview.is_none() {
                            if let Some(idx) = fs.git.git_history_state.selected() {
                                if let Some(commit) = fs.git.git_history.get(idx) {
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
                            app.core.current_view = CurrentView::Commit;
                            app.core.mode = AppMode::Viewer;
                            app.sidebar.sidebar_focus = false;
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

pub fn handle_git_mouse(
    me: &dracon_terminal_engine::contracts::MouseEvent,
    app: &mut App,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let row = me.row;
    if let MouseEventKind::Down(MouseButton::Left) = me.kind {
        if let Some(fs) = app.current_file_state() {
            let pending_len = fs.git.git_pending.len();

            // Layout mirrors draw_git_page:
            //   Outer block: Borders::ALL → inner starts at row 1
            //   inner.y = 1 (below top border)
            //   top_area (pending): rows 1..1+top_h
            //     pending table has Borders::RIGHT only, no header
            //     first data row = inner.y (= 1)
            //   history_area: rows 1+top_h..
            //     history table has Borders::TOP + .header().bottom_margin(1)
            //     border row = 1 row
            //     header row = 1 row
            //     bottom_margin = 1 row
            //     first data row = history_area_y + 3
            let inner_h = app.core.terminal_size.1.saturating_sub(2);
            let top_h: u16 = if pending_len == 0 {
                0
            } else {
                (pending_len as u16 + 2).min(inner_h / 3)
            };

            let inner_y: u16 = 1; // Below outer block top border

            // --- Pending section (ACTIVE) ---
            if top_h > 0 && row >= inner_y && row < inner_y + top_h {
                // Pending table: Borders::RIGHT only, no header row
                // First data row is at inner_y
                let rel_row = (row - inner_y) as usize;
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(tab) = pane.tabs.get_mut(pane.active_tab_index) {
                        if rel_row < tab.git.git_pending.len() {
                            tab.git.git_pending_state.select(Some(rel_row));
                            tab.git.git_history_state.select(None);
                            // Open diff preview
                            if let Some(change) = tab.git.git_pending.get(rel_row) {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                                    app.focused_pane_index,
                                    std::path::PathBuf::from(format!("git-diff://{}", change.path)),
                                ));
                            }
                            return true;
                        }
                    }
                }
            }

            // --- History section ---
            let history_area_y = inner_y + top_h;
            // Layout: Borders::TOP (1 row) + header (1 row) + bottom_margin(1) = 3 rows before data
            let table_data_start_y = history_area_y + 3;

            if row >= table_data_start_y {
                if let Some(pane) = app.panes.get_mut(app.focused_pane_index) {
                    if let Some(tab) = pane.tabs.get_mut(pane.active_tab_index) {
                        let scroll_offset = tab.git.git_history_state.offset();
                        let rel_row = (row - table_data_start_y) as usize + scroll_offset;
                        if rel_row < tab.git.git_history.len() {
                            tab.git.git_history_state.select(Some(rel_row));
                            tab.git.git_pending_state.select(None);
                            if let Some(commit) = tab.git.git_history.get(rel_row) {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(
                                    app.focused_pane_index,
                                    std::path::PathBuf::from(format!("git://{}", commit.hash)),
                                ));
                                app.core.current_view = CurrentView::Commit;
                                app.core.mode = AppMode::Viewer;
                                app.sidebar.sidebar_focus = false;
                            }
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
