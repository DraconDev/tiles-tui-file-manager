use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub struct HotkeyHint;

impl HotkeyHint {
    pub fn render<'a>(key: &'a str, label: &'a str, color: Color) -> Vec<Span<'a>> {
        vec![
            Span::styled(
                format!(" {} ", key),
                Style::default()
                    .bg(color)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {} ", label), Style::default().fg(Color::White)),
            Span::raw("  "),
        ]
    }
}
