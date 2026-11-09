//! Temporary live check for save_state dedup (deleted after verification).
fn main() {
    let cfg = std::env::args().nth(1).expect("config home");
    std::env::set_var("XDG_CONFIG_HOME", &cfg);
    let path = std::path::PathBuf::from(&cfg).join("tiles/state.json");

    let app = tiles_tui_file_manager::app::App::new();
    let r1 = tiles_tui_file_manager::config::save_state(&app);
    println!("save#1 -> {:?} exists={} len={}", r1.is_ok(), path.exists(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));

    let before = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let r2 = tiles_tui_file_manager::config::save_state(&app);
    let after = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    println!("save#2 (identical) -> ok={} rewritten={}", r2.is_ok(), before != after);

    std::fs::remove_file(&path).expect("delete state file");
    let r3 = tiles_tui_file_manager::config::save_state(&app);
    println!("save#3 (file deleted) -> ok={} recreated={} len={}", r3.is_ok(), path.exists(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
}
