use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use dracon_terminal_engine::input::parser::Parser as TuiParser;
use dracon_terminal_engine::input::mapping::to_ui_event;
use dracon_terminal_engine::integration::ratatui::RatatuiBackend as EngineBackend;

// Ratatui Imports
use ratatui::Terminal;

use crate::app::{App, AppEvent};
use crate::config::FILE_WATCH_DEBOUNCE_MS;
mod tree_walk;
mod setup;
mod app;
mod clipboard;
mod config;
mod event;
mod event_helpers;
mod events;
mod handlers;
mod icons;
mod modules;
mod state;
mod ui;



#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Handle --version / -V early, before TUI init
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        match args[1].as_str() {
            "--version" | "-V" | "-v" => {
                println!("tiles {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("tiles {} — {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_DESCRIPTION"));
                println!("Usage: tiles [options]");
                println!("Options:");
                println!("  --version, -V    Show version");
                println!("  --help, -h        Show this help");
                return Ok(());
            }
            _ => {}
        }
    }

    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        eprintln!("[{}] PANIC at {}: {}", chrono::Utc::now(), location, msg);
    }));

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    let result = run_tty(shutdown_clone).await;
    shutdown.store(true, Ordering::Release);
    result
}

async fn run_tty(shutdown: Arc<AtomicBool>) -> color_eyre::Result<()> {
    crate::app::log_debug("run_tty start");
    let backend = EngineBackend::new(std::io::stdout())?;
    let tile_queue = backend.tile_queue();
    let mut terminal = Terminal::new(backend)?;

    let (app, event_tx, mut event_rx) = setup::setup_app(tile_queue);

    // Watcher Setup
    let tx_clone = event_tx.clone();
    let debouncer = notify_debouncer_mini::new_debouncer(
        Duration::from_millis(FILE_WATCH_DEBOUNCE_MS),
        move |res: notify_debouncer_mini::DebounceEventResult| {
            match res {
                Ok(events) => {
                    for event in events {
                        crate::app::log_debug(&format!("File watch event: {:?}", event));
                        let _ = crate::app::try_send_event(&tx_clone, AppEvent::FilesChangedOnDisk(event.path));
                    }
                }
                Err(e) => {
                    crate::app::log_debug(&format!("File watch error: {:?}", e));
                    let _ = crate::app::try_send_event(&tx_clone, AppEvent::StatusMsg(format!(
                        "File watch error: {}",
                        e
                    )));
                }
            }
        },
    )?;

    // Create EventLoopCtx — bundles all event loop state
    let mut ctx = handlers::event_loop_ctx::EventLoopCtx::new(
        app.clone(),
        event_tx.clone(),
        debouncer,
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
    );

    // 1. Input Loop (Thread)
    {
        let tx = ctx.event_tx.clone();
        let shutdown_input = shutdown.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            use std::os::fd::AsRawFd;
            let mut parser = TuiParser::new();
            let mut stdin = std::io::stdin();
            let fd = stdin.as_raw_fd();
            let mut buffer = [0; 1024];
            while !shutdown_input.load(Ordering::Relaxed) {
                // SAFETY: poll_input takes a borrowed file descriptor (BorrowedFd) and only
                // checks for input availability (returns bool). The fd must be valid at this point.
                // If stdin was closed or redirected such that the fd became invalid, the poll
                // would return an error (not UB). We validate fd < 0 before the unsafe call to
                // break cleanly on obviously invalid fds, but a stale fd in poll is safe since
                // poll returns an error. We hold the only reference to stdin here, so there are
                // no aliasing issues.
                let polled = if fd < 0 {
                    crate::app::log_debug("stdin fd is invalid (< 0), breaking input loop");
                    break;
                } else {
                    unsafe {
                        dracon_terminal_engine::backend::tty::poll_input(std::os::fd::BorrowedFd::borrow_raw(fd), 20)
                    }
                };
                match polled {
                    Ok(true) => match stdin.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            for byte in buffer.iter().take(n) {
                                if let Some(evt) = parser.advance(*byte) {
                                    if let Some(converted) = crate::event::convert_event(evt) {
                                        if let Some(ui_event) = to_ui_event(&converted) {
                                            let _ = tx.blocking_send(AppEvent::Ui(ui_event));
                                        }
                                        let _ = tx.blocking_send(AppEvent::Raw(converted));
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {
                        if let Some(evt) = parser.check_timeout() {
                            if let Some(converted) = crate::event::convert_event(evt) {
                                if let Some(ui_event) = to_ui_event(&converted) {
                                    let _ = tx.blocking_send(AppEvent::Ui(ui_event));
                                }
                                let _ = tx.blocking_send(AppEvent::Raw(converted));
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // 2. System Stats Loop (Tokio) — polls every 3s; fast enough for the Monitor view
    //    without burning CPU when the user is in Files/Editor/Git.
    {
        let tx = ctx.event_tx.clone();
        let shutdown_stats = shutdown.clone();
        tokio::spawn(async move {
            let mut sys_mod = crate::modules::system::SystemModule::new();
            loop {
                if shutdown_stats.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(data) = sys_mod.get_data() {
                    let _ = tx.send(AppEvent::SystemUpdated(data)).await;
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    // 3. Tick Loop (Tokio)
    {
        let tx = ctx.event_tx.clone();
        let shutdown_tick = shutdown.clone();
        tokio::spawn(async move {
            loop {
                if shutdown_tick.load(Ordering::Relaxed) {
                    break;
                }
                let _ = tx.send(AppEvent::Tick).await;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        });
    }

    // Initial State Setup
    let pane_count = {
        let mut app_guard = ctx.app.lock();
        app_guard.core.running = true;
        if let Ok(size) = terminal.size() {
            app_guard.core.terminal_size = (size.width, size.height);
        }
        app_guard.panes.len()
    };
    for i in 0..pane_count {
        let _ = event_tx.send(AppEvent::RefreshFiles(i)).await;
    }

    // Initial watch sync
    ctx.sync_watches();

    crate::app::log_debug("Entering main loop");

    loop {
        let mut needs_draw = false;

        while let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::Tick => {
                    needs_draw = ctx.handle_tick();
                }
                AppEvent::Raw(raw) => {
                    {
                        let mut app_guard = ctx.app.lock();
                        if setup::handle_event(
                            raw,
                            &mut app_guard,
                            ctx.event_tx.clone(),
                            &mut ctx.panes_needing_refresh,
                        ) {
                            needs_draw = true;
                        }
                        // Note: ui::draw already calls f.render_widget(Clear, f.area())
                        // so terminal.clear() is redundant and can cause flicker/black screen
                        // between view transitions. Removed to prevent black screen bug.
                    }
                }
                AppEvent::Ui(_ui_event) => {}
                AppEvent::SystemUpdated(data) => {
                    ctx.handle_system_updated(data);
                    needs_draw = true;
                }
                AppEvent::ConnectToRemote(pane_idx, bookmark_idx) => {
                    ctx.handle_connect_to_remote(pane_idx, bookmark_idx);
                }
                AppEvent::RemoteConnected(pane_idx, session) => {
                    ctx.handle_remote_connected(pane_idx, session);
                    needs_draw = true;
                }
                AppEvent::RefreshFiles(pane_idx) => {
                    if ctx.handle_refresh_files(pane_idx) {
                        // pane was valid
                    }
                }
                AppEvent::FilesChangedOnDisk(path) => {
                    let (needs_redraw, should_skip) = ctx.handle_files_changed_on_disk(path);
                    if should_skip {
                        continue;
                    }
                    needs_draw = needs_draw || needs_redraw;
                }
                AppEvent::PreviewRequested(pane_idx, path) => {
                    ctx.handle_preview_requested(pane_idx, path, app.clone());
                }
                AppEvent::SaveFile(path, content) => {
                    ctx.handle_save_file(path, content);
                    needs_draw = true;
                }
                AppEvent::CreateFile(path) => {
                    ctx.handle_create_file(path);
                }
                AppEvent::CreateFolder(path) => {
                    ctx.handle_create_folder(path);
                }
                AppEvent::Rename(old, new) => {
                    ctx.handle_rename(old, new);
                }
                AppEvent::Delete(path) => {
                    ctx.handle_delete(path);
                }
                AppEvent::TrashFile(path) => {
                    ctx.handle_trash_file(path);
                }
                AppEvent::Copy(src, dest) => {
                    ctx.handle_copy(src, dest, app.clone());
                }
                AppEvent::Symlink(src, dest) => {
                    if ctx.handle_symlink(src, dest) {
                        continue;
                    }
                }
                AppEvent::SpawnTerminal {
                    path,
                    new_tab,
                    remote,
                    command,
                } => {
                    ctx.handle_spawn_terminal(path, new_tab, remote, command);
                }
                AppEvent::SpawnDetached { cmd, args } => {
                    ctx.handle_spawn_detached(cmd, args);
                }
                AppEvent::KillProcess(pid) => {
                    let _ = crate::modules::system::SystemModule::kill_process(pid);
                }
                AppEvent::GitHistoryUpdated(
                    p_idx,
                    t_idx,
                    history,
                    pending,
                    branch,
                    ahead,
                    behind,
                    summary,
                    remotes,
                    stashes,
                ) => {
                    ctx.handle_git_history_updated(p_idx, t_idx, history, pending, branch, ahead, behind, summary, remotes, stashes);
                    needs_draw = true;
                }
                AppEvent::TaskProgress(id, progress, status) => {
                    ctx.handle_task_progress(id, progress, status);
                    needs_draw = true;
                }
                AppEvent::TaskFinished(id) => {
                    ctx.handle_task_finished(id);
                    needs_draw = true;
                }
                AppEvent::GlobalSearchUpdated(pane_idx, files, _meta) => {
                    ctx.handle_global_search_updated(pane_idx, files);
                    needs_draw = true;
                }
                AppEvent::SystemMonitor => {
                    ctx.handle_system_monitor();
                    needs_draw = true;
                }
                AppEvent::GitHistory => {
                    ctx.handle_git_history();
                    needs_draw = true;
                }
                AppEvent::Editor => {
                    ctx.handle_editor();
                    needs_draw = true;
                }
                AppEvent::StatusMsg(msg) => {
                    ctx.handle_status_msg(msg);
                    needs_draw = true;
                }
                AppEvent::AddToFavorites(path) => {
                    ctx.handle_add_to_favorites(path);
                    needs_draw = true;
                }
            }
        }

        // Handle Refreshes
        crate::handlers::refresh::handle_refreshes(&mut ctx.panes_needing_refresh, app.clone(), ctx.event_tx.clone()).await;

        if needs_draw {
            let mut app_guard = ctx.app.lock();
            if !app_guard.core.running {
                shutdown.store(true, Ordering::Release);
                break;
            }
            terminal.draw(|f| ui::draw(f, &mut app_guard))?;
        }

        tokio::time::sleep(Duration::from_millis(33)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};
    use dracon_terminal_engine::compositor::engine::TilePlacement;
    use crate::app::App;
    use crate::setup;
    
    #[test]
    fn startup_prime_populates_first_pane_listing() {
        let tmp = std::env::temp_dir().join(format!("tiles-startup-prime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("example.txt"), "ok").unwrap();

        let queue: Arc<StdMutex<Vec<TilePlacement>>> = Arc::new(StdMutex::new(Vec::new()));
        let mut app = App::new(queue);
        if let Some(fs) = app.current_file_state_mut() {
            fs.nav.current_path = tmp.clone();
            fs.list.files.clear();
            fs.list.metadata.clear();
            fs.list.selection.selected = None;
        }

        setup::prime_visible_tabs(&mut app);

        let fs = app.current_file_state().unwrap();
        assert!(
            !fs.list.files.is_empty(),
            "startup should hydrate first pane file list"
        );
        assert!(fs
            .list.files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("example.txt")));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
