fn main() {
    let base = std::env::args().nth(1).unwrap();
    let dir = std::path::PathBuf::from(&base).join("tiles");
    println!("before: dir exists={} debug.log exists={}", dir.exists(), dir.join("debug.log").exists());
    tiles_tui_file_manager::app::log_debug("first call on a fresh install");
    tiles_tui_file_manager::app::log_debug("second call");
    println!("after:  dir exists={} debug.log exists={}", dir.exists(), dir.join("debug.log").exists());
    match std::fs::read_to_string(dir.join("debug.log")) {
        Ok(s) => println!("content: {s:?}"),
        Err(e) => println!("read err={e}"),
    }
}
