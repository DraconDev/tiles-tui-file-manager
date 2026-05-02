use crate::compositor::engine::TilePlacement;
use crate::visuals::assets::Icon;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use std::sync::{Arc, Mutex};

pub struct Button {
    label: String,
    is_active: bool,
    tile_queue: Arc<Mutex<Vec<TilePlacement>>>,
}

impl Button {
    pub fn new(
        label: impl Into<String>,
        is_active: bool,
        tile_queue: Arc<Mutex<Vec<TilePlacement>>>,
    ) -> Self {
        Self {
            label: label.into(),
            is_active,
            tile_queue,
        }
    }
}

impl Widget for Button {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 1. Push Tile Placements (The "Visuals")
        if area.width >= 3 {
            if let Ok(mut q) = self.tile_queue.lock() {
                let base_id = 7000 + (area.y as u32 * 100) + area.x as u32;

                // Left Slice
                q.push(TilePlacement {
                    asset_id: Icon::ButtonLeft as u32,
                    is_image: true,
                    x: area.x,
                    y: area.y,
                    z_index: 2,
                    cols: Some(1),
                    rows: Some(1),
                    placement_id: Some(base_id),
                    ..Default::default()
                });

                // Middle Slice (stretched)
                q.push(TilePlacement {
                    asset_id: Icon::ButtonMid as u32,
                    is_image: true,
                    x: area.x + 1,
                    y: area.y,
                    z_index: 2,
                    cols: Some(area.width.saturating_sub(2)),
                    rows: Some(1),
                    placement_id: Some(base_id + 1),
                    ..Default::default()
                });

                // Right Slice
                q.push(TilePlacement {
                    asset_id: Icon::ButtonRight as u32,
                    is_image: true,
                    x: area.x + area.width.saturating_sub(1),
                    y: area.y,
                    z_index: 2,
                    cols: Some(1),
                    rows: Some(1),
                    placement_id: Some(base_id + 2),
                    ..Default::default()
                });
            }
        }

        // 2. Render Text (The "Label") using Ratatui
        let style = if self.is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let paragraph = ratatui::widgets::Paragraph::new(self.label.as_str())
            .alignment(ratatui::layout::Alignment::Center)
            .style(style);

        paragraph.render(area, buf);
    }
}
