use ratatui::layout::{Constraint, Direction, Layout, Rect};
fn main() {
    let width = 120u16;
    let constraints = [
        Constraint::Length(8),
        Constraint::Min(25),
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
    ];
    for label in ["header(mirrors bugs)", "table spacing=1 (render)"] {
        let spacing: u16 = if label.contains("bug") { 2 } else { 1 };
        let cols = 6u16;
        let effective = width.saturating_sub((cols - 1) * spacing);
        let rects = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .spacing(spacing)
            .split(Rect::new(0, 0, effective, 1));
        let mut cur = 0u16;
        let mut by_acc = Vec::new();
        for r in rects.iter() {
            by_acc.push((cur, r.width));
            cur += r.width + spacing;
        }
        println!("{label:<26} spacing={spacing} rects={:?}", rects.iter().map(|r| (r.x, r.width)).collect::<Vec<_>>());
        println!("{:<26} accumulated={:?}", "", by_acc);
    }
}
