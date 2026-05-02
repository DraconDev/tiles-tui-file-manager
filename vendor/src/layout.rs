use crate::compositor::engine::Compositor;
use crate::input::event::Event;
use crate::widgets::component::{Bounds, Component};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Helper to create a centered rectangle for modals/popups.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Direction for Stack layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A flexbox-like container that arranges child components in a row or column.
///
/// Children are positioned sequentially, taking their preferred size or
/// splitting the available space evenly if no preference is given.
pub struct Stack<'a> {
    pub orientation: Orientation,
    pub children: Vec<&'a mut dyn Component>,
    pub spacing: u16,
}

impl<'a> Stack<'a> {
    pub fn new(orientation: Orientation) -> Self {
        Self {
            orientation,
            children: Vec::new(),
            spacing: 1,
        }
    }

    pub fn with_spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn add_child(&mut self, child: &'a mut dyn Component) {
        self.children.push(child);
    }

    /// Renders all children within the given bounds.
    pub fn render(&self, compositor: &mut Compositor, bounds: Bounds) {
        if self.children.is_empty() {
            return;
        }

        let count = self.children.len() as u16;
        let total_spacing = self.spacing * (count.saturating_sub(1));

        match self.orientation {
            Orientation::Horizontal => {
                let available_w = bounds.w.saturating_sub(total_spacing);
                let child_w = available_w / count;
                let mut x = bounds.x;

                for child in &self.children {
                    let child_bounds = Bounds::new(x, bounds.y, child_w, bounds.h);
                    child.render(compositor, child_bounds);
                    x += child_w + self.spacing;
                }
            }
            Orientation::Vertical => {
                let available_h = bounds.h.saturating_sub(total_spacing);
                let child_h = available_h / count;
                let mut y = bounds.y;

                for child in &self.children {
                    let child_bounds = Bounds::new(bounds.x, y, bounds.w, child_h);
                    child.render(compositor, child_bounds);
                    y += child_h + self.spacing;
                }
            }
        }
    }

    /// Dispatches events to children (first child that consumes wins).
    pub fn on_event(&mut self, event: &Event, bounds: Bounds) -> bool {
        // We need to recalculate bounds for each child during dispatch
        let count = self.children.len() as u16;
        if count == 0 {
            return false;
        }

        let total_spacing = self.spacing * (count.saturating_sub(1));

        match self.orientation {
            Orientation::Horizontal => {
                let available_w = bounds.w.saturating_sub(total_spacing);
                let child_w = available_w / count;
                let mut x = bounds.x;

                for child in &mut self.children {
                    let child_bounds = Bounds::new(x, bounds.y, child_w, bounds.h);
                    if child.on_event(event, child_bounds) {
                        return true;
                    }
                    x += child_w + self.spacing;
                }
            }
            Orientation::Vertical => {
                let available_h = bounds.h.saturating_sub(total_spacing);
                let child_h = available_h / count;
                let mut y = bounds.y;

                for child in &mut self.children {
                    let child_bounds = Bounds::new(bounds.x, y, bounds.w, child_h);
                    if child.on_event(event, child_bounds) {
                        return true;
                    }
                    y += child_h + self.spacing;
                }
            }
        }
        false
    }
}
