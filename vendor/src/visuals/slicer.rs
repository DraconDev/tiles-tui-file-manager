use crate::visuals::tiles::Tile;
use image::{GenericImageView, RgbaImage};

/// A utility for slicing images into reusable `Tile` assets.
///
/// This is a core part of the "Semantic UI" system, allowing high-resolution
/// assets to be automatically converted into TUI-compatible tiles.
pub struct ImageSlicer;

impl ImageSlicer {
    /// Slices an image into a regular grid of tiles.
    ///
    /// # Arguments
    /// * `img` - The source RGBA image.
    /// * `tile_w`, `tile_h` - Dimensions of each tile in pixels.
    /// * `cell_w`, `cell_h` - Terminal cell occupancy for each tile.
    /// * `start_id` - The starting asset ID for the generated tiles.
    pub fn slice_grid(
        img: &RgbaImage,
        tile_w: u32,
        tile_h: u32,
        cell_w: u16,
        cell_h: u16,
        start_id: u32,
    ) -> Vec<Tile> {
        let (width, height) = img.dimensions();
        let mut tiles = Vec::new();
        let mut id = start_id;

        for y in (0..height).step_by(tile_h as usize) {
            for x in (0..width).step_by(tile_w as usize) {
                // Handle edge cases where image size is not a multiple of tile size
                let actual_w = std::cmp::min(tile_w, width - x);
                let actual_h = std::cmp::min(tile_h, height - y);

                let sub_img = img.view(x, y, actual_w, actual_h).to_image();
                let data = sub_img.into_raw();

                tiles.push(Tile::new(id, actual_w, actual_h, data, cell_w, cell_h));
                id += 1;
            }
        }

        tiles
    }

    /// Slices an image using 9-slice logic (3x3 grid with flexible center).
    ///
    /// This is used for scalable UI elements like borders and buttons, where
    /// the corners remain fixed and the edges/center are stretched.
    ///
    /// # Arguments
    /// * `corner_w`, `corner_h` - Size of the corner patches in pixels.
    pub fn slice_9(
        img: &RgbaImage,
        corner_w: u32,
        corner_h: u32,
        cell_w: u16,
        cell_h: u16,
        start_id: u32,
    ) -> Vec<Tile> {
        let (width, height) = img.dimensions();
        let center_w = width.saturating_sub(corner_w * 2);
        let center_h = height.saturating_sub(corner_h * 2);

        let mut tiles = Vec::new();
        let mut id = start_id;

        let xs = [0, corner_w, width - corner_w];
        let ys = [0, corner_h, height - corner_h];
        let ws = [corner_w, center_w, corner_w];
        let hs = [corner_h, center_h, corner_h];

        for (row, &y) in ys.iter().enumerate() {
            for (col, &x) in xs.iter().enumerate() {
                let w = ws[col];
                let h = hs[row];
                if w == 0 || h == 0 {
                    continue;
                }

                let sub_img = img.view(x, y, w, h).to_image();
                let data = sub_img.into_raw();

                tiles.push(Tile::new(id, w, h, data, cell_w, cell_h));
                id += 1;
            }
        }

        tiles
    }
}

/// A semantic wrapper for a 3x3 grid of tiles used for scaling boxes/borders.
///
/// `NineSlice` allows you to render graphical boxes that scale perfectly without
/// distorting the corners.
///
/// # Example
/// ```rust,ignore
/// let slice = NineSlice::new(tile_ids);
/// slice.render(compositor, x, y, width, height, z_index);
/// ```
#[derive(Clone)]
pub struct NineSlice {
    /// IDs of the 9 tiles in row-major order:
    /// [TL, TC, TR,
    ///  ML, MC, MR,
    ///  BL, BC, BR]
    pub tiles: Vec<u32>,
}

impl NineSlice {
    pub fn new(tiles: Vec<u32>) -> Self {
        assert_eq!(tiles.len(), 9, "NineSlice must have exactly 9 tiles");
        Self { tiles }
    }

