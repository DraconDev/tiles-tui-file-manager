use crate::compositor::engine::{map_color, Compositor, TilePlacement};
use crate::compositor::plane::{Cell, Plane};
use crate::core::terminal::Terminal;
use crate::visuals::assets::Icon;
use ratatui::backend::Backend;
use ratatui::layout::{Position, Size};
use std::io::{self, Write};
use std::os::fd::AsFd;
use std::sync::{Arc, Mutex};
use unicode_width::UnicodeWidthStr;

pub struct RatatuiCompositorBackend<'a> {
    pub compositor: &'a mut Compositor,
}

impl<'a> Backend for RatatuiCompositorBackend<'a> {
    fn draw<'b, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'b ratatui::buffer::Cell)>,
    {
        if let Some(plane) = self.compositor.planes.first_mut() {
            for (x, y, cell) in content {
                let fg = map_color(cell.fg);
                let bg = map_color(cell.bg);
                let mut style = crate::compositor::plane::Styles::empty();
                if cell.modifier.contains(ratatui::style::Modifier::BOLD) {
                    style.insert(crate::compositor::plane::Styles::BOLD);
                }

                let sym = cell.symbol();
                let width = sym.width();

                // If width is 0 and it's not an empty string, it's a continuation cell
                if width == 0 && !sym.is_empty() {
                    plane.set_skip(x, y, true);
                    continue;
                }

                // Only overwrite if there is something to show or background is set
                if width > 0 || !sym.is_empty() || bg != crate::compositor::plane::Color::Reset {
                    plane.set_style(x, y, fg, bg, style);
                    let c = sym.chars().next().unwrap_or(' ');
                    plane.put_char(x, y, c);
                    if width > 1 {
                        for i in 1..width {
                            plane.set_skip(x + i as u16, y, true);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn show_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(Position { x: 0, y: 0 })
    }
    fn set_cursor_position<P: Into<Position>>(&mut self, _pos: P) -> io::Result<()> {
        Ok(())
    }
    fn clear(&mut self) -> io::Result<()> {
        if let Some(plane) = self.compositor.planes.first_mut() {
            for cell in &mut plane.cells {
                cell.char = ' ';
                cell.bg = crate::compositor::plane::Color::Reset;
            }
        }
        Ok(())
    }
    fn size(&self) -> io::Result<Size> {
        let (w, h) = self.compositor.size();
        Ok(Size {
            width: w,
            height: h,
        })
    }
    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        let (w, h) = self.compositor.size();
        Ok(ratatui::backend::WindowSize {
            columns_rows: ratatui::layout::Size {
                width: w,
                height: h,
            },
            pixels: ratatui::layout::Size {
                width: 0,
                height: 0,
            },
        })
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct RatatuiBackend<W: io::Write + std::os::fd::AsFd> {
    inner: Terminal<W>,
    compositor: Compositor,
    pub tile_queue: Arc<Mutex<Vec<TilePlacement>>>,
    last_size_check: std::time::Instant,
}

impl<W: io::Write + std::os::fd::AsFd> RatatuiBackend<W> {
    pub fn new(writer: W) -> io::Result<Self> {
        let size = crate::backend::tty::get_window_size(writer.as_fd()).unwrap_or((80, 24));
        let mut compositor = Compositor::new(size.0, size.1);

        // Pre-register icons
        let icons = vec![
            Icon::Folder,
            Icon::File,
            Icon::Rust,
            Icon::Json,
            Icon::Settings,
            Icon::Dracon,
            Icon::ButtonLeft,
            Icon::ButtonMid,
            Icon::ButtonRight,
        ];
        for icon in icons {
            let asset = icon.get_asset();
            compositor.add_tile_asset(icon as u32, asset.cells, asset.width, asset.height);
        }

        let base_plane = Plane::new(0, size.0, size.1);
        compositor.add_plane(base_plane);

        Ok(Self {
            inner: Terminal::new(writer)?,
            compositor,
            tile_queue: Arc::new(Mutex::new(Vec::new())),
            last_size_check: std::time::Instant::now(),
        })
    }

    pub fn compositor_mut(&mut self) -> &mut Compositor {
        &mut self.compositor
    }

    pub fn tile_queue(&self) -> Arc<Mutex<Vec<TilePlacement>>> {
        self.tile_queue.clone()
    }

    pub fn add_tile_asset(&mut self, id: u32, cells: Vec<Cell>, width: u16, height: u16) {
        self.compositor.add_tile_asset(id, cells, width, height);
    }

    pub fn add_image_asset(&mut self, id: u32, data: Vec<u8>, width: u32, height: u32) {
        self.compositor.add_image_asset(id, data, width, height);
    }

    pub fn add_image_tile(
        &mut self,

        asset_id: u32,

        x: u16,

        y: u16,

        z: i32,

        cols: Option<u16>,

        rows: Option<u16>,
    ) {
        let id = 5000 + (asset_id * 100) + (y as u32 * 10) + x as u32;

        let placement = TilePlacement {
            asset_id,

            is_image: true,

            x,

            y,

            z_index: z,

            cols,

            rows,

            placement_id: Some(id),

            ..Default::default()
        };

        if let Ok(mut q) = self.tile_queue.lock() {
            q.push(placement);
        }
    }

    pub fn add_icon(&mut self, icon: Icon, x: u16, y: u16, z: i32) {
        let id = 1000 + (icon as u32 * 100) + (y as u32 * 10) + x as u32;

        let placement = TilePlacement {
            asset_id: icon as u32,

            is_image: false,

            x,

            y,

            z_index: z,

            cols: None,

            rows: None,

            placement_id: Some(id),

            ..Default::default()
        };

        if let Ok(mut q) = self.tile_queue.lock() {
            q.push(placement);
        }
    }
}

impl<W: io::Write + std::os::fd::AsFd> Backend for RatatuiBackend<W> {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        // Throttle resizing handling to ~10hz
        if self.last_size_check.elapsed() > std::time::Duration::from_millis(100) {
            self.last_size_check = std::time::Instant::now();
            if let Ok((w, h)) = crate::backend::tty::get_window_size(self.inner.as_fd()) {
                let (cw, ch) = self.compositor.size();
                if w != cw || h != ch {
                    self.compositor.resize(w, h);
                    if let Some(plane) = self.compositor.planes.first_mut() {
                        *plane = Plane::new(0, w, h);
                    }
                }
            }
        }

        if let Some(plane) = self.compositor.planes.first_mut() {
            for (x, y, cell) in content {
                let fg = map_color(cell.fg);
                let bg = map_color(cell.bg);
                let mut style = crate::compositor::plane::Styles::empty();
                if cell.modifier.contains(ratatui::style::Modifier::BOLD) {
                    style.insert(crate::compositor::plane::Styles::BOLD);
                }

                let sym = cell.symbol();
                let width = sym.width();

                // If width is 0 and it's not an empty string, it's a continuation cell
                if width == 0 && !sym.is_empty() {
                    plane.set_skip(x, y, true);
                    continue;
                }

                // Only overwrite if there is something to show or background is set
                if width > 0 || !sym.is_empty() || bg != crate::compositor::plane::Color::Reset {
                    plane.set_style(x, y, fg, bg, style);
                    let c = sym.chars().next().unwrap_or(' ');
                    plane.put_char(x, y, c);
                    if width > 1 {
                        for i in 1..width {
                            plane.set_skip(x + i as u16, y, true);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        write!(self.inner, "\x1b[?25l")
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        write!(self.inner, "\x1b[?25h")
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(Position { x: 0, y: 0 })
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, pos: P) -> io::Result<()> {
        let pos = pos.into();

        write!(self.inner, "\x1b[{};{}H", pos.y + 1, pos.x + 1)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.compositor.force_clear();
        // Set background to pure black before clearing
        write!(self.inner, "\x1b[48;2;0;0;0m\x1b[2J")
    }

    fn size(&self) -> io::Result<Size> {
        let (w, h) = crate::backend::tty::get_window_size(self.inner.as_fd())?;
        Ok(Size {
            width: w,
            height: h,
        })
    }
    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        let (w, h) = crate::backend::tty::get_window_size(self.inner.as_fd())?;
        Ok(ratatui::backend::WindowSize {
            columns_rows: ratatui::layout::Size {
                width: w,
                height: h,
            },
            pixels: ratatui::layout::Size {
                width: 0,
                height: 0,
            },
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut queue) = self.tile_queue.lock() {
            self.compositor.clear_tile_placements();
            for p in queue.drain(..) {
                self.compositor.add_tile_placement(p);
            }
        }
        self.compositor.render(self.inner.inner())?;
        self.inner.flush()
    }
}
