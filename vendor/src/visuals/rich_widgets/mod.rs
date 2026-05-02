use crate::compositor::engine::Compositor;
use crate::compositor::plane::{Color, Styles};
use crate::input::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use crate::visuals::slicer::NineSlice;
use crate::widgets::component::{Bounds, Component};

/// A high-level container widget with graphical borders and a title.
///
/// `RichPanel` uses a `NineSlice` for its background, allowing it to scale
/// while maintaining high-fidelity corners.
pub struct RichPanel {
    pub title: String,
    pub background: Option<NineSlice>,
    pub border_color: Color,
}

impl RichPanel {
    /// Creates a new panel with the given title.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            background: None,
            border_color: Color::Ansi(4), // Default Blue
        }
    }

    /// Sets the 9-slice graphical background for the panel.
    pub fn with_background(mut self, slice: NineSlice) -> Self {
        self.background = Some(slice);
        self
    }

    /// Sets the theme color for the panel's borders and title.
    pub fn with_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }
}

impl Component for RichPanel {
    fn render(&self, compositor: &mut Compositor, bounds: Bounds) {
        // 1. Render Graphical Background/Border if present
        if let Some(ref slice) = self.background {
            slice.render(
                compositor,
                bounds.x,
                bounds.y,
                bounds.w,
                bounds.h,
                0,
                compositor.graphics_enabled,
            );
        }

        // 2. Render Text Title (Top-Left)
        if !self.title.is_empty() {
            let title_text = format!(" {} ", self.title);
            compositor.draw_text(
                &title_text,
                bounds.x + 2,
                bounds.y,
                self.border_color,
                Color::Reset,
                Styles::BOLD,
            );
        }
    }
}

/// A stateful button widget using graphical 9-slice assets.
/// Supports hover and click states for interactivity.
pub struct RichButton {
    pub label: String,
    pub background: NineSlice,
    pub focused: bool,
    pub hovered: bool,
    pub on_click: Option<Box<dyn Fn() + Send + Sync>>,
}

impl RichButton {
    /// Creates a new button with the given label and 9-slice background.
    pub fn new(label: &str, background: NineSlice) -> Self {
        Self {
            label: label.to_string(),
            background,
            focused: false,
            hovered: false,
            on_click: None,
        }
    }

    /// Sets the focus state of the button (for keyboard navigation).
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Sets a callback to be invoked on click.
    pub fn on_click<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl Component for RichButton {
    fn render(&self, compositor: &mut Compositor, bounds: Bounds) {
        let z = if self.focused {
            3
        } else if self.hovered {
            2
        } else {
            1
        };
        self.background.render(
            compositor,
            bounds.x,
            bounds.y,
            bounds.w,
            bounds.h,
            z,
            compositor.graphics_enabled,
        );

        // Render label in center
        let lx = bounds.x + (bounds.w.saturating_sub(self.label.len() as u16)) / 2;
        let ly = bounds.y + bounds.h / 2;

        let fg = if self.hovered {
            Color::Rgb(255, 255, 0) // Yellow on hover
        } else {
            Color::Rgb(255, 255, 255) // White default
        };

        compositor.draw_text(&self.label, lx, ly, fg, Color::Reset, Styles::BOLD);
    }

    fn on_event(&mut self, event: &Event, bounds: Bounds) -> bool {
        match event {
            Event::Mouse(MouseEvent {
                kind, column, row, ..
            }) => {
                let inside = bounds.contains(*column, *row);

                match kind {
                    MouseEventKind::Moved => {
                        self.hovered = inside;
                        inside // Consume if inside
                    }
                    MouseEventKind::Down(MouseButton::Left) if inside => {
                        if let Some(ref cb) = self.on_click {
                            cb();
                        }
                        true // Consumed
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
