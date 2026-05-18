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
    if let AppMode::OpenWith(ref path) = app.core.mode {
        draw_open_with_modal(f, app, path);
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

