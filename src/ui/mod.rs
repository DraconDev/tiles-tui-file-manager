use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Paragraph,
    },
    Frame,
};

use crate::app::{
    App, AppMode, CurrentView,
};
use dracon_terminal_engine::widgets::HotkeyHint;

pub mod header;
pub mod footer;
pub mod debug;
pub mod context_menu;
pub mod monitor;
pub mod modals;
pub mod pane;
pub mod git_page;
pub mod file_view;
pub mod git_view;
pub mod small_modals;
pub mod settings;
pub mod misc;
pub mod panes;
pub mod sparkline;
pub mod theme;

pub use header::draw_global_header;
#[allow(unused_imports)]
pub use footer::{draw_stat_bar, draw_footer};
pub use debug::{draw_debug_page, draw_add_remote_modal};
pub use context_menu::draw_context_menu;
pub use monitor::draw_monitor_page;
pub use modals::{
    draw_import_servers_modal, draw_command_palette, draw_rename_modal,
    draw_new_folder_modal, draw_new_file_modal, draw_bulk_rename_modal,
    draw_save_as_modal, draw_delete_modal, draw_properties_modal,
};
pub use pane::draw_main_stage;
pub use git_page::draw_git_page;
#[allow(unused_imports)]
pub use file_view::draw_file_view;
pub use git_view::draw_commit_view;
pub use small_modals::{
    draw_signal_select_modal, draw_drag_drop_modal,
    draw_hotkeys_modal, draw_open_with_modal,
};
pub use settings::draw_settings_modal;
pub use misc::{
    draw_style_color_modal, draw_reset_settings_modal,
    draw_highlight_modal, draw_drag_ghost, draw_marquee_rect,
};

/// A modal box of an ABSOLUTE size in cells, centered inside `area`.
///
/// `dracon_terminal_engine::layout::centered_rect` interprets its arguments as
/// PERCENTAGES. Passing cell counts there silently shrinks every dialog — 64x9
/// on an 80x24 frame becomes 52x2, so the box is smaller than the content drawn
/// at inner offsets and the rows spill outside their border. Fixed-size dialogs
/// therefore state their size here, in cells, clamped to the frame.
pub fn centered_modal(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width).max(1);
    let height = height.min(area.height).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

/// Cell size of the style-color editor dialog (see `draw_style_color_modal`).
pub const STYLE_COLOR_MODAL_SIZE: (u16, u16) = (64, 10);

/// Cell size of the reset-settings confirmation (see `draw_reset_settings_modal`).
pub const RESET_SETTINGS_MODAL_SIZE: (u16, u16) = (56, 13);

/// The style-color editor's rectangle for a frame of `w` by `h` cells.
pub fn style_color_modal_area(w: u16, h: u16) -> Rect {
    centered_modal(STYLE_COLOR_MODAL_SIZE.0, STYLE_COLOR_MODAL_SIZE.1, Rect::new(0, 0, w, h))
}

/// The reset-settings dialog's rectangle for a frame of `w` by `h` cells.
pub fn reset_settings_modal_area(w: u16, h: u16) -> Rect {
    centered_modal(RESET_SETTINGS_MODAL_SIZE.0, RESET_SETTINGS_MODAL_SIZE.1, Rect::new(0, 0, w, h))
}

/// Row height of the delete confirmation dialog (borders + message + buttons).
pub const DELETE_MODAL_HEIGHT: u16 = 7;

/// Width percent of the delete dialog; it also enforces a 36-cell minimum.
pub fn delete_modal_width_percent(area_width: u16) -> u16 {
    40u16.max(((36u32 * 100) / area_width.max(1) as u32) as u16)
}

/// The delete dialog's rectangle for a frame of `w` by `h` cells.
pub fn delete_modal_area(w: u16, h: u16) -> Rect {
    let area = Rect::new(0, 0, w, h);
    centered_modal(w * delete_modal_width_percent(w) / 100, DELETE_MODAL_HEIGHT, area)
}

