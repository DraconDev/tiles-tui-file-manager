use crate::compositor::engine::TilePlacement;
use std::sync::{Arc, Mutex};

/// Defines the asset IDs for a 9-slice UI element.
#[derive(Clone, Copy, Debug)]
pub struct Chrome {
    pub top_left: u32,
    pub top: u32,
    pub top_right: u32,
    pub right: u32,
    pub bottom_right: u32,
    pub bottom: u32,
    pub bottom_left: u32,
    pub left: u32,
    pub center: u32,
    pub header_bg: Option<u32>, // Special override for headers
}

impl Chrome {
    pub const CYBERPUNK: Self = Self {
        top_left: 9000,
        top: 9001,
        top_right: 9002,
        right: 9003,
        bottom_right: 9004,
        bottom: 9005,
        bottom_left: 9006,
        left: 9007,
        center: 9008,
        header_bg: Some(3001),
    };

    pub fn draw(&self, tile_queue: &Arc<Mutex<Vec<TilePlacement>>>, area: ratatui::layout::Rect, z: i32) {
        if let Ok(mut q) = tile_queue.lock() {
            let base_id = (area.y as u32 * 1000) + area.x as u32;

            // Header Background Override (if present)
            if let Some(header_id) = self.header_bg {
                q.push(TilePlacement {
                    asset_id: header_id,
                    is_image: true,
                    x: area.x,
                    y: area.y,
                    z_index: z,
                    cols: Some(area.width),
                    rows: Some(1),
                    placement_id: Some(base_id + 99),
                });
            }

            // We can implement full 9-slice drawing here later if we generate the assets.
            // For now, let's stick to the header glow as the primary "Chrome" feature.
        }
    }
}
