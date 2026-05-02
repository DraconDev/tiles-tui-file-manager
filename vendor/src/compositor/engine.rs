use crate::compositor::plane::{Cell, Color, Plane, Styles};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub struct TileAsset {
    pub id: u32,
    pub cells: Vec<Cell>, // Grid of characters/colors
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug)]
pub struct ImageAsset {
    pub id: u32,
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct TilePlacement {
    pub asset_id: u32,
    pub is_image: bool,
    pub x: u16, // Column
    pub y: u16, // Row
    pub z_index: i32,
    pub opacity: f32,
    pub cols: Option<u16>, // Stretch if provided
    pub rows: Option<u16>, // Stretch if provided
    pub placement_id: Option<u32>,
}

impl Default for TilePlacement {
    fn default() -> Self {
        Self {
            asset_id: 0,
            is_image: false,
            x: 0,
            y: 0,
            z_index: 0,
            opacity: 1.0,
            cols: None,
            rows: None,
            placement_id: None,
        }
    }
}

pub struct Compositor {
    pub planes: Vec<Plane>,
    pub tile_assets: HashMap<u32, TileAsset>,
    pub image_assets: HashMap<u32, ImageAsset>,
    pub tile_placements: Vec<TilePlacement>,
    width: u16,
    height: u16,
    last_frame: Vec<Cell>,
    transmitted_assets: HashSet<u32>,
    pub graphics_enabled: bool,
    pub time: f32,
}

impl Compositor {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            planes: Vec::new(),
            tile_assets: HashMap::new(),
            image_assets: HashMap::new(),
            tile_placements: Vec::new(),
            width,
            height,
            last_frame: vec![Cell::default(); (width * height) as usize],
            transmitted_assets: HashSet::new(),
            graphics_enabled: crate::utils::supports_kitty_graphics(),
            time: 0.0,
        }
    }

    pub fn tick(&mut self, delta: f32) {
        self.time += delta;
    }

    /// Returns the topmost visible plane at the given global coordinates.
    pub fn hit_test(&self, x: u16, y: u16) -> Option<&Plane> {
        // Iterate planes in reverse (topmost first)
        for plane in self.planes.iter().rev() {
            if !plane.visible {
                continue;
            }
            if x >= plane.x
                && x < plane.x + plane.width
                && y >= plane.y
                && y < plane.y + plane.height
            {
                // Check if the cell at that position is not transparent
                let lx = x - plane.x;
                let ly = y - plane.y;
                let idx = (ly * plane.width + lx) as usize;
                if !plane.cells[idx].transparent {
                    return Some(plane);
                }
            }
        }
        None
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn add_plane(&mut self, plane: Plane) {
        self.planes.push(plane);
        self.sort_planes();
    }

    pub fn add_tile_asset(&mut self, id: u32, cells: Vec<Cell>, width: u16, height: u16) {
        self.tile_assets.insert(
            id,
            TileAsset {
                id,
                cells,
                width,
                height,
            },
        );
    }

    pub fn add_image_asset(&mut self, id: u32, data: Vec<u8>, width: u32, height: u32) {
        self.image_assets.insert(
            id,
            ImageAsset {
                id,
                data,
                width,
                height,
            },
        );
        self.transmitted_assets.remove(&id);
    }

    pub fn add_tile_placement(&mut self, placement: TilePlacement) {
        self.tile_placements.push(placement);
    }

    /// High-level helper to draw text into the compositor's text layers.
    /// This creates or updates a plane for the text.
    pub fn draw_text(&mut self, text: &str, x: u16, y: u16, fg: Color, bg: Color, style: Styles) {
        let mut plane = Plane::new(999, text.len() as u16, 1); // Temporary ID for label layer
        plane.x = x;
        plane.y = y;
        plane.z_index = 10; // High z-index for labels

        for (i, c) in text.chars().enumerate() {
            if i < plane.cells.len() {
                plane.cells[i] = Cell {
                    char: c,
                    fg,
                    bg,
                    style,
                    transparent: false,
                    skip: false,
                };
            }
        }
        self.add_plane(plane);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        char: char,
        fg: Color,
        bg: Color,
        style: Styles,
    ) {
        let mut plane = Plane::new(998, width, height); // Temporary ID for shape layer
        plane.x = x;
        plane.y = y;
        plane.z_index = 5; // Mid z-index for shapes

        let cell = Cell {
            char,
            fg,
            bg,
            style,
            transparent: false,
            skip: false,
        };

        for i in 0..plane.cells.len() {
            plane.cells[i] = cell.clone();
        }
        self.add_plane(plane);
    }

    pub fn clear_tile_placements(&mut self) {
        self.tile_placements.clear();
    }

    pub fn force_clear(&mut self) {
        // Keep the base plane if it exists, clear others
        if self.planes.len() > 1 {
            self.planes.truncate(1);
        }
        if let Some(base) = self.planes.first_mut() {
            base.clear();
        }

        self.tile_placements.clear();
        // Fill last_frame with a dummy char to force a full diff-based redraw on next render
        for cell in &mut self.last_frame {
            cell.char = '\x01';
        }
    }

    pub fn draw_ratatui_line(&mut self, line: &ratatui::text::Line, x: u16, y: u16) {
        let mut cur_x = x;
        for span in &line.spans {
            let fg = map_color(span.style.fg.unwrap_or(ratatui::style::Color::Reset));
            let bg = map_color(span.style.bg.unwrap_or(ratatui::style::Color::Reset));
            let mut style = crate::compositor::plane::Styles::empty();
            if span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
            {
                style.insert(crate::compositor::plane::Styles::BOLD);
            }
            if span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::ITALIC)
            {
                style.insert(crate::compositor::plane::Styles::ITALIC);
            }
            if span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
            {
                style.insert(crate::compositor::plane::Styles::UNDERLINE);
            }

            let text = span.content.as_ref();
            let mut plane = Plane::new(997, text.chars().count() as u16, 1);
            plane.x = cur_x;
            plane.y = y;
            plane.z_index = 10;

            for (i, c) in text.chars().enumerate() {
                plane.cells[i] = Cell {
                    char: c,
                    fg,
                    bg,
                    style,
                    transparent: false,
                    skip: false,
                };
            }
            self.add_plane(plane);
            cur_x += text.chars().count() as u16;
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.last_frame = vec![Cell::default(); (width * height) as usize];
    }

    fn sort_planes(&mut self) {
        self.planes.sort_by(|a, b| a.z_index.cmp(&b.z_index));
    }

    pub fn render_to_buffer(&mut self) -> Vec<Cell> {
        let mut final_buffer = vec![
            Cell {
                bg: Color::Rgb(0, 0, 0),
                transparent: false,
                ..Cell::default()
            };
            (self.width * self.height) as usize
        ];

        // 1. Collect all layers
        #[derive(Clone)]
        enum Layer {
            PlaneLayer(usize),
            TileLayer(usize),
        }

        let mut layers = Vec::new();
        for i in 0..self.planes.len() {
            layers.push((self.planes[i].z_index, Layer::PlaneLayer(i)));
        }
        for i in 0..self.tile_placements.len() {
            layers.push((self.tile_placements[i].z_index, Layer::TileLayer(i)));
        }

        layers.sort_by(|a, b| a.0.cmp(&b.0));

        // 2. Composite
        for (_, layer) in layers {
            match layer {
                Layer::PlaneLayer(idx) => {
                    let plane = &self.planes[idx];
                    if !plane.visible {
                        continue;
                    }
                    for py in 0..plane.height {
                        for px in 0..plane.width {
                            let abs_x = plane.x + px;
                            let abs_y = plane.y + py;
                            if abs_x >= self.width || abs_y >= self.height {
                                continue;
                            }

                            let src_idx = (py * plane.width + px) as usize;
                            let dest_idx = (abs_y * self.width + abs_x) as usize;
                            let mut src_cell = plane.cells[src_idx].clone();

                            // Apply Filter if present
                            if let Some(filter) = &plane.filter {
                                filter.apply(&mut src_cell, abs_x, abs_y, self.time);
                            }

                            blend_cells(&mut final_buffer[dest_idx], &src_cell, plane.opacity);
                        }
                    }
                }
                Layer::TileLayer(idx) => {
                    let placement = &self.tile_placements[idx];
                    if !placement.is_image {
                        if let Some(asset) = self.tile_assets.get(&placement.asset_id) {
                            let target_w = placement.cols.unwrap_or(asset.width);
                            let target_h = placement.rows.unwrap_or(asset.height);

                            for ty in 0..target_h {
                                for tx in 0..target_w {
                                    let abs_x = placement.x + tx;
                                    let abs_y = placement.y + ty;
                                    if abs_x >= self.width || abs_y >= self.height {
                                        continue;
                                    }

                                    let src_x =
                                        (tx as f32 / target_w as f32 * asset.width as f32) as u16;
                                    let src_y =
                                        (ty as f32 / target_h as f32 * asset.height as f32) as u16;
                                    let src_idx = (src_y * asset.width + src_x) as usize;
                                    let dest_idx = (abs_y * self.width + abs_x) as usize;

                                    let src_cell = &asset.cells[src_idx];
                                    blend_cells(
                                        &mut final_buffer[dest_idx],
                                        src_cell,
                                        placement.opacity,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        final_buffer
    }

    pub fn render<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let mut final_buffer = vec![
            Cell {
                bg: Color::Rgb(0, 0, 0),
                transparent: false,
                ..Cell::default()
            };
            (self.width * self.height) as usize
        ];
        let mut image_commands = Vec::new();

        // 1. Collect all layers (Planes + TilePlacements) into one sorted sequence
        #[derive(Clone)]
        enum Layer {
            PlaneLayer(usize),
            TileLayer(usize),
        }

        let mut layers = Vec::new();
        for i in 0..self.planes.len() {
            layers.push((self.planes[i].z_index, Layer::PlaneLayer(i)));
        }
        for i in 0..self.tile_placements.len() {
            layers.push((self.tile_placements[i].z_index, Layer::TileLayer(i)));
        }

        layers.sort_by(|a, b| a.0.cmp(&b.0));

        // 2. Composite Layers
        for (_, layer) in layers {
            match layer {
                Layer::PlaneLayer(idx) => {
                    let plane = &self.planes[idx];
                    if !plane.visible {
                        continue;
                    }
                    for py in 0..plane.height {
                        for px in 0..plane.width {
                            let abs_x = plane.x + px;
                            let abs_y = plane.y + py;
                            if abs_x >= self.width || abs_y >= self.height {
                                continue;
                            }

                            let src_idx = (py * plane.width + px) as usize;
                            let dest_idx = (abs_y * self.width + abs_x) as usize;
                            let mut src_cell = plane.cells[src_idx].clone();

                            // Apply Filter if present
                            if let Some(filter) = &plane.filter {
                                filter.apply(&mut src_cell, abs_x, abs_y, self.time);
                            }

                            blend_cells(&mut final_buffer[dest_idx], &src_cell, plane.opacity);
                        }
                    }
                }
                Layer::TileLayer(idx) => {
                    let placement = &self.tile_placements[idx];
                    if placement.is_image {
                        if let Some(_asset) = self.image_assets.get(&placement.asset_id) {
                            image_commands.push(placement.clone());
                        }
                    } else if let Some(asset) = self.tile_assets.get(&placement.asset_id) {
                        let target_w = placement.cols.unwrap_or(asset.width);
                        let target_h = placement.rows.unwrap_or(asset.height);

                        for ty in 0..target_h {
                            for tx in 0..target_w {
                                let abs_x = placement.x + tx;
                                let abs_y = placement.y + ty;
                                if abs_x >= self.width || abs_y >= self.height {
                                    continue;
                                }

                                let src_x =
                                    (tx as f32 / target_w as f32 * asset.width as f32) as u16;
                                let src_y =
                                    (ty as f32 / target_h as f32 * asset.height as f32) as u16;
                                let src_idx = (src_y * asset.width + src_x) as usize;
                                let dest_idx = (abs_y * self.width + abs_x) as usize;

                                let src_cell = &asset.cells[src_idx];
                                blend_cells(
                                    &mut final_buffer[dest_idx],
                                    src_cell,
                                    placement.opacity,
                                );
                            }
                        }
                    }
                }
            }
        }

        // 3. Begin Synchronized Update (Mode 2026)
        write!(writer, "\x1b[?2026h")?;

        // 4. Emit Image Placements (Kitty Protocol)
        for placement in image_commands {
            if let Some(asset) = self.image_assets.get(&placement.asset_id) {
                if !self.transmitted_assets.contains(&asset.id) {
                    crate::visuals::image::ImageProtocol::transmit_rgba(
                        writer,
                        &asset.data,
                        asset.width,
                        asset.height,
                        asset.id,
                    )?;
                    self.transmitted_assets.insert(asset.id);
                }

                let options = crate::visuals::image::ImageOptions {
                    x: Some(placement.x as u32),
                    y: Some(placement.y as u32),
                    z_index: placement.z_index,
                    columns: placement.cols.map(|c| c as u32),
                    rows: placement.rows.map(|r| r as u32),
                    placement_id: placement.placement_id,
                };
                crate::visuals::image::ImageProtocol::put_image(writer, asset.id, options)?;
            }
        }

        // 5. Output Text Buffer to Terminal (Optimized)
        let mut current_fg = Color::Reset;
        let mut current_bg = Color::Reset;
        let mut current_style = Styles::empty();

        // Disable line wrap to prevent scrolling on last cell
        write!(writer, "\x1b[?7l")?;

        for y in 0..self.height {
            let mut line_cursor_moved = false;
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let cell = &final_buffer[idx];
                let last_cell = &self.last_frame[idx];

                if cell.skip {
                    continue;
                }

                // Optimization: Skip if cell matches last frame
                if cell == last_cell {
                    line_cursor_moved = false;
                    continue;
                }

                // Only move cursor if we haven't written yet this line or skipped some chars
                if !line_cursor_moved {
                    write!(writer, "\x1b[{};{}H", y + 1, x + 1)?;
                    line_cursor_moved = true;
                }

                if cell.style != current_style {
                    let diff = cell.style ^ current_style;
                    if diff.contains(Styles::BOLD) {
                        if cell.style.contains(Styles::BOLD) {
                            write!(writer, "\x1b[1m")?;
                        } else {
                            write!(writer, "\x1b[22m")?;
                        }
                    }
                    if diff.contains(Styles::ITALIC) {
                        if cell.style.contains(Styles::ITALIC) {
                            write!(writer, "\x1b[3m")?;
                        } else {
                            write!(writer, "\x1b[23m")?;
                        }
                    }
                    if diff.contains(Styles::UNDERLINE) {
                        if cell.style.contains(Styles::UNDERLINE) {
                            write!(writer, "\x1b[4m")?;
                        } else {
                            write!(writer, "\x1b[24m")?;
                        }
                    }
                    current_style = cell.style;
                }

                if cell.fg != current_fg {
                    match cell.fg {
                        Color::Reset => write!(writer, "\x1b[39m")?,
                        Color::Ansi(c) => write!(writer, "\x1b[38;5;{}m", c)?,
                        Color::Rgb(r, g, b) => write!(writer, "\x1b[38;2;{};{};{}m", r, g, b)?,
                    }
                    current_fg = cell.fg;
                }
                if cell.bg != current_bg {
                    match cell.bg {
                        Color::Reset => write!(writer, "\x1b[49m")?,
                        Color::Ansi(c) => write!(writer, "\x1b[48;5;{}m", c)?,
                        Color::Rgb(r, g, b) => write!(writer, "\x1b[48;2;{};{};{}m", r, g, b)?,
                    }
                    current_bg = cell.bg;
                }
                write!(writer, "{}", cell.char)?;
            }
        }

        // Re-enable line wrap
        write!(writer, "\x1b[?7h")?;

        // End Synchronized Update
        write!(writer, "\x1b[?2026l")?;

        self.last_frame = final_buffer;
        writer.flush()?;
        Ok(())
    }
}

fn blend_cells(dest: &mut Cell, src: &Cell, alpha: f32) {
    if src.transparent || alpha <= 0.0 {
        return;
    }

    if alpha >= 1.0 {
        // Full opacity: simple overwrite
        if src.bg != Color::Reset {
            dest.bg = src.bg;
        }

        // Propagate skip flag (crucial for wide characters)
        if src.skip {
            dest.skip = true;
            dest.char = ' '; // Placeholder for consistency
        } else if src.char != '\0' {
            if is_braille(dest.char) && is_braille(src.char) {
                dest.char = merge_braille(dest.char, src.char);
            } else {
                dest.char = src.char;
            }
            dest.fg = src.fg;
            dest.style = src.style;
            dest.skip = false;
        }
    } else {
        // Alpha Blending
        let blend = |c1: Color, c2: Color, a: f32| -> Color {
            match (c1, c2) {
                (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
                    ((r1 as f32 * (1.0 - a)) + (r2 as f32 * a)) as u8,
                    ((g1 as f32 * (1.0 - a)) + (g2 as f32 * a)) as u8,
                    ((b1 as f32 * (1.0 - a)) + (b2 as f32 * a)) as u8,
                ),
                // Fallback for non-RGB: take source if alpha > 0.5
                (_, c) => {
                    if a > 0.5 {
                        c
                    } else {
                        c1
                    }
                }
            }
        };

        if src.bg != Color::Reset {
            dest.bg = blend(dest.bg, src.bg, alpha);
        }

        if src.skip {
            if alpha > 0.5 {
                dest.skip = true;
                dest.char = ' ';
            }
        } else if src.char != '\0' {
            // Characters are harder to blend. We'll blend the foreground color.
            dest.fg = blend(dest.fg, src.fg, alpha);

            // Only update char if mostly opaque
            if alpha > 0.5 {
                dest.char = src.char;
                dest.style = src.style;
                dest.skip = false;
            }
        }
    }

    dest.transparent = false;
}

fn is_braille(c: char) -> bool {
    let u = c as u32;
    (0x2800..=0x28FF).contains(&u)
}

fn merge_braille(c1: char, c2: char) -> char {
    let b1 = (c1 as u32) & 0xFF;
    let b2 = (c2 as u32) & 0xFF;
    std::char::from_u32(0x2800 | (b1 | b2)).unwrap_or(c1)
}

pub fn map_color(c: ratatui::style::Color) -> Color {
    use ratatui::style::Color as RColor;
    match c {
        RColor::Reset => Color::Reset,
        RColor::Black => Color::Rgb(0, 0, 0),
        RColor::Red => Color::Rgb(255, 0, 85),
        RColor::Green => Color::Rgb(0, 255, 150),
        RColor::Yellow => Color::Rgb(255, 255, 0),
        RColor::Blue => Color::Rgb(0, 150, 255),
        RColor::Magenta => Color::Rgb(255, 0, 255),
        RColor::Cyan => Color::Rgb(0, 255, 200),
        RColor::Gray => Color::Rgb(180, 180, 180),
        RColor::DarkGray => Color::Rgb(60, 60, 70),
        RColor::LightRed => Color::Rgb(255, 100, 100),
        RColor::LightGreen => Color::Rgb(100, 255, 100),
        RColor::LightYellow => Color::Rgb(255, 255, 150),
        RColor::LightBlue => Color::Rgb(150, 150, 255),
        RColor::LightMagenta => Color::Rgb(255, 150, 255),
        RColor::LightCyan => Color::Rgb(150, 255, 255),
        RColor::White => Color::Rgb(255, 255, 255),
        RColor::Indexed(i) => Color::Ansi(i),
        RColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
