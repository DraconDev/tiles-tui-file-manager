use ratatui::layout::Rect;
fn main() {
    // content rows each modal needs (borders consume 2)
    let cases = [
        ("style color", 64u16, 9u16, 9usize),
        ("reset settings", 56, 12, 10),
        ("import servers", 60, 20, 10),
        ("bulk rename", 60, 20, 13),
        ("properties", 50, 50, 20),
    ];
    for (w, h) in [(80u16, 24u16), (120, 40), (80, 10), (100, 30)] {
        println!("--- {w}x{h} ---");
        for (name, px, py, need) in cases {
            let r = dracon_terminal_engine::layout::centered_rect(px, py, Rect::new(0, 0, w, h));
            let inner = r.height.saturating_sub(2) as usize;
            let flag = if inner < need { "  <-- CONTENT CLIPPED" } else { "" };
            println!("  {name:<16} rect h={:>3} inner rows={:>3} need>={}{flag}", r.height, inner, need);
        }
    }
}