/// Render the entire TUI frame.
///
/// Dispatches to sub-modules based on the current view and mode:
/// - Files view → header + sidebar + file panes + footer
/// - Editor view → header + sidebar + editor pane
/// - Git view → git page layout
/// - Processes view → system monitor
/// - Debug view → debug overlay
/// - Modals overlay on top of the current view.
pub fn draw(f: &mut Frame, app: &mut App) {
    f.render_widget(Clear, f.area());

    if app.core.current_view == CurrentView::Commit {
        draw_commit_view(f, f.area(), app);
    } else if matches!(
        app.core.mode,
        AppMode::Editor
            | AppMode::Viewer
            | AppMode::EditorSearch
            | AppMode::EditorGoToLine
            | AppMode::EditorReplace
    ) && app.show_main_stage
        && !app.core.is_split_mode
    {
        // --- FULL SCREEN EDITOR VIEW (Zen Mode / Overlay) ---
        let mut header_left = Vec::new();
        let border_color = if let Some(preview) = &app.editor_global.editor_state {
            if let Some(last_saved) = preview.last_saved {
                if last_saved.elapsed().as_secs() < 2 {
                    crate::ui::theme::accent_secondary()
                } else if let Some(editor) = &preview.editor {
                    if editor.modified {
                        crate::ui::theme::accent_primary()
                    } else {
                        crate::ui::theme::border_active()
                    }
                } else {
                    crate::ui::theme::border_active()
                }
            } else if let Some(editor) = &preview.editor {
                if editor.modified {
                    crate::ui::theme::accent_primary()
                } else {
                    crate::ui::theme::border_active()
                }
            } else {
                crate::ui::theme::border_active()
            }
        } else {
            crate::ui::theme::border_active()
        };

        match app.core.mode {
            AppMode::EditorSearch => {
                header_left.push(Span::styled(
                    "SEARCH: ",
                    Style::default()
                        .fg(crate::ui::theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ));
                header_left.push(Span::styled(
                    &app.core.input.value,
                    Style::default().fg(theme::fg()),
                ));
            }
            AppMode::EditorGoToLine => {
                header_left.push(Span::styled(
                    "GO TO LINE: ",
                    Style::default()
                        .fg(crate::ui::theme::accent_primary())
                        .add_modifier(Modifier::BOLD),
                ));
                header_left.push(Span::styled(
                    &app.core.input.value,
                    Style::default().fg(theme::fg()),
                ));
            }
            AppMode::EditorReplace => {
                if app.editor_global.replace_buffer.is_empty() {
                    header_left.push(Span::styled(
                        "REPLACE [FIND]: ",
                        Style::default()
                            .fg(crate::ui::theme::accent_secondary())
                            .add_modifier(Modifier::BOLD),
                    ));
                    header_left.push(Span::styled(
                        &app.core.input.value,
                        Style::default().fg(theme::fg()),
                    ));
                } else {
                    header_left.push(Span::styled(
                        "REPLACE [WITH]: ",
                        Style::default()
                            .fg(crate::ui::theme::accent_secondary())
                            .add_modifier(Modifier::BOLD),
                    ));
                    header_left.push(Span::styled(
                        &app.core.input.value,
                        Style::default().fg(theme::fg()),
                    ));
                }
            }
            _ => {
                header_left.extend(HotkeyHint::render(
                    "^F",
                    "Find",
                    crate::ui::theme::accent_secondary(),
                ));
                header_left.extend(HotkeyHint::render(
                    "F2",
                    "Replace",
                    crate::ui::theme::accent_secondary(),
                ));
                header_left.extend(HotkeyHint::render(
                    "^G",
                    "Line",
                    crate::ui::theme::accent_secondary(),
                ));
            }
        }

        let mut header_right = Vec::new();
        header_right.extend(HotkeyHint::render("Esc", "Back", crate::ui::theme::accent_primary()));
        header_right.extend(HotkeyHint::render("^Q", "Quit", crate::ui::theme::accent_primary()));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title_top(Line::from(header_left))
            .title_top(Line::from(header_right).alignment(ratatui::layout::Alignment::Right))
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme::bg()));

        f.render_widget(block.clone(), f.area());

        let inner_area = block.inner(f.area());
        // Fix for line number border overlap: add 1 column of padding on left
        let inner_area = ratatui::layout::Rect {
            x: inner_area.x + 1,
            width: inner_area.width.saturating_sub(1),
            ..inner_area
        };

        let footer_height = 1u16;
        let editor_area = Rect::new(
            inner_area.x,
            inner_area.y,
            inner_area.width,
            inner_area.height.saturating_sub(footer_height),
        );
        let footer_area = Rect::new(
            inner_area.x,
            inner_area.y + inner_area.height.saturating_sub(footer_height),
            inner_area.width,
            footer_height,
        );

        if let Some(preview) = &app.editor_global.editor_state {
            if let Some(editor) = &preview.editor {
                let mut editor_clone = editor.clone();
                editor_clone.wrap = app.core.is_split_mode;
                f.render_widget(&editor_clone, editor_area);

                // Footer bar: Ln X, Col Y | language | ^S Save ^R Run
                let cursor_row = editor.cursor_row + 1;
                let cursor_col = editor.cursor_col + 1;
                let footer_bg = if editor.modified {
                    crate::ui::theme::selection_bg()
                } else {
                    Color::Reset
                };

                let footer_line = Line::from(vec![
                    Span::styled(" ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled(format!("Ln {}, Col {}", cursor_row, cursor_col), Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled(" | ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled(format!(" {} ", editor.language), Style::default().fg(crate::ui::theme::accent_secondary()).bg(footer_bg)),
                    Span::styled(" | ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled("  ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled("^S ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled("Save", Style::default().fg(crate::ui::theme::accent_secondary()).bg(footer_bg)),
                    Span::styled("  ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled("^R ", Style::default().fg(crate::ui::theme::muted()).bg(footer_bg)),
                    Span::styled("Run", Style::default().fg(crate::ui::theme::accent_secondary()).bg(footer_bg)),
                ]);
                f.render_widget(Paragraph::new(footer_line).alignment(Alignment::Left), footer_area);
            }
        }

        if matches!(app.core.mode, AppMode::EditorSearch | AppMode::EditorGoToLine | AppMode::EditorReplace) {
            let search_footer_height = 2;
            let search_footer_area = Rect::new(
                f.area().x,
                f.area().height.saturating_sub(search_footer_height),
                f.area().width,
                search_footer_height,
            );
            draw_footer(f, search_footer_area, app);
        }
    } else if matches!(
        app.core.mode,
        AppMode::Settings | AppMode::StyleColorInput | AppMode::ResetSettingsConfirm
    ) {
        f.render_widget(
            Block::default().style(Style::default().bg(theme::bg())),
            f.area(),
        );
        draw_settings_modal(f, app);
    } else if matches!(
        app.core.current_view,
        CurrentView::Processes | CurrentView::Git | CurrentView::Debug
    ) {
        f.render_widget(
            Block::default().style(Style::default().bg(theme::bg())),
            f.area(),
        );
        match app.core.current_view {
            CurrentView::Processes => draw_monitor_page(f, f.area(), app),
            CurrentView::Git => draw_git_page(f, f.area(), app),
            CurrentView::Debug => draw_debug_page(f, f.area(), app),
            _ => {}
        }
    } else {
        // Normal File Manager Background
        f.render_widget(
            Block::default().style(Style::default().bg(theme::bg())),
            f.area(),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(2),
            ])
            .split(f.area());

        let workspace_constraints = if app.show_main_stage {
            if app.sidebar.show_sidebar {
                [Constraint::Length(app.sidebar_width()), Constraint::Fill(1)]
            } else {
                [Constraint::Length(0), Constraint::Fill(1)]
            }
        } else {
            [Constraint::Fill(1), Constraint::Length(0)]
        };

        let workspace = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(workspace_constraints)
            .split(chunks[1]);

        draw_global_header(f, chunks[0], workspace[0].width, app);

        if app.sidebar.show_sidebar || !app.show_main_stage {
            crate::ui::panes::sidebar::draw_sidebar(f, workspace[0], app);
        }

        if app.show_main_stage {
            draw_main_stage(f, workspace[1], app);
        }

        draw_footer(f, chunks[2], app);
    }

    // --- OVERLAYS ---
    if let AppMode::Hotkeys = app.core.mode {
        draw_hotkeys_modal(f, f.area());
    }
    if matches!(app.core.mode, AppMode::ContextMenu { .. }) {
        if let AppMode::ContextMenu {
            x, y, ref target, ..
        } = app.core.mode
        {
            draw_context_menu(f, x, y, target, app);
        }
    }
    if matches!(app.core.mode, AppMode::Highlight) {
        draw_highlight_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::Rename) {
        draw_rename_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::BulkRename { .. }) {
        draw_bulk_rename_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::Delete(_) | AppMode::DeleteFile(_)) {
        draw_delete_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::Properties) {
        draw_properties_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::NewFolder) {
        draw_new_folder_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::NewFile) {
        draw_new_file_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::SaveAs(_)) {
        draw_save_as_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::CommandPalette) {
        draw_command_palette(f, app);
    }
    if matches!(app.core.mode, AppMode::StyleColorInput) {
        draw_style_color_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::ResetSettingsConfirm) {
        draw_reset_settings_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::AddRemote(_)) {
        draw_add_remote_modal(f, app);
    }
    if matches!(app.core.mode, AppMode::ImportServers) {
        draw_import_servers_modal(f, app);
    }
    // Clone the path: draw_open_with_modal needs &mut app (scroll offset).
    let open_with_path = if let AppMode::OpenWith(ref path) = app.core.mode {
        Some(path.clone())
    } else {
        None
    };
    if let Some(path) = open_with_path {
        draw_open_with_modal(f, app, &path);
    }
    if let AppMode::DragDropMenu {
        ref sources,
        ref target,
    } = app.core.mode
    {
        draw_drag_drop_modal(f, app, sources, target);
    }
    if let AppMode::SignalSelect { pid, ref name, selected_index } = app.core.mode {
        draw_signal_select_modal(f, app, pid, name, selected_index);
    }

    if app.drag.is_dragging {
        draw_drag_ghost(f, app);
    }

    // Marquee selection rect
    draw_marquee_rect(f, app);
}

#[cfg(test)]
mod modal_geometry_tests {
    use super::*;
    use ratatui::layout::Rect;

    /// Every fixed-size dialog gets the size it asks for, centered — the
    /// engine's `centered_rect` treats its arguments as percentages, which
    /// shrank these boxes to a couple of rows and pushed their content
    /// outside the border.
    #[test]
    fn centered_modal_returns_the_requested_cell_size_at_the_center() {
        let frame = Rect::new(0, 0, 80, 24);
        let area = centered_modal(64, 10, frame);
        assert_eq!(area.width, 64, "width must be a cell count");
        assert_eq!(area.height, 10, "height must be a cell count");
        assert_eq!(area.x, 8, "(80 - 64) / 2");
        assert_eq!(area.y, 7, "(24 - 10) / 2");
    }

    /// Oversized requests are clamped to the frame instead of drawing outside.
    #[test]
    fn centered_modal_clamps_to_the_frame() {
        let frame = Rect::new(0, 0, 40, 10);
        let area = centered_modal(64, 16, frame);
        assert_eq!(area.width, 40);
        assert_eq!(area.height, 10);
        assert_eq!(area.x, 0);
        assert_eq!(area.y, 0);
    }

    /// The comparison that failed before the fix: for the same frame, the
    /// percent-based helper produced a box far too small for a dialog that
    /// declares its size in cells.
    #[test]
    fn centered_modal_is_stable_across_frame_sizes_unlike_percentages() {
        for (w, h) in [(80u16, 24u16), (120, 40), (200, 60)] {
            let area = centered_modal(64, 10, Rect::new(0, 0, w, h));
            assert_eq!(area.width, 64, "width is frame-independent");
            assert_eq!(area.height, 10, "height is frame-independent");
            let percent = dracon_terminal_engine::layout::centered_rect(64, 10, Rect::new(0, 0, w, h));
            assert!(
                area.height > percent.height || area.height == 10,
                "the fixed size must not shrink with the frame"
            );
        }
    }

    /// The delete dialog's render box and its click hit-test box must agree.
    #[test]
    fn delete_modal_area_is_wide_enough_for_its_buttons() {
        for w in [40u16, 80, 90, 120, 200] {
            let area = delete_modal_area(w, 24);
            assert!(
                area.width >= 36,
                "width {} at frame width {w} must leave room for both buttons",
                area.width
            );
            assert_eq!(area.height, DELETE_MODAL_HEIGHT);
            assert!(area.width <= w, "never wider than the frame");
        }
    }
}

