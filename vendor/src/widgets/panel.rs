use crate::compositor::engine::TilePlacement;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use std::sync::{Arc, Mutex};

pub struct Panel {
    title: String,
    tile_queue: Arc<Mutex<Vec<TilePlacement>>>,
    border_color: Color,
}

impl Panel {
    pub fn new(title: impl Into<String>, tile_queue: Arc<Mutex<Vec<TilePlacement>>>) -> Self {
        Self {
            title: title.into(),
            tile_queue,
            border_color: Color::Cyan, // Default
        }
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Computes the inner area inside the panel (accounting for borders)
    pub fn inner(&self, area: Rect) -> Rect {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(self.title.as_str());
        block.inner(area)
    }
}

impl Widget for Panel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 1. Draw Graphical Border using Tiles
        if let Ok(mut q) = self.tile_queue.lock() {
            let base_id = 8000 + (area.y as u32 * 100) + area.x as u32;

            // Draw Header Background (Gradient)
            let header_bg_id = 3001;

            q.push(TilePlacement {
                asset_id: header_bg_id,
                is_image: true,
                x: area.x,
                y: area.y,
                z_index: 0,
                cols: Some(area.width),
                rows: Some(1),
                placement_id: Some(base_id),
                ..Default::default()
            });
        }

        // 2. Render Standard Ratatui Block on top
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border_color))
            .title(self.title.as_str())
            .title_style(Style::default().fg(Color::Yellow));

        block.render(area, buf);
    }
}
