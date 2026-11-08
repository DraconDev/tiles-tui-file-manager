use crate::ui::theme as theme;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct Sparkline {
    data: Vec<u64>,
    width: usize,
    color: Color,
}

impl Sparkline {
    pub fn new(data: impl IntoIterator<Item = u64>, width: usize) -> Self {
        Self {
            data: data.into_iter().collect(),
            width,
            color: theme::fg(),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn render(self) -> Line<'static> {
        if self.data.is_empty() || self.width == 0 {
            return Line::from("");
        }
        let max_val = self.data.iter().copied().max().unwrap_or(1).max(1);
        let recent: Vec<u64> = self.data.iter().copied().skip(self.data.len().saturating_sub(self.width)).collect();
        let filled: String = recent
            .iter()
            .map(|&v| {
                let ratio = v as f64 / max_val as f64;
                if ratio > 0.75 {
                    '▓'
                } else if ratio > 0.5 {
                    '▒'
                } else if ratio > 0.25 {
                    '░'
                } else {
                    '·'
                }
            })
            .collect();
        // Pad to the requested width so callers can rely on the line
        // filling `width` columns (previously short data gave a short line).
        let width = usize::try_from(self.width).unwrap_or(usize::MAX);
        let padded = format!("{:>width$}", filled, width = width);
        Line::from(Span::raw(padded)).style(Style::default().fg(self.color))
    }
}
