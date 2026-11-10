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

fn probe(area: Rect, cons: &[Constraint], spacing: u16, flex: Option<Flex>) -> (Vec<Option<u16>>, String) {
    let markers: Vec<String> = (0..cons.len())
        .map(|i| ((b'a' + i as u8) as char).to_string())
        .collect();
    let header = Row::new(markers.iter().cloned().map(Cell::from).collect::<Vec<_>>())
        .height(1)
        .bottom_margin(1);
    let rows: Vec<Row> = Vec::new();
    let mut t = Table::new(rows, cons.to_vec()).header(header).column_spacing(spacing);
    if let Some(fx) = flex {
        t = t.flex(fx);
    }
    let backend = TestBackend::new(area.x + area.width + 4, area.y + area.height + 4);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| f.render_widget(t, area)).unwrap();
    let buf = term.backend().buffer();
    let row: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, area.y)].symbol().to_string())
        .collect();
    let mut cursor = 0usize;
    let found = markers
        .iter()
        .map(|m| {
            let rest = &row[cursor..];
            let p = rest.find(m.as_str()).map(|i| area.x + (cursor as u16) + (i as u16));
            if let Some(x) = p {
                cursor = (x - area.x) as usize + 1;
            }
            p
        })
        .collect();
    (found, row)
}

fn solve_b(area: Rect, cons: &[Constraint], spacing: u16, flex: Flex) -> Vec<(u16, u16)> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(cons.to_vec())
        .flex(flex)
        .spacing(spacing)
        .split(Rect::new(0, 0, area.width, 1))
        .iter()
        .map(|r| (area.x + r.x, r.width))
        .collect()
}

fn main() {
    let mut shown = 0;
    let mut bad_by_width: std::collections::BTreeMap<u16, u32> = Default::default();
    for w in 20u16..=260 {
        for (ox, oy) in [(0u16, 0u16), (2, 5), (3, 1)] {
            for h in [1u16, 2, 10, 17, 24, 40] {
                let area = Rect::new(ox, oy, w, h);
                let (painted, row) = probe(area, &procs(), 1, None);
                let bs = solve_b(area, &procs(), 1, Flex::Start);
                let bad = (0..6).any(|i| painted[i] != Some(bs[i].0));
                if bad {
                    *bad_by_width.entry(w).or_default() += 1;
                    if shown < 8 {
                        shown += 1;
                        println!("w={w} h={h} o=({ox},{oy}) painted={painted:?} bStart={bs:?}\n  row={row:?}");
                    }
                }
            }
        }
    }
    println!("bad widths: {bad_by_width:?}");
}
