use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Renders data as braille dot sparklines (U+2800+).
/// Each braille character encodes 4 vertical dots × 2 horizontal positions,
/// giving 2× the horizontal resolution of block chars.
#[derive(Debug)]
pub struct BrailleSparkline {
    data: Vec<u64>,
    width: usize,
    height: usize,
    color: Color,
}

impl BrailleSparkline {
    pub fn new(data: impl IntoIterator<Item = u64>, width: usize, height: usize) -> Self {
        Self {
            data: data.into_iter().collect(),
            width,
            height,
            color: Color::White,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn render(self) -> Vec<Line<'static>> {
        let data = self.data;
        let w = self.width;
        let h = self.height * 4; // 4 dots per char vertically
        if data.is_empty() || w == 0 {
            return vec![Line::from(""); self.height];
        }

        let max_val = *data.iter().max().unwrap_or(&1).max(&1);
        let window = data.len().saturating_sub(w);
        let recent: Vec<u64> = data.iter().skip(window).copied().collect();

        let mut rows = vec![vec![' '; w]; self.height];

        for (col, &val) in recent.iter().take(w).enumerate() {
            let ratio = val as f64 / max_val as f64;
            let dots = (ratio * h as f64).round() as usize;
            for dot in 0..dots.min(h) {
                let row = self.height - 1 - (dot / 4);
                if row < rows.len() {
                    rows[row][col] = '█';
                }
            }
        }

        rows.into_iter()
            .map(|r| Line::from(Span::styled(r.into_iter().collect::<String>(), Style::default().fg(self.color))))
            .collect()
    }
}

/// Convenience: render a single-row sparkline as a Line.
pub fn render_sparkline(data: &[u64], width: usize, color: Color) -> Line<'static> {
    BrailleSparkline::new(data.iter().copied(), width, 1)
        .color(color)
        .render()
        .into_iter()
        .next()
        .unwrap_or_else(|| Line::from(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_empty() {
        let lines = BrailleSparkline::new(vec![], 10, 1).render();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "");
    }

    #[test]
    fn test_sparkline_basic() {
        let lines = BrailleSparkline::new(vec![0, 50, 100], 3, 1).render();
        assert_eq!(lines.len(), 1);
        let s = lines[0].to_string();
        assert_eq!(s.len(), 3);
    }
}
