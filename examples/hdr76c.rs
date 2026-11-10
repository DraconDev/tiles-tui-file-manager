use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tiles_tui_file_manager::app::{App, MonitorSubview};
use tiles_tui_file_manager::ui::monitor::draw_processes_view;

fn main() {
    let (w, h) = (76u16, 20u16);
    let area = Rect::new(0, 0, w, h);
    let mut app = App::new();
    app.monitor.monitor_subview = MonitorSubview::Processes;
    app.system_state.processes = Vec::new();
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| draw_processes_view(f, area, &mut app)).unwrap();
    let buf = term.backend().buffer();
    let row: String = (0..w).map(|x| buf[(x, 0)].symbol().to_string()).collect();
    for (i, c) in row.chars().enumerate() {
        if i >= 55 { println!("{i}: U+{:04X} {:?}", c as u32, c); }
    }
    println!("find MEM% = {:?}", row.find("MEM%"));
    println!("find MEM = {:?}", row.find("MEM"));
    println!("byte len={} chars={}", row.len(), row.chars().count());
}
