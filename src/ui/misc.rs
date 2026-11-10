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
    let area = crate::ui::centered_modal(64, 9, f.area());
    f.render_widget(Clear, area);

    // The modal edits a custom-color row, so the label and the value both come
    // from the canonical field table. It previously carried its own index base
    // (7), which never matched the settings rows (15..=20): every field showed
    // "Accent Primary" because the offset always fell through to the fallback.
    let field_name = theme::style_field_name(app.settings.settings_index).unwrap_or("Color");
    let color = theme::style_field_color(&theme::style_settings(), app.settings.settings_index)
        .unwrap_or_else(|| theme::style_settings().accent_primary);

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
    let area = crate::ui::centered_modal(56, 13, f.area());
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
    // Clamp to frame: a hard-coded 34x5 overflowed small terminals.
    let mw = 34u16.min(f.area().width);
    let mh = 5u16.min(f.area().height);
    let area = Rect::new(
        (f.area().width.saturating_sub(mw)) / 2,
        (f.area().height.saturating_sub(mh)) / 2,
        mw,
        mh,
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
        // Truncate name if too long (char-wise: byte slicing would panic
        // when byte 20 falls inside a multibyte character, e.g. CJK names).
        let max_chars = 20;
        let display_name = if name.chars().count() > max_chars {
            format!("{}...", name.chars().take(max_chars).collect::<String>())
        } else {
            name.to_string()
        };

        let text = format!(" {} ", display_name);
        // Display width, not byte length: CJK/emoji filenames would size
        // the ghost to their UTF-8 length (3-4 cells per character).
        let width = unicode_width::UnicodeWidthStr::width(text.as_str()) as u16;

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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn drag_ghost_width_uses_display_columns() {
        // 9 CJK chars: display width 18 (+2 padding = 20 cells) vs 27 bytes
        // (+2 = 29). The ghost box must be sized by display columns: with the
        // cursor near the right edge, byte length would clamp x further left
        // (60 vs 51 in an 80-wide buffer).
        // Global theme style is shared across tests: serialize.
        let _guard = crate::ui::theme::STYLE_TEST_LOCK.lock().unwrap();
        // Capture the accent before rendering: a concurrent theme test
        // mutating the global style must not change the expected color
        // between the draw and the assertion.
        let accent = crate::ui::theme::accent_primary();
        let mut app = App::new();
        app.drag.drag_source = Some(std::path::PathBuf::from("/tmp/日本語ドキュメント"));
        app.core.mouse_pos = (78, 0);
        let backend = TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| draw_drag_ghost(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Ghost row is y=1 (mouse row + 1). The box must span the full
        // display width (20 columns): measure first..last accent cell —
        // wide-char continuation cells do not count as separate cells.
        let mut first: Option<u16> = None;
        let mut last = 0u16;
        for x in 0..80 {
            let cell = buffer
                .cell(ratatui::layout::Position { x, y: 1 })
                .expect("ghost row cell in bounds");
            if cell.bg == accent {
                if first.is_none() {
                    first = Some(x);
                }
                last = x;
            }
        }
        let first = first.expect("ghost must render");
        // x = min(78 + 2, 80 - 20) = 60 with display width; byte length gives 51.
        assert_eq!(first, 60, "ghost must be clamped by display width, not byte length");
        assert_eq!(
            last - first + 1,
            20,
            "ghost width must be display columns + padding, not bytes"
        );
    }
    /// The modal used its own index base (7) while the settings rows number
    /// from 15, so every editable color rendered as "Accent Primary". Render
    /// the modal at each canonical row and require the title and the swatch to
    /// name that row's field and color.
    #[test]
    fn style_color_modal_shows_the_selected_field_and_color() {
        let _guard = crate::ui::theme::STYLE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let fields = |s: &theme::ThemeStyle| theme::style_color_rows(s);
        let style = theme::style_settings();
        let rows = fields(&style);
        assert_eq!(rows.len(), theme::STYLE_COLOR_FIELD_COUNT);

        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        for (expected_index, (label, color)) in rows.iter().enumerate() {
            let settings_index = theme::STYLE_COLOR_START_INDEX + expected_index;
            let mut app = App::new();
            app.settings.settings_section = crate::app::SettingsSection::Style;
            app.settings.settings_index = settings_index;
            app.core.input.value = "#000000".to_string();

            terminal
                .draw(|f| draw_style_color_modal(f, &app))
                .unwrap();
            let buffer = terminal.backend().buffer().clone();

            // The block title carries the field label. The modal is centered,
            // so scan the whole buffer rather than assuming row 0.
            let mut title = String::new();
            for y in 0..24 {
                for x in 0..80 {
                    if let Some(cell) = buffer.cell(ratatui::layout::Position { x, y }) {
                        title.push_str(cell.symbol());
                    }
                }
                title.push('\n');
            }
            assert!(
                title.contains(&format!("Edit {} ", label)),
                "index {settings_index} must title the modal {:?}, got {title:?}",
                label
            );

            // The swatch must paint that field's color.
            let mut found = false;
            for y in 0..24 {
                for x in 0..80 {
                    if let Some(cell) = buffer.cell(ratatui::layout::Position { x, y }) {
                        if cell.fg == color.to_color() && cell.symbol() == "■" {
                            found = true;
                        }
                    }
                }
            }
            assert!(
                found,
                "index {settings_index} ({label}) must render its color {:?}",
                color
            );
        }
    }

    /// Distinct rows must actually correspond to distinct fields — the stale
    /// base collapsed all six onto "Accent Primary", which this catches even
    /// if the renderer ignored the title.
    #[test]
    fn style_color_rows_map_one_to_one_onto_settings_indices() {
        let style = theme::ThemeStyle::preset_legacy_red();
        let rows = theme::style_color_rows(&style);
        for (offset, (row_label, row_color)) in rows.iter().enumerate() {
            let index = theme::STYLE_COLOR_START_INDEX + offset;
            let named = theme::style_field_name(index).expect("row must have a name");
            assert_eq!(named, *row_label, "row {offset} label must match its index");
            let color = theme::style_field_color(&style, index).expect("row must have a color");
            assert_eq!(color, *row_color, "row {offset} color must match its index");
        }
        assert!(
            theme::style_field_name(theme::STYLE_COLOR_START_INDEX + theme::STYLE_COLOR_FIELD_COUNT).is_none(),
            "the first preset index must not be reported as a color row"
        );
    }

}