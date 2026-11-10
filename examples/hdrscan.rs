use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::widgets::{Cell, Row, Table};
use ratatui::Terminal;

fn procs() -> [Constraint; 6] {
    [
        Constraint::Length(8),
        Constraint::Min(25),
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
    ]
}

fn painted(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<Option<u16>> {
    let markers: Vec<String> = (0..cons.len()).map(|i| ((b'a' + i as u8) as char).to_string()).collect();
    let header = Row::new(markers.iter().cloned().map(Cell::from).collect::<Vec<_>>()).height(1).bottom_margin(1);
    let t = Table::new(Vec::<Row>::new(), cons.to_vec()).header(header).column_spacing(spacing);
    let mut term = Terminal::new(TestBackend::new(area.x + area.width + 2, area.y + area.height + 2)).unwrap();
    term.draw(|f| f.render_widget(t, area)).unwrap();
    let buf = term.backend().buffer();
    let mut cur = 0u16;
    markers.iter().map(|m| {
        let found = (cur..area.width).find(|i| buf[(area.x + *i, area.y)].symbol() == m.as_str());
        if let Some(i) = found { cur = i + 1; }
        found.map(|i| area.x + i)
    }).collect()
}

fn old_solve(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<(u16, u16)> {
    Layout::default().direction(Direction::Horizontal).constraints(cons.to_vec()).spacing(spacing)
        .split(area).iter().map(|r| (r.x, r.width)).collect()
}
fn new_solve(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<(u16, u16)> {
    Layout::default().direction(Direction::Horizontal).constraints(cons.to_vec()).flex(Flex::Start).spacing(spacing)
        .split(Rect::new(0, 0, area.width, 1)).iter().map(|r| (area.x + r.x, r.width)).collect()
}

fn main() {
    let names = ["PID", "NAME", "USER", "STATUS", "CPU%", "MEM%"];
    let mut old_bad = Vec::new();
    let mut new_bad = 0;
    let mut checks = 0;
    for w in 20u16..=260 {
        for (ox, oy) in [(0u16, 0u16), (2, 5), (3, 1)] {
            for h in [1u16, 2, 10, 17, 24, 40] {
                let area = Rect::new(ox, oy, w, h);
                let p = painted(area, &procs(), 1);
                let o = old_solve(area, &procs(), 1);
                let n = new_solve(area, &procs(), 1);
                for i in 0..6 {
                    if n[i].1 == 0 { continue; }
                    checks += 1;
                    if p[i] != Some(o[i].0) {
                        let d = p[i].map(|x| x as i32 - o[i].0 as i32).unwrap_or(0);
                        old_bad.push(format!("w={w} o=({ox},{oy}) h={h} {} band={} painted={:?} drift={}", names[i], o[i].0, p[i], d));
                    }
                    if p[i] != Some(n[i].0) { new_bad += 1; }
                }
            }
        }
    }
    println!("checks={checks} old_solve_mismatches={} new_solve_mismatches={new_bad}", old_bad.len());
    for line in &old_bad { println!("  OLD {line}"); }
}
