use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
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

/// Render a real Table with distinct single-char marker headers and report each
/// marker's painted x. This is ratatui's own column solve, not a re-derivation.
fn painted_marker_xs(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<Option<u16>> {
    let markers: Vec<String> = (0..cons.len())
        .map(|i| ((b'a' + i as u8) as char).to_string())
        .collect();
    let header = Row::new(markers.iter().cloned().map(Cell::from).collect::<Vec<_>>())
        .height(1)
        .bottom_margin(1);
    let rows: Vec<Row> = Vec::new();
    let backend = TestBackend::new(area.x + area.width + 4, area.y + area.height + 4);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| {
        f.render_widget(
            Table::new(rows, cons.to_vec())
                .header(header)
                .column_spacing(spacing),
            area,
        )
    })
    .unwrap();
    let buf: &Buffer = term.backend().buffer();
    let row: Vec<(u16, String)> = (area.x..area.x + area.width)
        .map(|x| (x, buf[(x, area.y)].symbol().to_string()))
        .collect();
    let mut cursor = area.x;
    markers
        .iter()
        .map(|m| {
            let found = row
                .iter()
                .find(|(x, s)| *x >= cursor && s.as_str() == m.as_str())
                .map(|(x, _)| *x);
            if let Some(x) = found {
                cursor = x + 1;
            }
            found
        })
        .collect()
}

fn solve_b(area: Rect, cons: &[Constraint], spacing: u16, flex: Flex) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(cons.to_vec())
        .flex(flex)
        .spacing(spacing)
        .split(Rect::new(0, 0, area.width, 1))
        .iter()
        .map(|r| Rect::new(area.x + r.x, area.y, r.width, 1))
        .collect()
}

fn solve_cur(area: Rect, cons: &[Constraint], spacing: u16) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(cons.to_vec())
        .spacing(spacing)
        .split(area)
        .iter()
        .map(|r| Rect::new(r.x, area.y, r.width, 1))
        .collect()
}

fn main() {
    let mut cur_bad = 0;
    let mut b_legacy_bad = 0;
    let mut b_start_bad = 0;
    let mut checks = 0;
    for w in 20u16..=260 {
        for (ox, oy) in [(0u16, 0u16), (2, 5), (3, 1)] {
            for h in [1u16, 2, 10, 17, 24, 40] {
                let area = Rect::new(ox, oy, w, h);
                let painted = painted_marker_xs(area, &procs(), 1);
                let cur = solve_cur(area, &procs(), 1);
                let bl = solve_b(area, &procs(), 1, Flex::Legacy);
                let bs = solve_b(area, &procs(), 1, Flex::Start);
                for i in 0..6 {
                    checks += 1;
                    if painted[i] != Some(cur[i].x) {
                        cur_bad += 1;
                    }
                    if painted[i] != Some(bl[i].x) {
                        b_legacy_bad += 1;
                    }
                    if painted[i] != Some(bs[i].x) {
                        b_start_bad += 1;
                    }
                }
                // last-column right edge sanity for Start vs Legacy at wide widths
                if w == 260 && h == 24 {
                    println!(
                        "area={:?} painted={:?}\n  cur={:?}\n  bLegacy={:?}\n  bStart={:?}",
                        area,
                        painted,
                        cur.iter().map(|r| (r.x, r.width)).collect::<Vec<_>>(),
                        bl.iter().map(|r| (r.x, r.width)).collect::<Vec<_>>(),
                        bs.iter().map(|r| (r.x, r.width)).collect::<Vec<_>>(),
                    );
                }
            }
        }
    }
    println!(
        "checks={checks} cur_bad={cur_bad} bLegacy_bad={b_legacy_bad} bStart_bad={b_start_bad}"
    );
}
