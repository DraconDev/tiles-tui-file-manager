use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    Frame,
};
use std::time::SystemTime;

use crate::app::{
    App, AppMode, CurrentView, DropTarget, FileColumn,
};
use crate::icons::Icon;
use crate::ui::theme::THEME;
use dracon_terminal_engine::utils::{
    format_permissions, format_size, get_visual_width, squarify, truncate_to_width,
};
use dracon_terminal_engine::widgets::HotkeyHint;

pub mod header;
pub mod footer;
pub mod debug;
pub mod context_menu;
pub mod monitor;
pub mod modals;
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
pub use git_view::draw_commit_view;
pub use small_modals::{
    draw_signal_select_modal, draw_drag_drop_modal,
    draw_hotkeys_modal, draw_open_with_modal,
};
pub use settings::draw_settings_modal;
pub use misc::{
    draw_style_color_modal, draw_reset_settings_modal,
    draw_highlight_modal, draw_drag_ghost,
    format_modified_time,
};
pub use panes::breadcrumbs::draw_pane_breadcrumbs;

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
                    Style::default().fg(THEME.fg),
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
                    Style::default().fg(THEME.fg),
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
                        Style::default().fg(THEME.fg),
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
                        Style::default().fg(THEME.fg),
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
        header_right.extend(HotkeyHint::render("Esc", "Back", Color::Red));
        header_right.extend(HotkeyHint::render("^Q", "Quit", Color::Red));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title_top(Line::from(header_left))
            .title_top(Line::from(header_right).alignment(ratatui::layout::Alignment::Right))
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Rgb(0, 0, 0)));

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
                    Span::styled(" ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled(format!("Ln {}, Col {}", cursor_row, cursor_col), Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled(" | ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled(format!(" {} ", editor.language), Style::default().fg(crate::ui::theme::accent_secondary()).bg(footer_bg)),
                    Span::styled(" | ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled("  ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled("^S ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled("Save", Style::default().fg(crate::ui::theme::accent_secondary()).bg(footer_bg)),
                    Span::styled("  ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
                    Span::styled("^R ", Style::default().fg(Color::DarkGray).bg(footer_bg)),
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
            Block::default().style(Style::default().bg(Color::Black)),
            f.area(),
        );
        draw_settings_modal(f, app);
    } else if matches!(
        app.core.current_view,
        CurrentView::Processes | CurrentView::Git | CurrentView::Debug
    ) {
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
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
            Block::default().style(Style::default().bg(Color::Rgb(0, 0, 0))),
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
}


fn draw_main_stage(f: &mut Frame, area: Rect, app: &mut App) {
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
        }
        CurrentView::Editor => {
            crate::ui::panes::editor::draw_ide_editor(f, area, app);
        }
        _ => {}
    }
}

fn parse_commit_refs(decorations: &str) -> Vec<String> {
    decorations
        .trim()
        .trim_matches(|c| c == '(' || c == ')')
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn style_for_ref_label(label: &str) -> Style {
    if label.starts_with("HEAD -> ") {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if label.starts_with("tag: ") {
        Style::default().fg(Color::Magenta)
    } else if label.starts_with("origin/") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Yellow)
    }
}