    pub fn top_left(&self) -> u32 {
        self.tiles[0]
    }
    pub fn top_center(&self) -> u32 {
        self.tiles[1]
    }
    pub fn top_right(&self) -> u32 {
        self.tiles[2]
    }
    pub fn mid_left(&self) -> u32 {
        self.tiles[3]
    }
    pub fn mid_center(&self) -> u32 {
        self.tiles[4]
    }
    pub fn mid_right(&self) -> u32 {
        self.tiles[5]
    }
    pub fn bot_left(&self) -> u32 {
        self.tiles[6]
    }
    pub fn bot_center(&self) -> u32 {
        self.tiles[7]
    }
    pub fn bot_right(&self) -> u32 {
        self.tiles[8]
    }

    /// Renders the 9-slice into the compositor at the given bounds.
    /// If `use_graphics` is false, it can fallback to a text-based border.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        compositor: &mut crate::compositor::engine::Compositor,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        z_index: i32,
        use_graphics: bool,
    ) {
        if w < 2 || h < 2 {
            return;
        }

        if !use_graphics {
            // FALLBACK: Unicode Box Drawing
            // ╔═══╗
            // ║   ║
            // ╚═══╝
            compositor.draw_text(
                "╔",
                x,
                y,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Styles::empty(),
            );
            compositor.draw_text(
                "╗",
                x + w - 1,
                y,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Styles::empty(),
            );
            compositor.draw_text(
                "╚",
                x,
                y + h - 1,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Styles::empty(),
            );
            compositor.draw_text(
                "╝",
                x + w - 1,
                y + h - 1,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Color::Reset,
                crate::compositor::plane::Styles::empty(),
            );

            for i in 1..(w - 1) {
                compositor.draw_text(
                    "═",
                    x + i,
                    y,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Styles::empty(),
                );
                compositor.draw_text(
                    "═",
                    x + i,
                    y + h - 1,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Styles::empty(),
                );
            }
            for i in 1..(h - 1) {
                compositor.draw_text(
                    "║",
                    x,
                    y + i,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Styles::empty(),
                );
                compositor.draw_text(
                    "║",
                    x + w - 1,
                    y + i,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Color::Reset,
                    crate::compositor::plane::Styles::empty(),
                );
            }
            return;
        }

        // Graphical Path
        // Corners
        compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
            asset_id: self.top_left(),
            x,
            y,
            z_index,
            is_image: true,
            ..Default::default()
        });
        compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
            asset_id: self.top_right(),
            x: x + w - 1,
            y,
            z_index,
            is_image: true,
            ..Default::default()
        });
        compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
            asset_id: self.bot_left(),
            x,
            y: y + h - 1,
            z_index,
            is_image: true,
            ..Default::default()
        });
        compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
            asset_id: self.bot_right(),
            x: x + w - 1,
            y: y + h - 1,
            z_index,
            is_image: true,
            ..Default::default()
        });

        // Horizontal edges
        for i in 1..(w - 1) {
            compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
                asset_id: self.top_center(),
                x: x + i,
                y,
                z_index,
                is_image: true,
                ..Default::default()
            });
            compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
                asset_id: self.bot_center(),
                x: x + i,
                y: y + h - 1,
                z_index,
                is_image: true,
                ..Default::default()
            });
        }

        // Vertical edges
        for i in 1..(h - 1) {
            compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
                asset_id: self.mid_left(),
                x,
                y: y + i,
                z_index,
                is_image: true,
                ..Default::default()
            });
            compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
                asset_id: self.mid_right(),
                x: x + w - 1,
                y: y + i,
                z_index,
                is_image: true,
                ..Default::default()
            });
        }

        // Center
        for row in 1..(h - 1) {
            for col in 1..(w - 1) {
                compositor.add_tile_placement(crate::compositor::engine::TilePlacement {
                    asset_id: self.mid_center(),
                    x: x + col,
                    y: y + row,
                    z_index,
                    is_image: true,
                    ..Default::default()
                });
            }
        }
    }
}
