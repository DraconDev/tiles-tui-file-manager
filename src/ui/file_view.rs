#![allow(unused_imports)]

//! File view rendering — file list, preview, editor, tree view.
//! Extracted from ui/pane.rs (Phase 3 continuation).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Widget,
    },
    Frame,
};
use std::path::Path;
use std::time::SystemTime;
use std::vec::Vec;

use crate::app::{App, CurrentView, DropTarget};
use crate::icons::Icon;
use crate::state::FileColumn;
use crate::ui::theme as theme;
use crate::ui::theme::THEME;
use crate::ui::footer::draw_stat_bar;
use crate::ui::panes::editor::draw_ide_editor;
use crate::ui::panes::breadcrumbs::draw_pane_breadcrumbs;
use crate::ui::misc::format_modified_time;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::{
    format_permissions, format_size, get_visual_width, squarify, truncate_to_width,
};
use dracon_terminal_engine::widgets::HotkeyHint;
use unicode_width::UnicodeWidthStr;

pub fn draw_file_view(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    pane_idx: usize,
    is_focused: bool,
    borders: Borders,
) {
    if let Some(pane) = app.panes.get_mut(pane_idx) {
        if let Some(fs) = pane.current_state_mut() {
            if let Some(preview) = &mut fs.view.preview {
            let block = Block::default()
                .borders(borders)
                .border_type(BorderType::Rounded)
                .title(format!(" Preview: {} ", preview.path.display()))
                .border_style(if is_focused {
                    Style::default().fg(theme::border_active())
                } else {
                    Style::default().fg(theme::border_inactive())
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

        let (base_r, base_g, base_b) = match theme::border_active() {
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
        Style::default().fg(theme::border_inactive())
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
        file_state.view.view_height = area.height as usize;

        let mut render_state = TableState::default();

        if let Some(sel) = file_state.list.selection.selected {
            let offset = file_state.view.table_state.offset();

            let capacity = file_state.view.view_height.saturating_sub(3);

            if sel >= offset && sel < offset + capacity {
                render_state.select(Some(sel));
            }
        }

        *render_state.offset_mut() = file_state.view.table_state.offset();

        let mut display_columns = Vec::new();

        for col in &file_state.list.columns {
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
                let name = if *c == file_state.nav.sort_column {
                    if file_state.nav.sort_ascending {
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
                        .fg(theme::header_fg())
                        .add_modifier(Modifier::BOLD),
                )])
            })
            .collect();

        // --- ABSOLUTE CELL ISOLATION RENDERING ---
        file_state.view.column_bounds.clear();
        file_state.view.file_row_bounds.clear();
        let header_y = inner_area.y;
        let content_y = header_y + 1;
        let visible_height = inner_area.height.saturating_sub(1) as usize;

        // 1. Render Headers
        for (col_idx, rect) in column_layout.iter().enumerate() {
            if let Some(col_type) = display_columns.get(col_idx) {
                file_state.view.column_bounds.push((*rect, *col_type));
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
        let offset_val = file_state.view.table_state.offset();
        let total_files = file_state.list.files.len();
        for i in 0..visible_height {
            let file_idx = offset_val + i;
            if file_idx >= total_files {
                break;
            }
            let row_y = content_y + i as u16;
            let path = &file_state.list.files[file_idx];
            let is_selected = file_state.list.selection.selected == Some(file_idx);
            let is_multi_selected = file_state.list.selection.multi.contains(&file_idx);

            let mut row_bg_style = Style::default();
            let is_hovered_drop =
                matches!(&app.drag.hovered_drop_target, Some(DropTarget::Folder(p)) if p == path);

            if is_selected {
                row_bg_style = row_bg_style.bg(theme::selection_bg());
            } else if is_multi_selected {
                row_bg_style = row_bg_style.bg(Color::Rgb(78, 58, 112));
            } else if is_hovered_drop {
                row_bg_style = row_bg_style.bg(theme::accent_secondary());
            } else if let Some(&c) = app.selection.path_colors.get(path) {
                let color = match c {
                    1 => theme::danger(),
                    2 => theme::success(),
                    3 => theme::warning(),
                    4 => Color::Blue,
                    5 => theme::accent_secondary(),
                    6 => theme::info(),
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

            let metadata = file_state.list.metadata.get(path);
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
                                    .fg(theme::warning())
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

                                    let depth = file_state.list.tree_file_depths.get(file_idx).copied().unwrap_or(0) as usize;
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
                                                cell_style.fg(theme::accent_secondary());
                                        } else {
                                            cell_style = cell_style.fg(cat.cyber_color());
                                        }
                                    }
                                    let icon_w = icon_str.chars().map(get_visual_width).sum::<usize>();
                                    let marker_w = if expand_marker { 2 } else { 0 };
                                    if is_dir {
                                        let arrow_end_x = col_rect.x + 1 + (depth * 2) as u16 + marker_w as u16 + icon_w as u16;
                                        file_state.view.file_row_bounds.push(crate::state::FileRowBounds {
                                            file_idx,
                                            arrow_end_x,
                                        });
                                    }
                                    // 12 = leading space (1) + minimal trailing pad + room for "[*]" suffix (4)
                                    const CELL_TEXT_RESERVE: usize = 12;
                                    let available_width =
                                        (col_rect.width as usize).saturating_sub(icon_w + marker_w + CELL_TEXT_RESERVE);

                                    let display_name = if file_idx > file_state.list.local_count {
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

            let mut scroll_state = ScrollbarState::new(file_state.list.files.len())
                .position(file_state.view.table_state.offset())
                .viewport_content_length(inner_area.height as usize);

            f.render_stateful_widget(scrollbar, area, &mut scroll_state);
        }
    }
}

