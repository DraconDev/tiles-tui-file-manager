use ratatui::layout::{Constraint, Direction, Layout, Rect};
fn constraints() -> Vec<Constraint> {
    vec![
        Constraint::Length(8),
        Constraint::Min(25),
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
    ]
}
fn header(area: Rect, spacing: u16) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints())
        .spacing(spacing)
        .split(Rect::new(area.x, area.y, area.width, 1))
        .to_vec()
}
fn main() {
    for width in [60u16, 80] {
        let area = Rect::new(0, 0, width, 10);
        let h = header(area, 1);
        let t = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints())
            .spacing(1)
            .split(area);
        println!("width {width}");
        println!("  header(area,1): {:?}", h.iter().map(|r| (r.x, r.width, r.height)).collect::<Vec<_>>());
        println!("  table(area):    {:?}", t.iter().map(|r| (r.x, r.width, r.height)).collect::<Vec<_>>());
    }
}
