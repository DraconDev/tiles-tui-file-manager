use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tiles_tui_file_manager::app::{App, MonitorSubview};
use tiles_tui_file_manager::ui::monitor::draw_monitor_page;

fn main() {
    // The auditor's defect cases: terminal 86x18 and 84x24.
    for (tw, th) in [(86u16, 18u16), (84, 24), (120, 24), (200, 30)] {
        let mut app = App::new();
        app.monitor.monitor_subview = MonitorSubview::Processes;
        app.system_state.processes = Vec::new();
        let mut term = Terminal::new(TestBackend::new(tw, th)).unwrap();
        term.draw(|f| draw_monitor_page(f, Rect::new(0, 0, tw, th), &mut app))
            .unwrap();
        let buf = term.backend().buffer();
        let bands = app.monitor.process_column_bounds.clone();
        let y = bands[0].0.y;
        let row: String = (0..tw).map(|x| buf[(x, y)].symbol().to_string()).collect();
        // glyph-run starts on the header row = where captions are painted
        let runs: Vec<u16> = (0..tw)
            .filter(|x| {
                let cur = !buf[(*x, y)].symbol().trim().is_empty();
                let prev = *x == 0 || buf[(*x - 1, y)].symbol().trim().is_empty();
                cur && prev
            })
            .collect();
        println!("terminal {tw}x{th}  header row y={y}\n  |{row}|");
        println!("  band   col   band_x..end  caption_at_band_x");
        for (i, (r, c)) in bands.iter().enumerate() {
            println!(
                "   {i}   {c:?}   {}..{}   {:?}",
                r.x,
                r.x + r.width,
                buf[(r.x, y)].symbol()
            );
        }
        println!("  painted captions start at x = {runs:?}");
        // clicks at each painted caption start
        for (i, x) in runs.iter().enumerate() {
            let hit = bands
                .iter()
                .find(|(r, _)| r.contains(ratatui::layout::Position { x: *x, y }))
                .map(|(_, c)| *c);
            let _ = i;
            print!("  click x={x} -> {hit:?} |");
        }
        println!("\n");
    }
}
