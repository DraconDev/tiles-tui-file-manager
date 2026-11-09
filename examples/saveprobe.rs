//! Temporary live check for save_state dedup (deleted after verification).
fn main() {
    let cfg = std::env::args().nth(1).expect("config home");
    std::env::set_var("XDG_CONFIG_HOME", &cfg);
    let dir = std::path::PathBuf::from(&cfg).join("tiles");
    let path = dir.join("state.json");
    let tmp = dir.join("state.json.tmp");

    let app = tiles_tui_file_manager::app::App::new();
    let r0 = tiles_tui_file_manager::config::save_state(&app);
    let mtime0 = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    println!("seed -> ok={} mtime0 set={}", r0.is_ok(), mtime0.is_some());

    std::fs::create_dir_all(&tmp).expect("occupy tmp path");
    let failed = tiles_tui_file_manager::config::save_state(&app);
    println!("identical save, tmp obstructed -> err={}", failed.is_err());

    std::fs::remove_dir_all(&tmp).expect("clear tmp path");
    let retry = tiles_tui_file_manager::config::save_state(&app);
    let mtime1 = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    println!(
        "identical retry -> ok={} rewritten={} (a poisoned cache would skip it)",
        retry.is_ok(),
        mtime0 != mtime1
    );
}
