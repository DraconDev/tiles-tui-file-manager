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
fn apps() -> [Constraint; 5] {
    [
        Constraint::Min(35),
        Constraint::Length(10),
        Constraint::Length(15),
        Constraint::Length(10),
        Constraint::Length(15),
    ]
}

fn probe(area: Rect, cons: &[Constraint], spacing: u16, flex: Option<Flex>) -> Vec<Option<u16>> {
    let markers: Vec<String> = (0..cons.len()).map(|i| ((b'a' + i as u8) as char).to_string()).collect();
    let header = Row::new(markers.iter().cloned().map(Cell::from).collect::<Vec<_>>()).height(1).bottom_margin(1);
    let rows: Vec<Row> = Vec::new();
    let mut t = Table::new(rows, cons.to_vec()).header(header).column_spacing(spacing);
    if let Some(fx) = flex { t = t.flex(fx); }
    let backend = TestBackend::new(area.x + area.width + 4, area.y + area.height + 4);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| f.render_widget(t, area)).unwrap();
    let buf = term.backend().buffer();
    let row: String = (area.x..area.x + area.width).map(|x| buf[(x, area.y)].symbol().to_string()).collect();
    let mut cursor = 0usize;
    markers.iter().map(|m| {
        let rest = &row[cursor..];
        let p = rest.find(m.as_str()).map(|i| area.x + (cursor as u16) + (i as u16));
        if let Some(x) = p { cursor = (x - area.x) as usize + 1; }
        p
    }).collect()
}

fn solve_b(area: Rect, cons: &[Constraint], spacing: u16, flex: Flex) -> Vec<(u16, u16)> {
    Layout::default().direction(Direction::Horizontal).constraints(cons.to_vec()).flex(flex).spacing(spacing)
        .split(Rect::new(0, 0, area.width, 1)).iter().map(|r| (area.x + r.x, r.width)).collect()
}
fn solve_cur(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<(u16, u16)> {
    Layout::default().direction(Direction::Horizontal).constraints(cons.to_vec()).spacing(spacing)
        .split(area).iter().map(|r| (r.x, r.width)).collect()
}

fn run(name: &str, cons: &[Constraint], spacing: u16, painted_of: impl Fn() -> Vec<Option<u16>>) {
    let mut checks = 0u32;
    let mut cur_bad = 0u32;
    let mut bl_bad = 0u32;
    let mut bs_bad = 0u32;
    let mut examples = Vec::new();
    for w in 20u16..=260 {
        for (ox, oy) in [(0u16, 0u16), (2, 5), (3, 1)] {
            for h in [1u16, 2, 10, 17, 24, 40] {
                let area = Rect::new(ox, oy, w, h);
                let painted = painted_of();
                let cur = solve_cur(area, cons, spacing);
                let bl = solve_b(area, cons, spacing, Flex::Legacy);
                let bs = solve_b(area, cons, spacing, Flex::Start);
                let _ = h;
                for i in 0..cons.len() {
                    // skip zero-width columns: nothing is painted there
                    if bs[i].1 == 0 { continue; }
                    checks += 1;
                    if painted[i] != Some(cur[i].0) { cur_bad += 1; }
                    if painted[i] != Some(bl[i].0) { bl_bad += 1; }
                    if painted[i] != Some(bs[i].0) {
                        bs_bad += 1;
                        if examples.len() < 5 {
                            examples.push(format!("w={w} o=({ox},{oy}) col{i} painted={:?} bStart={:?}", painted[i], bs[i]));
                        }
                    }
                }
            }
        }
    }
    println!("{name}: spacing={spacing} checks={checks} cur_bad={cur_bad} bLegacy_bad={bl_bad} bStart_bad={bs_bad} {examples:?}");
    let _ = painted_of;
}

fn main() {
    for cons in [procs().to_vec(), apps().to_vec()] {
        let spacing = 1u16;
        let c = cons.clone();
        run("tbl", &cons, spacing, || Vec::new());
        let _ = c;
    }
    // real sweep for each
    for (name, cons, spacing) in [("PROC", procs().to_vec(), 1u16), ("APPS", apps().to_vec(), 2u16)] {
        let mut checks = 0u32;
        let (mut cur_bad, mut bl_bad, mut bs_bad) = (0u32, 0u32, 0u32);
        let mut worst = (0u16, 0u16);
        for w in 20u16..=260 {
            for (ox, oy) in [(0u16, 0u16), (2, 5), (3, 1)] {
                for h in [1u16, 2, 10, 17, 24, 40] {
                    let area = Rect::new(ox, oy, w, h);
                    let painted = probe(area, &cons, spacing, None);
                    let cur = solve_cur(area, &cons, spacing);
                    let bl = solve_b(area, &cons, spacing, Flex::Legacy);
                    let bs = solve_b(area, &cons, spacing, Flex::Start);
                    for i in 0..cons.len() {
                        if bs[i].1 == 0 { continue; }
                        checks += 1;
                        if painted[i] != Some(cur[i].0) { cur_bad += 1; }
                        if painted[i] != Some(bl[i].0) { bl_bad += 1; }
                        if painted[i] != Some(bs[i].0) { bs_bad += 1; if w > worst.0 { worst = (w, i as u16); } }
                    }
                }
            }
        }
        println!("{name}: spacing={spacing} checks={checks} cur_bad={cur_bad} bLegacy_bad={bl_bad} bStart_bad={bs_bad} worst={worst:?}");
    }
}
