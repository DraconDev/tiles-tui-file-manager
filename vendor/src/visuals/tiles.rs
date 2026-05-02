use crate::visuals::image::{ImageOptions, ImageProtocol};
use std::io::{self, Write};

/// A graphical tile that can be transmitted once and displayed multiple times.
pub struct Tile {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub cell_width: u16,
    pub cell_height: u16,
}

impl Tile {
    pub fn new(
        id: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        cell_width: u16,
        cell_height: u16,
    ) -> Self {
        Self {
            id,
            width,
            height,
            data,
            cell_width,
            cell_height,
        }
    }

    /// Transmit the tile to the terminal.
    pub fn transmit<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        ImageProtocol::transmit_rgba(writer, &self.data, self.width, self.height, self.id)
    }

    /// Place the tile at a specific character coordinate.
    pub fn place<W: Write>(&self, writer: &mut W, x: u16, y: u16, z_index: i32) -> io::Result<()> {
        let options = ImageOptions {
            x: Some(x as u32), // Note: Kitty protocol usually uses pixels for x/y if specified,
            // but 'p' action uses cell position by default if x/y are omitted.
            y: Some(y as u32),
            z_index,
            columns: Some(self.cell_width as u32),
            rows: Some(self.cell_height as u32),
            ..Default::default()
        };
        ImageProtocol::put_image(writer, self.id, options)
    }
}

/// A collection of tiles.
pub struct TileSet {
    pub tiles: Vec<Tile>,
}

impl TileSet {
    pub fn new() -> Self {
        Self { tiles: Vec::new() }
    }
}

impl Default for TileSet {
    fn default() -> Self {
        Self::new()
    }
}

impl TileSet {
    pub fn add(&mut self, tile: Tile) {
        self.tiles.push(tile);
    }

    pub fn transmit_all<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for tile in &self.tiles {
            tile.transmit(writer)?;
        }
        Ok(())
    }
}

/// A grid of tiles.
pub struct TileMap {
    pub width: u16,
    pub height: u16,
    pub data: Vec<Option<u32>>, // Stores tile IDs
}

impl TileMap {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            data: vec![None; (width * height) as usize],
        }
    }

    pub fn set_tile(&mut self, x: u16, y: u16, tile_id: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.data[idx] = Some(tile_id);
        }
    }

    pub fn render<W: Write>(
        &self,
        writer: &mut W,
        tileset: &TileSet,
        start_x: u16,
        start_y: u16,
        z_index: i32,
    ) -> io::Result<()> {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                if let Some(id) = self.data[idx] {
                    if let Some(tile) = tileset.tiles.iter().find(|t| t.id == id) {
                        // Calculate character position based on tile size
                        let px = start_x + x * tile.cell_width;
                        let py = start_y + y * tile.cell_height;
                        tile.place(writer, px, py, z_index)?;
                    }
                }
            }
        }
        Ok(())
    }
}
