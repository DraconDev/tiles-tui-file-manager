#![allow(unused_imports)]

//! Modal dialogs — import servers, command palette, rename, new, delete, etc.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Paragraph, Row, Widget,
    },
    Frame,
};
use std::time::SystemTime;

use crate::app::{App, AppMode};
use crate::ui::theme as theme;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::{format_size, format_time, format_permissions};
use dracon_terminal_engine::widgets::HotkeyHint;

pub fn draw_import_servers_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Import Servers (TOML) ")
        .border_style(Style::default().fg(theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new("Enter path to server configuration file:"),
        chunks[0],
    );

    let input_area = chunks[1];
    f.render_widget(
        Paragraph::new("> ").style(Style::default().fg(theme::accent_secondary())),
        Rect::new(input_area.x, input_area.y, 2, 1),
    );
    f.render_widget(
        &app.core.input,
        Rect::new(
            input_area.x + 2,
            input_area.y,
            input_area.width.saturating_sub(2),
            1,
        ),
    );

    let example_toml = r#"Example format:
[[servers]]
name = "Production"
host = "192.168.1.10"
user = "admin"
port = 22"#;

    f.render_widget(
        Paragraph::new(example_toml).style(Style::default().fg(theme::muted())),
        chunks[2],
    );

    let mut footer_text = Vec::new();
    footer_text.extend(HotkeyHint::render("Enter", "Import", theme::success()));
    footer_text.extend(HotkeyHint::render("Esc", "Cancel", theme::accent_primary()));

    f.render_widget(Paragraph::new(Line::from(footer_text)), chunks[3]);
}

pub fn draw_command_palette(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 40, f.area());
    f.render_widget(Clear, area);
    let inner = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Command Palette ")
        .border_style(Style::default().fg(theme::accent_secondary()))
        .inner(area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Command Palette ")
            .border_style(Style::default().fg(theme::accent_secondary())),
        area,
    );

    f.render_widget(
        Paragraph::new("> ").style(Style::default().fg(theme::warning())),
        Rect::new(inner.x, inner.y, 2, 1),
    );
    f.render_widget(
        &app.core.input,
        Rect::new(inner.x + 2, inner.y, inner.width.saturating_sub(2), 1),
    );

    let items: Vec<ListItem> = app.nav.filtered_commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let style = if i == app.nav.command_index {
                Style::default().bg(theme::muted()).fg(theme::fg())
            } else {
                Style::default()
            };
            ListItem::new(cmd.desc.clone()).style(style)
        })
        .collect();
    f.render_widget(
        List::new(items),
        Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1),
    );
}

pub fn draw_rename_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Rename ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.selection.rename_selected {
        let text = if let Some(idx) = app.core.input.value.rfind('.') {
            if idx > 0 {
                let stem_part = &app.core.input.value[..idx];
                let ext_part = &app.core.input.value[idx..];
                Line::from(vec![
                    Span::styled(
                        stem_part,
                        Style::default()
                            .bg(theme::accent_primary())
                            .fg(theme::selection_fg()),
                    ),
                    Span::raw(ext_part),
                ])
            } else {
                Line::from(vec![Span::styled(
                    &app.core.input.value,
                    Style::default()
                        .bg(theme::accent_primary())
                        .fg(theme::selection_fg()),
                )])
            }
        } else {
            Line::from(vec![Span::styled(
                &app.core.input.value,
                Style::default()
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg()),
            )])
        };
        f.render_widget(Paragraph::new(text), inner);
    } else {
        f.render_widget(&app.core.input, inner);
    }
}

pub fn draw_new_folder_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" New Folder ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::success()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.core.input, inner);
}

pub fn draw_new_file_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" New File ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::success()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.core.input, inner);
}

pub fn draw_bulk_rename_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Bulk Rename ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::info()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let label_style = Style::default().fg(theme::muted());
    let input_style = Style::default().fg(theme::fg());

    let file_count = if let AppMode::BulkRename { ref files, .. } = app.core.mode {
        files.len()
    } else {
        0
    };

    let mut content = Vec::new();
    content.push(Line::from(vec![Span::styled(format!("{} files selected - Enter to apply", file_count), Style::default().fg(theme::info()))]));
    content.push(Line::from(vec![Span::raw("")]));
    content.push(Line::from(vec![Span::styled("Pattern: ", label_style)]));
    content.push(Line::from(vec![Span::styled(&app.core.input.value, input_style)]));
    content.push(Line::from(vec![Span::raw("")]));
    content.push(Line::from(vec![Span::styled("Preview (first 5):", label_style)]));

    let mut preview_lines: Vec<String> = Vec::new();
    if let AppMode::BulkRename { ref files, ref pattern, ref replacement, .. } = app.core.mode {
        let re = regex::Regex::new(pattern);
        for (i, f) in files.iter().take(5).enumerate() {
            let name_str = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let new_name = if let Ok(ref re) = re {
                re.replace_all(&name_str, replacement.as_str()).to_string()
            } else {
                name_str.clone()
            };
            let changed = if new_name != name_str { " → " } else { "   " };
            preview_lines.push(format!("  {} {}{}{}", i + 1, name_str, changed, new_name));
        }
        if files.len() > 5 {
            preview_lines.push(format!("  ... and {} more", files.len() - 5));
        }
    }
    for line in preview_lines {
        content.push(Line::from(vec![Span::styled(line, Style::default().fg(theme::muted()))]));
    }

    f.render_widget(Paragraph::new(content), inner);

    let hint_style = Style::default().fg(theme::muted());
    f.render_widget(
        Paragraph::new("Enter = Apply  Esc = Cancel").style(hint_style),
        Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1),
    );
}

