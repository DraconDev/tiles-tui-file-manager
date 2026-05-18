#![allow(unused_imports)]

//! Misc UI — style color modal, reset settings modal, highlight modal, drag ghost, format time.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Paragraph, Row, Table, TableState, Widget,
    },
    Frame,
};
use std::time::SystemTime;

use crate::app::App;
use crate::ui::theme as theme;
use dracon_terminal_engine::layout::centered_rect;
use dracon_terminal_engine::utils::format_time;
use dracon_terminal_engine::widgets::HotkeyHint;

pub fn draw_style_color_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(64, 9, f.area());
    f.render_widget(Clear, area);

    const STYLE_COLOR_START_INDEX: usize = 7;
    let field_name = match app.settings.settings_index.saturating_sub(STYLE_COLOR_START_INDEX) {
        0 => "Accent Primary",
        1 => "Accent Secondary",
        2 => "Selection Background",
        3 => "Border Active",
        4 => "Border Inactive",
        5 => "Header Accent",
        _ => "Accent Primary",
    };

    let color = {
        let style = theme::style_settings();
        match app.settings.settings_index.saturating_sub(STYLE_COLOR_START_INDEX) {
            0 => style.accent_primary,
            1 => style.accent_secondary,
            2 => style.selection_bg,
            3 => style.border_active,
            4 => style.border_inactive,
            5 => style.header_fg,
            _ => style.accent_primary,
        }
    };

    let block = Block::default()
        .title(format!(" Edit {} ", field_name))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::raw("Current: "),
            Span::styled(
                "■",
                Style::default()
                    .fg(Color::Rgb(color.r, color.g, color.b))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  rgb({}, {}, {})", color.r, color.g, color.b)),
        ]),
        Line::from("Input: #RRGGBB or R,G,B"),
    ];
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(theme::fg())),
        Rect::new(inner.x, inner.y, inner.width, 2),
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent_secondary()));
    f.render_widget(
        Paragraph::new(app.core.input.value.as_str()).block(input_block),
        Rect::new(inner.x, inner.y + 2, inner.width, 3),
    );

    let footer = Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default().fg(theme::selection_fg()).bg(theme::success()),
        ),
        Span::raw(" apply  "),
        Span::styled(" Esc ", Style::default().fg(theme::selection_fg()).bg(theme::accent_primary())),
        Span::raw(" cancel"),
    ]);
    f.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.y + 6, inner.width, 1),
    );

    if let Some((msg, time)) = &app.output.last_action_msg {
        if time.elapsed().as_secs() < 5 && msg.starts_with("Invalid color for ") {
            f.render_widget(
                Paragraph::new(msg.as_str()).style(Style::default().fg(theme::danger())),
                Rect::new(inner.x, inner.y + 7, inner.width, 1),
            );
        }
    }
}

pub fn draw_reset_settings_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(56, 12, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Reset All Settings ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::danger()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from("This resets global settings to defaults."),
        Line::from("Bookmarks and remotes are kept."),
        Line::from(""),
        Line::from(vec![
            Span::raw("Type "),
            Span::styled(
                "RESET",
                Style::default()
                    .fg(theme::warning())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" and press Enter."),
        ]),
    ];
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(theme::fg())),
        Rect::new(inner.x, inner.y, inner.width, 5),
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent_primary()));
    f.render_widget(
        Paragraph::new(app.core.input.value.as_str()).block(input_block),
        Rect::new(inner.x, inner.y + 5, inner.width, 3),
    );

    let footer = Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default().fg(theme::selection_fg()).bg(theme::success()),
        ),
        Span::raw(" apply  "),
        Span::styled(" Esc ", Style::default().fg(theme::selection_fg()).bg(theme::accent_primary())),
        Span::raw(" cancel"),
    ]);
    f.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.y + 9, inner.width, 1),
    );
}

