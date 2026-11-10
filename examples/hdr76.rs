use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tiles_tui_file_manager::app::{App, MonitorSubview};
use tiles_tui_file_manager::ui::monitor::draw_processes_view;

fn main() {
    for w in [74u16, 75, 76, 77, 78, 79, 80, 81, 82] {
        let area = Rect::new(0, 0, w, 20);
        let mut app = App::new();
        app.monitor.monitor_subview = MonitorSubview::Processes;
        app.system_state.processes = Vec::new();
        let backend = TestBackend::new(w, 20);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| draw_processes_view(f, area, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row: String = (0..w).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        let bands: Vec<_> = app.monitor.process_column_bounds.iter().map(|(r, c)| (format!("{c:?}"), r.x, r.width)).collect();
        println!("w={w} row={row:?}\n   bands={bands:?}");
    }
}