pub fn draw_save_as_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 10, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Save As ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::warning()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(&app.core.input, inner);
}

pub fn draw_delete_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    f.render_widget(Clear, area);

    let (title, message) = match &app.core.mode {
        AppMode::DeleteFile(ref path) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            (format!(" Delete {}? ", name), "Confirm deletion? [Y/n]: ".to_string())
        }
        AppMode::Delete(ref mode) if mode == "trash" => {
            (" Trash selected items? ".to_string(), "Move to trash? [Y/n]: ".to_string())
        }
        _ => {
            (" Delete selected items? ".to_string(), "Permanently delete? [Y/n]: ".to_string())
        }
    };

    let border_color = match &app.core.mode {
        AppMode::Delete(ref mode) if mode == "trash" => theme::warning(),
        _ => theme::danger(),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Message
    f.render_widget(
        Paragraph::new(format!("{}{}", message, app.core.input.value))
            .alignment(Alignment::Center),
        inner,
    );

    // Buttons
    let (mx, my) = app.core.mouse_pos;
    let button_y = inner.y + inner.height.saturating_sub(2);

    let is_hover =
        |bx: u16, len: u16| mx >= inner.x + bx && mx < inner.x + bx + len && my == button_y;

    // [ YES ] at x=5 (width 9)
    // [ NO ]  at x=25 (width 8)

    let yes_style = if is_hover(5, 9) {
        Style::default()
            .bg(theme::danger())
            .fg(theme::selection_fg())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::danger()).add_modifier(Modifier::BOLD)
    };

    let no_style = if is_hover(25, 8) {
        Style::default()
            .bg(theme::selection_bg())
            .fg(theme::selection_fg())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::fg())
    };

    f.render_widget(
        Paragraph::new(" [ YES ] ").style(yes_style),
        Rect::new(inner.x + 5, button_y, 9, 1),
    );

    f.render_widget(
        Paragraph::new(" [ NO ] ").style(no_style),
        Rect::new(inner.x + 25, button_y, 8, 1),
    );
}

pub fn draw_properties_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let mut text = Vec::new();

    if let Some(fs) = app.current_file_state() {
        let target_path = fs
            .list.selection
            .selected
            .and_then(|idx| fs.list.files.get(idx))
            .unwrap_or(&fs.nav.current_path);

        let name = target_path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
            .unwrap_or_else(|| target_path.to_string_lossy().to_string());
        let parent = target_path
            .parent()
            .map(|p: &std::path::Path| p.to_string_lossy().to_string())
            .unwrap_or_default();

        text.push(Line::from(vec![
            Span::styled(
                "Name: ",
                Style::default().fg(theme::accent_secondary()),
            ),
            Span::raw(name),
        ]));
        text.push(Line::from(vec![
            Span::styled(
                "Location: ",
                Style::default().fg(theme::accent_secondary()),
            ),
            Span::raw(parent),
        ]));
        text.push(Line::from(""));

        if let Some(meta) = fs.list.metadata.get(target_path) {
            let type_str = if meta.is_dir { "Folder" } else { "File" };
            text.push(Line::from(vec![
                Span::styled(
                    "Type: ",
                    Style::default().fg(theme::accent_secondary()),
                ),
                Span::raw(type_str),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Size: ",
                    Style::default().fg(theme::accent_secondary()),
                ),
                Span::raw(format_size(meta.size)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Modified: ",
                    Style::default().fg(theme::accent_secondary()),
                ),
                Span::raw(format_time(meta.modified)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Created: ",
                    Style::default().fg(theme::accent_secondary()),
                ),
                Span::raw(format_time(meta.created)),
            ]));
            text.push(Line::from(vec![
                Span::styled(
                    "Permissions: ",
                    Style::default().fg(theme::accent_secondary()),
                ),
                Span::raw(format_permissions(meta.permissions)),
            ]));
} else if fs.nav.remote_session.is_none() {
                if let Some(m) = fs.list.metadata.get(target_path) {
                    let is_dir = m.is_dir;
                    text.push(Line::from(vec![
                        Span::styled(
                            "Type: ",
                            Style::default().fg(theme::accent_secondary()),
                        ),
                        Span::raw(if is_dir { "Folder" } else { "File" }),
                    ]));
                    text.push(Line::from(vec![
                        Span::styled(
                            "Size: ",
                            Style::default().fg(theme::accent_secondary()),
                        ),
                        Span::raw(format_size(m.size)),
                    ]));
                    text.push(Line::from(vec![
                        Span::styled(
                            "Modified: ",
                            Style::default().fg(theme::accent_secondary()),
                        ),
                        Span::raw(format_time(m.modified)),
                    ]));
                } else {
                    text.push(Line::from(Span::styled(
                        "No metadata available",
                        Style::default().fg(theme::muted()),
                    )));
                }
        } else {
            text.push(Line::from(Span::styled(
                "No metadata available (Remote)",
                Style::default().fg(theme::muted()),
            )));
        }
    }

    let block = Block::default()
        .title(" Properties ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::accent_primary()));
    f.render_widget(Paragraph::new(text).block(block), area);
}
