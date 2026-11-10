use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tiles_tui_file_manager::app::{App, MonitorSubview};
use tiles_tui_file_manager::ui::monitor::draw_processes_view;

fn main() {
    for extra in [0u16, 2] {
        let (w, h) = (76u16, 20u16);
        let area = Rect::new(0, 0, w, h);
        let mut app = App::new();
        app.monitor.monitor_subview = MonitorSubview::Processes;
        app.system_state.processes = Vec::new();
        let backend = TestBackend::new(w + extra, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| draw_processes_view(f, area, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row: String = (0..w).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        println!("extra={extra} row.len={} row={row:?}", row.chars().count());
        for cap in ["PID", "NAME", "USER", "STATUS", "CPU%", "MEM%"] {
            println!("  {cap} find={:?}", row.find(cap));
        }
        let cells: Vec<(u16, String, usize)> = (52..76)
            .map(|x| (x, buf[(x, 0)].symbol().to_string(), buf[(x, 0)].symbol().chars().count()))
            .collect();
        println!("  cells={cells:?}");
        println!("  sort={:?} asc={}", app.monitor.process_sort_col, app.monitor.process_sort_asc);
    }
}
