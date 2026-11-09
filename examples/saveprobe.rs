//! Temporary live check for save_state dedup (deleted after verification).
fn main() {
    let cfg = std::env::args().nth(1).expect("config home");
    std::env::set_var("XDG_CONFIG_HOME", &cfg);
    let dir = std::path::PathBuf::from(&cfg).join("tiles");
    let path = dir.join("state.json");
    let tmp = dir.join("state.json.tmp");
    let len = |p: &std::path::Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);

    // App::new seeds its pane from the working directory, so changing the
    // working directory changes the serialized state.
    std::fs::create_dir_all("/tmp/saveprobe-a").unwrap();
    std::fs::create_dir_all("/tmp/saveprobe-b").unwrap();
    std::env::set_current_dir("/tmp/saveprobe-a").unwrap();
    let app_a = tiles_tui_file_manager::app::App::new();
    println!("seed (cwd=a) -> ok={} len={}", tiles_tui_file_manager::config::save_state(&app_a).is_ok(), len(&path));

    std::env::set_current_dir("/tmp/saveprobe-b").unwrap();
    let app_b = tiles_tui_file_manager::app::App::new();
    // Obstruct the atomic write, so this save fails for real.
    std::fs::create_dir_all(&tmp).unwrap();
    let failed = tiles_tui_file_manager::config::save_state(&app_b);
    println!("changed content, tmp obstructed -> err={} (state.json still a-cwd: {})", failed.is_err(), len(&path));

    std::fs::remove_dir_all(&tmp).unwrap();
    let retry = tiles_tui_file_manager::config::save_state(&app_b);
    println!("identical retry -> ok={} len={} (a poisoned cache would keep the old file)", retry.is_ok(), len(&path));
}
