#![allow(unused_imports)]

//! Main stage rendering — dispatches to file view or IDE editor.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Borders,
    Frame,
};

use crate::app::{App, CurrentView};
use crate::ui::theme as theme;
use crate::ui::git_page::draw_git_page;
use crate::ui::file_view::draw_file_view;

pub fn draw_main_stage(f: &mut Frame, area: Rect, app: &mut App) {
    match app.core.current_view {
        CurrentView::Files => {
            let pane_count = app.panes.len();
            if pane_count == 0 {
                return;
            }

            let constraints = vec![Constraint::Fill(1); pane_count];
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .spacing(1) // Add 1-column gap between panes to prevent bleed-through
                .split(area);
            for i in 0..pane_count {
                let is_focused = i == app.focused_pane_index && !app.sidebar.sidebar_focus;
                let borders = Borders::ALL;
                draw_file_view(f, chunks[i], app, i, is_focused, borders);
            }
            // Store pane rects for cross-pane drag-drop in event handler
            app.layout.pane_rects = chunks.to_vec();
        }
        CurrentView::Editor => {
            crate::ui::panes::editor::draw_ide_editor(f, area, app);
        }
        _ => {}
    }
}