fn refs_line(refs: &[String], max_refs: usize) -> Line<'static> {
    if refs.is_empty() {
        return Line::from("");
    }

    let mut spans = Vec::new();
    let shown = refs.len().min(max_refs);
    for (i, r) in refs.iter().take(shown).enumerate() {
        if i > 0 {
            spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            truncate_to_width(r, 18, ".."),
            style_for_ref_label(r),
        ));
    }

    if refs.len() > shown {
        spans.push(Span::styled(
            format!(" +{}", refs.len() - shown),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}

fn draw_git_page(f: &mut Frame, area: Rect, app: &mut App) {
    f.render_widget(Clear, area);
    let pane_idx = app.focused_pane_index;
    let tab_idx = if let Some(pane) = app.panes.get(pane_idx) {
        pane.active_tab_index
    } else {
        0
    };

    let Some(tab) = app.panes.get(pane_idx).and_then(|p| p.tabs.get(tab_idx)) else {
        return;
    };

    let branch_name = tab.git_branch.as_deref().unwrap_or("HEAD");
    let summary_text = tab.git_summary.as_deref().unwrap_or("");
    let current_path_label = tab.current_path.to_string_lossy().to_string();
    let history_len = tab.git_history.len();
    let pending_len = tab.git_pending.len();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(crate::ui::theme::accent_primary()))
        .style(Style::default().bg(Color::Rgb(0, 0, 0)))
        .title_top(Line::from(vec![
            Span::styled(
                " GIT HUB ",
                Style::default()
                    .fg(Color::Black)
                    .bg(crate::ui::theme::accent_primary())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [{}] ", branch_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", current_path_label),
                Style::default().fg(crate::ui::theme::accent_secondary()),
            ),
        ]))
        .title_bottom(Line::from(format!(" {} ", summary_text)).alignment(Alignment::Right))
        .title_top(
            Line::from(vec![
                Span::styled(
                    " Esc ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Back ", Style::default().fg(Color::Red)),
            ])
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Only show ACTIVE (pending) changes at top, no INFO panel
    let top_h = if pending_len == 0 {
        0
    } else {
        (pending_len as u16 + 2).min(inner.height / 3)
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_h), Constraint::Min(0)])
        .split(inner);

    let top_area = main_chunks[0];
    let history_area = main_chunks[1];

    if top_h > 0 {
        let active_area = top_area;
        f.render_widget(Clear, active_area);

        if pending_len > 0 {
            let active_title = format!(" ACTIVE ({} Affected) ", pending_len);
            let pending_rows: Vec<_> = app.panes
                .get(pane_idx)
                .and_then(|p| p.tabs.get(tab_idx))
                .map(|t| {
                    t.git_pending
                        .iter()
                        .map(|p| {
                            let status_color = match p.status.as_str() {
                                "M" => Color::Yellow,
                                "A" | "??" => Color::Green,
                                "D" => Color::Red,
                                "R" => Color::Cyan,
                                _ => Color::White,
                            };

                            let mut stats_spans = Vec::new();
                            if p.insertions > 0 {
                                stats_spans.push(Span::styled(
                                    format!(" +{}", p.insertions),
                                    Style::default().fg(Color::Green),
                                ));
                            }
                            if p.deletions > 0 {
                                stats_spans.push(Span::styled(
                                    format!(" -{}", p.deletions),
                                    Style::default().fg(Color::Red),
                                ));
                            }

                            Row::new(vec![
                                Cell::from(format!(" {} ", p.status)).style(
                                    Style::default()
                                        .bg(status_color)
                                        .fg(Color::Black)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Cell::from(p.path.clone()).style(Style::default().fg(THEME.fg)),
                                Cell::from(Line::from(stats_spans)),
                            ])
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let pending_table = Table::new(
                pending_rows,
                [
                    Constraint::Length(6),
                    Constraint::Fill(1),
                    Constraint::Length(15),
                ],
            )
            .block(
                Block::default()
                    .title(active_title)
                    .border_style(Style::default().fg(Color::Rgb(40, 45, 55)))
                    .borders(Borders::RIGHT),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

            if let Some(pane) = app.panes.get_mut(pane_idx) {
                if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                    f.render_stateful_widget(
                        pending_table,
                        active_area,
                        &mut tab.git_pending_state,
                    );
                }
            }
        }
    }

    if history_len == 0 {
        f.render_widget(
            Paragraph::new("\n\n No git history found for this path or not a git repository.")
                .alignment(Alignment::Center),
            history_area,
        );
    } else {
        let rows: Vec<_> = app.panes
            .get(pane_idx)
            .and_then(|p| p.tabs.get(tab_idx))
            .map(|t| {
                t.git_history
                    .iter()
                    .map(|act| {
                        let h_short = act.hash.chars().take(7).collect::<String>();
                        let refs = parse_commit_refs(&act.decorations);
                        let refs_compact = refs_line(&refs, 2);

                        let mut stats_cells = Vec::new();
                        if act.files_changed > 0 {
                            stats_cells.push(
                                Cell::from(act.files_changed.to_string())
                                    .style(Style::default().fg(Color::Cyan)),
                            );
                            stats_cells.push(
                                Cell::from(format!("+{}", act.insertions))
                                    .style(Style::default().fg(Color::Green)),
                            );
                            stats_cells.push(
                                Cell::from(format!("-{}", act.deletions))
                                    .style(Style::default().fg(Color::Red)),
                            );
                        } else {
                            stats_cells.push(Cell::from(""));
                            stats_cells.push(Cell::from(""));
                            stats_cells.push(Cell::from(""));
                        }

                        let mut row_cells = vec![
                            Cell::from(act.date.clone())
                                .style(Style::default().fg(Color::DarkGray)),
                            Cell::from(h_short).style(
                                Style::default()
                                    .fg(crate::ui::theme::accent_secondary())
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Cell::from(refs_compact),
                            Cell::from(act.author.clone()).style(Style::default().fg(Color::Cyan)),
                            Cell::from(act.message.clone()).style(Style::default().fg(THEME.fg)),
                        ];
                        row_cells.extend(stats_cells);

                        Row::new(row_cells)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let table = Table::new(
            rows,
            [
                Constraint::Length(15),
                Constraint::Length(8),
                Constraint::Length(20),
                Constraint::Length(15),
                Constraint::Fill(1),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(6),
            ],
        )
        .header(
            Row::new(vec![
                "DATE", "HASH", "REFS", "AUTHOR", "MESSAGE", "FILES", "ADD", "DEL",
            ])
            .style(
                Style::default()
                    .fg(crate::ui::theme::accent_secondary())
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(" HISTORY ")
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(40, 45, 55))),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 50))
                .fg(crate::ui::theme::accent_secondary())
                .add_modifier(Modifier::BOLD),
        );

        if let Some(pane) = app.panes.get_mut(pane_idx) {
            if let Some(tab) = pane.tabs.get_mut(tab_idx) {
                f.render_stateful_widget(table, history_area, &mut tab.git_history_state);
            }
        }
    }
}

fn draw_file_view(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    pane_idx: usize,
    is_focused: bool,
    borders: Borders,
) {
    if let Some(pane) = app.panes.get_mut(pane_idx) {
        if let Some(fs) = pane.current_state_mut() {
            if let Some(preview) = &mut fs.preview {
            let block = Block::default()
                .borders(borders)
                .border_type(BorderType::Rounded)
                .title(format!(" Preview: {} ", preview.path.display()))
                .border_style(if is_focused {
                    Style::default().fg(crate::ui::theme::border_active())
                } else {
                    Style::default().fg(crate::ui::theme::border_inactive())
                });

            let lines = if let Some(cached) = &preview.highlighted_lines {
                cached.clone()
            } else {
                let language = preview
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                // PERFORMANCE OPTIMIZATION: Only highlight what's likely to be visible + some buffer
                // This is a PREVIEW, so full file highlighting is overkill for large files.
                let content_to_highlight = if preview.content.lines().count() > 500 {
                    preview
                        .content
                        .lines()
                        .take(500)
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    preview.content.clone()
                };

                let highlighted = dracon_terminal_engine::utils::highlight_code(&content_to_highlight, language);
                let mut lines = Vec::new();
                for (i, line) in highlighted.iter().enumerate() {
                    let mut spans = line
                        .spans
                        .iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style))
                        .collect::<Vec<_>>();
                    // Prepend line number gutter
                    let num = format!("{:>3} │ ", i + 1);
                    spans.insert(
                        0,
                        Span::styled(num, Style::default().fg(Color::Rgb(60, 60, 70))),
                    );
                    lines.push(Line::from(spans));
                }
                preview.highlighted_lines = Some(lines.clone());
                lines
            };

            let text = Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(block);

            f.render_widget(text, area);
            return;
        }
        }
    }

    // --- BORDER & BACKGROUND (Rendered FIRST to create base) ---

    let border_style = if is_focused {
        let pulse = ((SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 1500) as f32
            / 1500.0
            * std::f32::consts::PI
            * 2.0)
            .sin()
            * 0.5
            + 0.5;

        let (base_r, base_g, base_b) = match crate::ui::theme::border_active() {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (0.0, 150.0, 255.0),
        };
        let intensity = 0.7 + 0.3 * pulse;
        let r = (base_r * intensity) as u8;
        let g = (base_g * intensity) as u8;
        let b = (base_b * intensity) as u8;

        Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(crate::ui::theme::border_inactive())
    };


    let main_block = Block::default()
        .borders(borders)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    f.render_widget(main_block, area);

    draw_pane_breadcrumbs(f, area, app, pane_idx);

    if let Some(file_state) = app.panes
        .get_mut(pane_idx)
        .and_then(|p| p.current_state_mut())
    {
        file_state.view_height = area.height as usize;

        let mut render_state = TableState::default();

        if let Some(sel) = file_state.selection.selected {
            let offset = file_state.table_state.offset();

            let capacity = file_state.view_height.saturating_sub(3);

            if sel >= offset && sel < offset + capacity {
                render_state.select(Some(sel));
            }
        }

        *render_state.offset_mut() = file_state.table_state.offset();

        let mut display_columns = Vec::new();

        for col in &file_state.columns {
            match col {
                FileColumn::Name => display_columns.push(FileColumn::Name),
                FileColumn::Size if area.width > 40 => display_columns.push(FileColumn::Size),
                FileColumn::Modified if area.width > 70 => {
                    display_columns.push(FileColumn::Modified)
                }
                FileColumn::Created if area.width > 90 => display_columns.push(FileColumn::Created),
                FileColumn::Permissions if area.width > 110 => {
                    display_columns.push(FileColumn::Permissions)
                }
                _ => {}
            }
        }
        // Ensure Name is always there as a safety fallback
        if !display_columns.contains(&FileColumn::Name) {
            display_columns.insert(0, FileColumn::Name);
        }

        let constraints: Vec<Constraint> = display_columns
            .iter()
            .map(|c| match c {
                FileColumn::Name => Constraint::Fill(1),
                FileColumn::Size => Constraint::Length(12),
                FileColumn::Modified => Constraint::Length(20),
                FileColumn::Created => Constraint::Length(20),
                FileColumn::Permissions => Constraint::Length(12),
            })
            .collect();

        let dummy_block = Block::default().borders(borders);
        let inner_area = dummy_block.inner(area);
        let column_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints.clone())
            .spacing(0)
            .split(inner_area);

        let header_lines: Vec<Line> = display_columns
            .iter()
            .map(|c| {
                let base_name = match c {
                    FileColumn::Name => "Name",
                    FileColumn::Size => "Size",
                    FileColumn::Modified => "Modified",
                    FileColumn::Created => "Created",
                    FileColumn::Permissions => "Permissions",
                };
                let name = if *c == file_state.sort_column {
                    if file_state.sort_ascending {
                        format!("{} ▲", base_name)
                    } else {
                        format!("{} ▼", base_name)
                    }
                } else {
                    base_name.to_string()
                };
                Line::from(vec![Span::styled(
                    name,
                    Style::default()
                        .fg(crate::ui::theme::header_fg())
                        .add_modifier(Modifier::BOLD),
                )])
            })
            .collect();

        // --- ABSOLUTE CELL ISOLATION RENDERING ---
        file_state.column_bounds.clear();
        file_state.file_row_bounds.clear();
        let header_y = inner_area.y;
        let content_y = header_y + 1;
        let visible_height = inner_area.height.saturating_sub(1) as usize;

        // 1. Render Headers
        for (col_idx, rect) in column_layout.iter().enumerate() {
            if let Some(col_type) = display_columns.get(col_idx) {
                file_state.column_bounds.push((*rect, *col_type));
                let header_line = header_lines.get(col_idx).cloned().unwrap_or(Line::from(""));
                let header_rect = Rect::new(rect.x, header_y, rect.width, 1);
                let alignment = match col_type {
                    FileColumn::Name => ratatui::layout::Alignment::Left,
                    _ => ratatui::layout::Alignment::Right,
                };
                f.render_widget(
                    Paragraph::new(header_line).alignment(alignment),
                    header_rect,
                );
            }
        }

        // 2. Render Rows
        let offset_val = file_state.table_state.offset();
        let total_files = file_state.files.len();
        for i in 0..visible_height {
            let file_idx = offset_val + i;
            if file_idx >= total_files {
                break;
            }
            let row_y = content_y + i as u16;
            let path = &file_state.files[file_idx];
            let is_selected = file_state.selection.selected == Some(file_idx);
            let is_multi_selected = file_state.selection.multi.contains(&file_idx);

            let mut row_bg_style = Style::default();
            let is_hovered_drop =
                matches!(&app.drag.hovered_drop_target, Some(DropTarget::Folder(p)) if p == path);

            if is_selected {
                row_bg_style = row_bg_style.bg(crate::ui::theme::selection_bg());
            } else if is_multi_selected {
                row_bg_style = row_bg_style.bg(Color::Rgb(78, 58, 112));
            } else if is_hovered_drop {
                row_bg_style = row_bg_style.bg(crate::ui::theme::accent_secondary());
            } else if let Some(&c) = app.selection.path_colors.get(path) {
                let color = match c {
                    1 => Color::Red,
                    2 => Color::Green,
                    3 => Color::Yellow,
                    4 => Color::Blue,
                    5 => Color::Magenta,
                    6 => Color::Cyan,
                    _ => Color::Reset,
                };
                if color != Color::Reset {
                    row_bg_style = row_bg_style.bg(color);
                }
            }
            if row_bg_style.bg.is_some() {
                f.render_widget(
                    Block::default().style(row_bg_style),
                    Rect::new(inner_area.x, row_y, inner_area.width, 1),
                );
            }

            let metadata = file_state.metadata.get(path);
            for (col_idx, col_rect) in column_layout.iter().enumerate() {
                if let Some(col_type) = display_columns.get(col_idx) {
                    let cell_rect = Rect::new(col_rect.x, row_y, col_rect.width, 1);
                    let mut cell_style = if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else if is_multi_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if is_hovered_drop || app.selection.path_colors.contains_key(path) {
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(THEME.fg)
                    };

                    let content = match col_type {
                        FileColumn::Name => {
                            if path.to_string_lossy() == "__DIVIDER__" {
                                cell_style = Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD);
                                "> Global results".to_string()
} else {
                                    let name =
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("..");
                                    let is_dir = metadata.map(|m| m.is_dir).unwrap_or(false);
                                    let cat = crate::modules::files::get_file_category(path);
                                    let icon_str = Icon::get_for_path(path, cat, is_dir, app.core.icon_mode);
                                    let icon_str = if !is_dir && matches!(
                                        path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                                        "package.json" | "package-lock.json"
                                    ) {
                                        Icon::get_for_path(path, cat, false, crate::icons::IconMode::Unicode)
                                    } else {
                                        icon_str
                                    };

                                    let depth = file_state.tree_file_depths.get(file_idx).copied().unwrap_or(0) as usize;
                                    let indent = "  ".repeat(depth);
                                    let is_expanded = is_dir && app.layout.expanded_folders.contains(path);
                                    let marker = if is_dir {
                                        if is_expanded { "▾ " } else { "▸ " }
                                    } else {
                                        "  "
                                    };
                                    let (depth_indent, expand_marker) = (
                                        format!("{}{}", indent, marker),
                                        is_dir && !marker.is_empty(),
                                    );

                                    let mut suffix = String::new();
                                    if app.nav.starred.contains(path) {
                                        suffix.push_str(" [*]");
                                    }
                                    if !is_selected
                                        && !is_multi_selected
                                        && !app.selection.path_colors.contains_key(path)
                                        && !is_hovered_drop
                                        && app.settings.semantic_coloring
                                    {
                                        if is_dir {
                                            cell_style =
                                                cell_style.fg(crate::ui::theme::accent_secondary());
                                        } else {
                                            cell_style = cell_style.fg(cat.cyber_color());
                                        }
                                    }
                                    let icon_w = icon_str.chars().map(get_visual_width).sum::<usize>();
                                    let marker_w = if expand_marker { 2 } else { 0 };
                                    if is_dir {
                                        let arrow_end_x = col_rect.x + 1 + (depth * 2) as u16 + marker_w as u16 + icon_w as u16;
                                        file_state.file_row_bounds.push(crate::state::FileRowBounds {
                                            file_idx,
                                            arrow_end_x,
                                        });
                                    }
                                    // 12 = leading space (1) + minimal trailing pad + room for "[*]" suffix (4)
                                    const CELL_TEXT_RESERVE: usize = 12;
                                    let available_width =
                                        (col_rect.width as usize).saturating_sub(icon_w + marker_w + CELL_TEXT_RESERVE);

                                    let display_name = if file_idx > file_state.local_count {
                                        let full_str = path.to_string_lossy();
                                        let home = dirs::home_dir()
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "/root".to_string());
                                        if full_str.starts_with(&home) {
                                            full_str.replacen(&home, "~", 1)
                                        } else {
                                            full_str.to_string()
                                        }
                                    } else {
                                        name.to_string()
                                    };
                                    let display_name = squarify(&display_name);

                                    let truncated_name =
                                        truncate_to_width(&display_name, available_width, "..");
                                    let cell_text = if depth_indent.is_empty() {
                                        format!(" {} {}{}", icon_str, truncated_name, suffix)
                                    } else {
                                        format!("{}{} {}{}", depth_indent, icon_str, truncated_name, suffix)
                                    };

                                    cell_text
                            }
                        }
                        FileColumn::Size => {
                            let size = metadata.map(|m| m.size).unwrap_or(0);
                            let is_dir = metadata.map(|m| m.is_dir).unwrap_or(false);
                            let text = if is_dir && size == 0 {
                                "<DIR>".to_string()
                            } else {
                                format_size(size)
                            };
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Modified => {
                            let text = format_modified_time(
                                metadata
                                    .map(|m| m.modified)
                                    .unwrap_or(SystemTime::UNIX_EPOCH),
                                app.settings.smart_date,
                            );
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Permissions => {
                            let text =
                                format_permissions(metadata.map(|m| m.permissions).unwrap_or(0));
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                        FileColumn::Created => {
                            let text = format_modified_time(
                                metadata
                                    .map(|m| m.created)
                                    .unwrap_or(SystemTime::UNIX_EPOCH),
                                app.settings.smart_date,
                            );
                            truncate_to_width(
                                &text,
                                (col_rect.width as usize).saturating_sub(2),
                                "",
                            )
                        }
                    };
                    let alignment = match col_type {
                        FileColumn::Name => ratatui::layout::Alignment::Left,
                        _ => ratatui::layout::Alignment::Right,
                    };
                    f.render_widget(
                        Paragraph::new(Span::styled(content, cell_style)).alignment(alignment),
                        cell_rect,
                    );
                }
            }
        }

        if total_files > area.height.saturating_sub(4) as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            let mut scroll_state = ScrollbarState::new(file_state.files.len())
                .position(file_state.table_state.offset())
                .viewport_content_length(inner_area.height as usize);

            f.render_stateful_widget(scrollbar, area, &mut scroll_state);
        }
    }
}

