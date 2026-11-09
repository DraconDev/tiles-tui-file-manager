//! Temporary live check for in-session log rotation (deleted after verification).
use std::io::Write;

fn zcount(p: &std::path::Path) -> usize {
    std::fs::read(p)
        .map(|b| b.iter().filter(|&&c| c == b'Z').count())
        .unwrap_or(0)
}

fn main() {
    let base = std::env::args().nth(1).expect("data home");
    let dir = std::path::PathBuf::from(&base).join("tiles");
    std::fs::create_dir_all(&dir).unwrap();
    let active = dir.join("debug.log");
    let mut f = std::fs::File::create(&active).unwrap();
    let chunk = vec![b'Z'; 64 * 1024];
    while f.metadata().unwrap().len() <= 5 * 1024 * 1024 {
        f.write_all(&chunk).unwrap();
    }
    drop(f);
    std::fs::write(dir.join("debug.log.1"), "OLDER-GEN\n").unwrap();
    println!("seeded active={} bytes", std::fs::metadata(&active).unwrap().len());

    tiles_tui_file_manager::app::log_debug("live-verify line one");
    tiles_tui_file_manager::app::log_debug("live-verify line two");

    for name in ["debug.log", "debug.log.1", "debug.log.2"] {
        let p = dir.join(name);
        let len = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let body = std::fs::read_to_string(&p).unwrap_or_default();
        let head: String = body.chars().take(46).collect();
        println!(
            "{name}: exists={} len={len} z_bytes={} head={:?}",
            p.exists(),
            zcount(&p),
            head
        );
    }
}
