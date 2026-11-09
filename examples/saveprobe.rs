//! Temporary live check for save_state dedup (deleted after verification).
fn main() {
    let cfg = std::env::args().nth(1).expect("config home");
    std::env::set_var("XDG_CONFIG_HOME", &cfg);
    let dir = std::path::PathBuf::from(&cfg).join("tiles");
    let path = dir.join("state.json");
    let tmp = dir.join("state.json.tmp");

    let app = tiles_tui_file_manager::app::App::new();

    // Seed a real state file so the dedup cache holds OLD content.
    let r0 = tiles_tui_file_manager::config::save_state(&app);
    println!("seed -> ok={} len={}", r0.is_ok(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));

    // Obstruct the tmp path, then change what would be saved.
    std::fs::create_dir_all(&tmp).expect("occupy tmp path");
    std::env::set_var("HOME", "/tmp/saveprobe-home");
    let app2 = tiles_tui_file_manager::app::App::new();
    let failed = tiles_tui_file_manager::config::save_state(&app2);
    println!("save with obstructed tmp -> err={}", failed.is_err());

    // Clear the obstruction; the identical retry must actually write.
    std::fs::remove_dir_all(&tmp).expect("clear tmp path");
    let retry = tiles_tui_file_manager::config::save_state(&app2);
    println!("retry -> ok={} len={}", retry.is_ok(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
}
