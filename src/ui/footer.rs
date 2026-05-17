//! Footer rendering — status bar with task info.
//! Extracted from ui/mod.rs (Phase 3).

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::ui::theme::THEME;

pub fn draw_stat_bar(
    label: &str,
    value: f32,
    max: f32,
    low_color: Color,
    mid_color: Color,
    label_color: Color,
) -> Line<'static> {
    let width = 10;
    let ratio = (value / max.max(1.0)).clamp(0.0, 1.0);
    let filled = (ratio * width as f32).round() as usize;

    let mut spans = vec![Span::styled(
        format!("{} ", label),
        Style::default().fg(label_color),
    )];

    for i in 0..width {
        let symbol = if i < filled { "█" } else { "░" };
        let color = if ratio < 0.4 {
            low_color
        } else if ratio < 0.7 {
            mid_color
        } else {
            Color::Red // Warning/Danger
        };

        if i < filled {
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        } else {
            spans.push(Span::styled(
                symbol,
                Style::default().fg(Color::Rgb(30, 30, 35)),
            ));
        }
    }

    spans.push(Span::styled(
        format!(" {:>3.0}%", ratio * 100.0),
        Style::default().fg(THEME.fg).add_modifier(Modifier::BOLD),
    ));
    Line::from(spans)
}