pub fn draw_highlight_modal(f: &mut Frame, _app: &App) {
    // Actually let's use absolute sizing for palette
    let area = Rect::new(
        (f.area().width.saturating_sub(34)) / 2,
        (f.area().height.saturating_sub(5)) / 2,
        34,
        5,
    );

    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Highlight ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::accent_primary()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let colors = [
        (1, " R ", theme::danger()),
        (2, " G ", theme::success()),
        (3, " Y ", theme::warning()),
        (4, " B ", Color::Blue),
        (5, " M ", theme::accent_secondary()),
        (6, " C ", theme::info()),
        (0, " X ", Color::Reset),
    ];

    let mut spans = Vec::new();
    for (i, (code, label, color)) in colors.iter().enumerate() {
        let style = if *code == 0 {
            Style::default().bg(theme::muted()).fg(theme::fg())
        } else {
            Style::default().bg(*color).fg(theme::selection_fg())
        };
        spans.push(Span::styled(*label, style));
        if i < colors.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    f.render_widget(
        Paragraph::new("1   2   3   4   5   6   0")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(theme::muted())),
        Rect::new(inner.x, inner.y + 2, inner.width, 1),
    );
}

pub fn format_modified_time(time: SystemTime, smart: bool) -> String {
    use chrono::{DateTime, Local};
    let dt: DateTime<Local> = time.into();
    let now = Local::now();

    if smart {
        let duration = now.signed_duration_since(dt);
        let days = duration.num_days();
        if days == 0 {
            let total_minutes = duration.num_minutes();
            if total_minutes < 1 {
                "just now".to_string()
            } else if total_minutes < 60 {
                format!("{}m ago", total_minutes)
            } else {
                format!("{}h ago", duration.num_hours())
            }
        } else if days == 1 {
            "yesterday".to_string()
        } else if days < 7 {
            format!("{}d ago", days)
        } else if days < 30 {
            format!("{}w ago", days / 7)
        } else if days < 365 {
            format!("{}mo ago", days / 30)
        } else {
            format!("{}y ago", days / 365)
        }
    } else if dt.date_naive() == now.date_naive() {
        dt.format("%H:%M:%S").to_string()
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

pub fn draw_drag_ghost(f: &mut Frame, app: &App) {
    if let Some(path) = &app.drag.drag_source {
        let (col, row) = app.core.mouse_pos;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        // Truncate name if too long
        let max_len = 20;
        let display_name = if name.len() > max_len {
            format!("{}...", &name[..max_len])
        } else {
            name.to_string()
        };

        let text = format!(" {} ", display_name);
        let width = text.len() as u16;

        // Draw slightly offset from cursor
        let x = col
            .saturating_add(2)
            .min(f.area().width.saturating_sub(width));
        let y = row.saturating_add(1).min(f.area().height.saturating_sub(1));

        let area = Rect::new(x, y, width, 1);

        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(Span::styled(
                text,
                Style::default()
                    .bg(theme::accent_primary())
                    .fg(theme::selection_fg())
                    .add_modifier(Modifier::BOLD),
            )),
            area,
        );
    }
}

/// Draw marquee selection rectangle overlay.
pub fn draw_marquee_rect(f: &mut Frame, app: &App) {
    if !app.drag.is_marquee {
        return;
    }
    let Some(rect) = app.drag.marquee_rect() else {
        return;
    };

    let area = f.area();
    let x = rect.min_col.min(area.width.saturating_sub(1));
    let y = rect.min_row.min(area.height.saturating_sub(1));
    let w = rect.max_col.saturating_sub(rect.min_col).saturating_add(1);
    let h = rect.max_row.saturating_sub(rect.min_row).saturating_add(1);

    // Clamp to screen bounds
    let w = w.min(area.width.saturating_sub(x));
    let h = h.min(area.height.saturating_sub(y));

    if w == 0 || h == 0 {
        return;
    }

    let marquee_area = Rect::new(x, y, w, h);

    // Border frame only — transparent background, no fill
    let block = ratatui::widgets::Block::new()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(
            Style::default()
                .fg(theme::accent_primary())
                .add_modifier(Modifier::BOLD),
        )
        .border_type(ratatui::widgets::BorderType::Rounded);

    f.render_widget(block, marquee_area);
}
