#![allow(dead_code)]

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const BRAILLE_BASE: u32 = 0x2800;

const BIT_MAP: [[u32; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x08, 0x40],
];

fn braille_char(bits: u32) -> char {
    char::from_u32(BRAILLE_BASE + (bits & 0xFF)).unwrap_or('⠀')
}

pub struct BrailleSparkline {
    data: Vec<u64>,
    max_val: Option<u64>,
    color: Color,
    height: u16,
}

impl BrailleSparkline {
    pub fn new(data: impl IntoIterator<Item = u64>) -> Self {
        Self {
            data: data.into_iter().collect(),
            max_val: None,
            color: Color::White,
            height: 2,
        }
    }

    pub fn max_val(mut self, v: u64) -> Self {
        self.max_val = Some(v);
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn height(mut self, h: u16) -> Self {
        self.height = h.max(1);
        self
    }

    pub fn render(self) -> Vec<Line<'static>> {
        if self.data.is_empty() || self.height == 0 {
            return vec![Line::from("")];
        }

        let dot_rows = self.height as usize * 4;
        let braille_cols = self.data.len().div_ceil(2);
        let max = self.max_val.unwrap_or_else(|| self.data.iter().copied().max().unwrap_or(1));
        let max = max.max(1);

        let mut dot_data = vec![vec![false; self.data.len()]; dot_rows];
        for (data_idx, &val) in self.data.iter().enumerate() {
            let normalized = (val as f64 / max as f64 * dot_rows as f64).round() as usize;
            let filled = normalized.min(dot_rows);
            for row in dot_data.iter_mut().take(filled) {
                row[data_idx] = true;
            }
        }

        let style = Style::default().fg(self.color);
        let mut lines = Vec::with_capacity(self.height as usize);

        for term_row in (0..self.height as usize).rev() {
            let mut row_str = String::with_capacity(braille_cols);
            for col in 0..braille_cols {
                let mut bits: u32 = 0;
                for (dot_row, dot_row_bits) in BIT_MAP.iter().enumerate() {
                    let abs_dot_row = term_row * 4 + dot_row;
                    for (sub_col, &bit_val) in dot_row_bits.iter().enumerate() {
                        let data_idx = col * 2 + sub_col;
                        if data_idx < self.data.len() && dot_data[abs_dot_row][data_idx] {
                            bits |= bit_val;
                        }
                    }
                }
                row_str.push(braille_char(bits));
            }
            lines.push(Line::from(Span::styled(row_str, style)));
        }

        lines
    }
}

pub fn render_sparkline(data: impl IntoIterator<Item = u64>, max_val: Option<u64>, color: Color, height: u16) -> Vec<Line<'static>> {
    BrailleSparkline::new(data)
        .max_val(max_val.unwrap_or(0))
        .color(color)
        .height(height)
        .render()
}
