use crate::compositor::plane::Cell;

/// A trait for visual effects that can be applied to a Plane's cells.
pub trait Filter {
    fn apply(&self, cell: &mut Cell, x: u16, y: u16, time: f32);
}

/// A filter that dims the foreground and background colors.
/// Useful for "Modal Backgrounds".
pub struct Dim {
    pub factor: f32, // 0.0 to 1.0
}

impl Default for Dim {
    fn default() -> Self {
        Self { factor: 0.5 }
    }
}

impl Filter for Dim {
    fn apply(&self, cell: &mut Cell, _x: u16, _y: u16, _time: f32) {
        let dim = |color: crate::compositor::plane::Color| match color {
            crate::compositor::plane::Color::Rgb(r, g, b) => crate::compositor::plane::Color::Rgb(
                (r as f32 * self.factor) as u8,
                (g as f32 * self.factor) as u8,
                (b as f32 * self.factor) as u8,
            ),
            crate::compositor::plane::Color::Ansi(c) => {
                if c > 8 {
                    crate::compositor::plane::Color::Ansi(8)
                } else {
                    color
                }
            }
            crate::compositor::plane::Color::Reset => color,
        };

        cell.fg = dim(cell.fg);
        cell.bg = dim(cell.bg);
    }
}

/// Invert colors filter
pub struct Invert;

impl Filter for Invert {
    fn apply(&self, cell: &mut Cell, _x: u16, _y: u16, _time: f32) {
        std::mem::swap(&mut cell.fg, &mut cell.bg);
    }
}

/// A filter that adds a scanline effect by dimming alternate rows.
pub struct Scanline;

impl Filter for Scanline {
    fn apply(&self, cell: &mut Cell, _x: u16, y: u16, _time: f32) {
        if y.is_multiple_of(2) {
            let dim = |color: crate::compositor::plane::Color| match color {
                crate::compositor::plane::Color::Rgb(r, g, b) => {
                    crate::compositor::plane::Color::Rgb(
                        (r as f32 * 0.8) as u8,
                        (g as f32 * 0.8) as u8,
                        (b as f32 * 0.8) as u16 as u8,
                    )
                }
                _ => color,
            };
            cell.fg = dim(cell.fg);
            cell.bg = dim(cell.bg);
        }
    }
}

/// A filter that pulses the brightness of the cell over time.
pub struct Pulse;

impl Filter for Pulse {
    #[allow(clippy::redundant_closure_call)]
    fn apply(&self, cell: &mut Cell, _x: u16, _y: u16, time: f32) {
        let factor = (time.sin() * 0.2 + 0.8).clamp(0.0, 1.0);
        let dim = |color: crate::compositor::plane::Color| match color {
            crate::compositor::plane::Color::Rgb(r, g, b) => crate::compositor::plane::Color::Rgb(
                (r as f32 * factor) as u8,
                (g as f32 * factor) as u8,
                (b as f32 * factor) as u8,
            ),
            _ => color,
        };
        cell.fg = dim(cell.fg);
    }
}

/// A filter that randomly glitches cells based on time and position.
pub struct Glitch;

impl Filter for Glitch {
    fn apply(&self, cell: &mut Cell, x: u16, y: u16, time: f32) {
        let seed = (x as f32 * 12.9898 + y as f32 * 78.233 + time).sin() * 43_758.547;
        let rand = seed - seed.floor();

        if rand > 0.98 {
            cell.char = if rand > 0.99 { '█' } else { '░' };
            cell.fg = crate::compositor::plane::Color::Rgb(255, 0, 85); // Neon Pink
        } else if rand > 0.95 {
            // Horizontal shift look
            let shift = (time * 10.0).sin() * 5.0;
            if (y as f32 - shift.abs()).abs() < 1.0 {
                cell.style.insert(crate::compositor::plane::Styles::REVERSE);
            }
        }
    }
}
