use ratatui::layout::Rect;
fn main() {
    for (w, h) in [(80u16, 24u16), (120, 40), (80, 10), (200, 60)] {
        let r = Rect::new(0, 0, w, h);
        let cr = dracon_terminal_engine::layout::centered_rect(64, 9, r);
        println!("{w}x{h}: centered_rect(64,9) -> {cr:?}  (height {} rows)", cr.height);
    }
    println!("---");
    for (w, h) in [(80u16, 24u16), (120, 40)] {
        let r = Rect::new(0, 0, w, h);
        for (px, py, name) in [(64u16, 9u16, "style color"), (60, 20, "import servers"), (70, 20, "reset?"), (60, 40, "bulk rename")] {
            let cr = dracon_terminal_engine::layout::centered_rect(px, py, r);
            println!("{w}x{h} {name} ({px}%,{py}%) -> y={} h={}", cr.y, cr.height);
        }
    }
}
