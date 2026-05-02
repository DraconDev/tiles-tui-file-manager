use dracon_terminal_engine::{
    compositor::plane::Plane,
    // Terminal, // Unused in this demo structure as we use RatatuiBackend
    integration::ratatui::RatatuiBackend,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Terminal,
};
use std::io::stdout;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Backend (God Mode)
    let stdout = stdout();
    let backend = RatatuiBackend::new(stdout)?;
    let mut terminal = Terminal::new(backend)?;

    // 2. Render Loop (Single Frame for Demo)
    terminal.draw(|f| {
        let size = f.area(); // Use area() instead of size()
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(size);

        let block = Block::default()
            .title("Ratatui Layer (Z=0)")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        f.render_widget(block, chunks[0]);
    })?;

    // 3. GOD MODE VISUALS (Z-Index Overlay)
    let backend = terminal.backend_mut();

    // Add a God Mode Plane (Z=10)
    let mut float_plane = Plane::new(1, 20, 5);
    float_plane.set_absolute_position(10, 5);
    float_plane.set_z_index(10);
    // Draw some manual text on it
    let msg = "God Mode (Z=10)";
    for (i, c) in msg.chars().enumerate() {
        float_plane.put_char(i as u16, 1, c);
    }

    backend.compositor_mut().add_plane(float_plane);

    // Force another draw to show the new plane
    terminal.draw(|f| {
        let block = Block::default()
            .title("Ratatui Layer (Z=0)")
            .borders(Borders::ALL);
        f.render_widget(block, f.area());
    })?;

    Ok(())
}
