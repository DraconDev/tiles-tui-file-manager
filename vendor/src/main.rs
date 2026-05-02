// TRANSFORMED FROM examples/cyberpunk_dashboard.rs
// to src/main.rs for "Default Run" capability

use std::io::{self, stdout};
use std::time::{Duration, Instant};

// Since this is inside the crate, we can use `crate::` or `dracon_terminal_engine::` if linked.
// Using `crate::` allows us to strict bind to the library sources.
use dracon_terminal_engine::{compositor::plane::Plane, integration::ratatui::RatatuiBackend};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph, Sparkline},
    Terminal,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print Launch Banner
    println!("Starting Engine Cyberpunk Dashboard...");
    std::thread::sleep(Duration::from_millis(500));

    // Initialize
    let mut terminal = Terminal::new(RatatuiBackend::new(stdout())?)?;
    let _stdin = io::stdin();

    let mut data1 = vec![0.0; 40];
    let mut tick = 0;
    let mut alert_visible;

    loop {
        let _start = Instant::now();
        tick += 1;
        data1.remove(0);
        data1.push((tick as f64).sin().abs() * 100.0);

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.area());

            let header = Paragraph::new(Span::styled(
                " DRACON CYBERPUNK SHOWCASE ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ))
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
            f.render_widget(header, chunks[0]);

            let grid = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let cpu_data: Vec<(f64, f64)> = data1
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as f64, v))
                .collect();
            let datasets = vec![Dataset::default()
                .name("CPU LOAD")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Magenta))
                .data(&cpu_data)];
            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .title("CORE 01")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                )
                .x_axis(Axis::default().bounds([0.0, 40.0]))
                .y_axis(Axis::default().bounds([0.0, 100.0]));
            f.render_widget(chart, grid[0]);

            let right_chunks = Layout::default()
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(grid[1]);

            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .title("MEMORY")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .gauge_style(Style::default().fg(Color::Green))
                .percent((tick % 100) as u16);
            f.render_widget(gauge, right_chunks[0]);

            let spark = Sparkline::default()
                .block(
                    Block::default()
                        .title("NET I/O")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().fg(Color::Yellow))
                .data(data1.iter().map(|v| *v as u64).collect::<Vec<u64>>());
            f.render_widget(spark, right_chunks[1]);
        })?;

        let backend = terminal.backend_mut();
        let width = 30;
        let height = 7;

        backend.compositor_mut().planes.retain(|p| p.id == 0);

        alert_visible = tick > 50 && tick < 150;

        if alert_visible {
            let mut alert = Plane::new(99, width, height);
            alert.set_z_index(50);
            alert.set_absolute_position(25, 8);

            let msg = "SYSTEM BREACH DETECTED";
            let sub = "[SPACE] TO DISMISS";

            for y in 0..height {
                for x in 0..width {
                    let c = if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                        '#'
                    } else {
                        ' '
                    };
                    alert.put_char(x, y, c);
                }
            }

            for (i, c) in msg.chars().enumerate() {
                alert.put_char(2 + i as u16, 2, c);
            }
            for (i, c) in sub.chars().enumerate() {
                alert.put_char(2 + i as u16, 4, c);
            }

            backend.compositor_mut().add_plane(alert);
            terminal.flush()?;
        }

        if tick > 200 {
            break;
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
