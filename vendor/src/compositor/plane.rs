use bitflags::bitflags;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Reset,
    Ansi(u8),
    Rgb(u8, u8, u8),
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct Styles: u8 {
        const BOLD          = 1 << 0;
        const DIM           = 1 << 1;
        const ITALIC        = 1 << 2;
        const UNDERLINE     = 1 << 3;
        const BLINK         = 1 << 4;
        const REVERSE       = 1 << 5;
        const HIDDEN        = 1 << 6;
        const STRIKETHROUGH = 1 << 7;
    }
}

/// A single cell in the terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub char: char,
    pub fg: Color,
    pub bg: Color,
    pub style: Styles,
    pub transparent: bool,
    pub skip: bool, // New: if true, this cell is ignored by the renderer (e.g. for wide chars)
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        }
    }
}

use crate::compositor::filter::Filter;

/// A layer in the "God Mode" Compositor.
/// Planes are Z-indexed and can be moved efficiently.
pub struct Plane {
    pub id: usize,
    pub z_index: i32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
    pub visible: bool,
    pub opacity: f32, // 0.0 to 1.0
    pub filter: Option<Box<dyn Filter>>,
}

impl Plane {
    pub fn new(id: usize, width: u16, height: u16) -> Self {
        Self {
            id,
            z_index: 0,
            x: 0,
            y: 0,
            width,
            height,
            cells: vec![Cell::default(); (width * height) as usize],
            visible: true,
            opacity: 1.0,
            filter: None,
        }
    }

    pub fn set_absolute_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }

    pub fn set_z_index(&mut self, z: i32) {
        self.z_index = z;
    }

    /// Safe write char to local plane coordinates
    pub fn put_char(&mut self, x: u16, y: u16, c: char) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) as usize;
        self.cells[idx].char = c;
        self.cells[idx].transparent = false;
        self.cells[idx].skip = false;
    }

    pub fn put_cell(&mut self, x: u16, y: u16, mut cell: Cell) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) as usize;
        cell.transparent = false;
        self.cells[idx] = cell;
    }

    // Helper to set style
    pub fn set_style(&mut self, x: u16, y: u16, fg: Color, bg: Color, style: Styles) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) as usize;
        self.cells[idx].fg = fg;
        self.cells[idx].bg = bg;
        self.cells[idx].style = style;
        self.cells[idx].transparent = false;
        self.cells[idx].skip = false;
    }

    pub fn set_skip(&mut self, x: u16, y: u16, skip: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) as usize;
        self.cells[idx].skip = skip;
        if skip {
            self.cells[idx].transparent = false;
        }
    }

    /// Writes a string handling Unicode width.
    /// Returns the new X position.
    pub fn put_str(&mut self, mut x: u16, y: u16, text: &str) -> u16 {
        use unicode_width::UnicodeWidthChar;

        for c in text.chars() {
            if x >= self.width {
                break;
            }

            let width = c.width().unwrap_or(0);
            if width == 0 {
                continue;
            } // Skip zero-width chars

            // If it's a wide char (width 2), we need to ensure space
            if width == 2 && x + 1 >= self.width {
                break;
            }

            self.put_char(x, y, c);

            if width == 2 {
                let idx = (y * self.width + x + 1) as usize;
                if idx < self.cells.len() {
                    self.cells[idx].char = ' ';
                    self.cells[idx].transparent = false;
                    self.cells[idx].skip = true;
                }
            }

            x += width as u16;
        }
        x
    }

    /// Set a filter for this plane (e.g. Dim, Invert)
    pub fn set_filter(&mut self, filter: Box<dyn Filter>) {
        self.filter = Some(filter);
    }

    /// Set transparency for all cells in the plane.
    /// Note: This overwrites per-cell transparency.
    pub fn set_transparent(&mut self, transparent: bool) {
        for cell in &mut self.cells {
            cell.transparent = transparent;
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }
}
