use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use parking_lot::Mutex as PLMutex;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use dracon_terminal_engine::contracts::InputEvent as Event;
use dracon_terminal_engine::input::parser::Parser as TuiParser;
use dracon_terminal_engine::input::mapping::to_ui_event;
use dracon_terminal_engine::integration::ratatui::RatatuiBackend as EngineBackend;

// Ratatui Imports
use ratatui::Terminal;

use crate::app::{App, AppEvent, CurrentView, PreviewState};
use crate::config::{fuzzy_contains, FILE_WATCH_DEBOUNCE_MS, FUZZY_SEARCH, GIT_CACHE_TTL_SECONDS, MAX_TREE_DEPTH, MPSC_CHANNEL_CAPACITY};
use image::GenericImageView;
mod app;
mod config;
mod event;
mod event_helpers;
mod events;
mod icons;
mod modules;
mod servers;
mod state;
mod terminal;
mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

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

    let (app, event_tx, mut event_rx) = setup_app(tile_queue);

    // Watcher Setup
    let tx_clone = event_tx.clone();
    let mut debouncer = notify_debouncer_mini::new_debouncer(
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
    let mut watched_paths: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();

    // Helper to sync watched paths with current pane paths
    let mut last_synced_paths: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    let mut sync_watches = |app: &App, debouncer: &mut notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>| {
        let mut current_paths = std::collections::HashSet::new();
        
        // Collect all paths from panes
        for pane in &app.panes {
            for tab in &pane.tabs {
                current_paths.insert(tab.current_path.clone());
            }
        }
        
        // Also watch expanded folders in sidebar for editor view
        for path in &app.expanded_folders {
            current_paths.insert(path.clone());
        }

        // Fast bail-out: skip if nothing changed since last sync
        if current_paths == last_synced_paths {
            return;
        }
        last_synced_paths = current_paths.clone();

        // Add paths that aren't being watched yet
        for path in &current_paths {
            if !watched_paths.contains(path) {
                crate::app::log_debug(&format!("Starting file watch for: {:?}", path));
                if let Ok(()) = debouncer.watcher().watch(path, notify::RecursiveMode::NonRecursive) {
                    watched_paths.insert(path.clone());
                    crate::app::log_debug(&format!("Now watching: {:?}", path));
                } else {
                    crate::app::log_debug(&format!("Failed to watch: {:?}", path));
                }
            }
        }

        // Remove paths that are no longer current
        let to_remove: Vec<_> = watched_paths
            .iter()
            .filter(|p| !current_paths.contains(*p))
            .cloned()
            .collect();
        for path in to_remove {
            // Note: notify_debouncer_mini doesn't support unwatch
            // The watch will just be inactive when path is no longer accessed
            watched_paths.remove(&path);
        }
    };

    // 1. Input Loop (Thread)
    {
        let tx = event_tx.clone();
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
                        dracon_terminal_engine::backend::tty::poll_input(std::os::fd::BorrowedFd::borrow_raw(fd), 100)
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

    // 2. System Stats Loop (Tokio) — polls every 3s only when monitor is visible
    //    Skip collection when in Files/Editor/Git to save CPU
    {
        let tx = event_tx.clone();
        let shutdown_stats = shutdown.clone();
        let app_for_stats = app.clone();
        let sys_mod = std::sync::Arc::new(std::sync::Mutex::new(
            crate::modules::system::SystemModule::new()
        ));
        tokio::spawn(async move {
            let mut was_monitor_active = false;
            loop {
                if shutdown_stats.load(Ordering::Relaxed) {
                    break;
                }
                
                // Only collect stats when monitor view is active
                let is_monitor_active = {
                    let app_guard = app_for_stats.lock();
                    app_guard.current_view == CurrentView::Processes
                };
                
                if is_monitor_active {
                    was_monitor_active = true;
                    let sys_mod = sys_mod.clone();
                    let data = tokio::task::spawn_blocking(move || {
                        sys_mod.lock().unwrap().get_data()
                    })
                    .await
                    .ok()
                    .and_then(|r| r.ok());
                    if let Some(data) = data {
                        let _ = tx.send(AppEvent::SystemUpdated(data)).await;
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                } else {
                    // When leaving monitor, do one final capture so the overview
                    // isn't completely stale when user returns
                    if was_monitor_active {
                        was_monitor_active = false;
                        let sys_mod = sys_mod.clone();
                        let data = tokio::task::spawn_blocking(move || {
                            sys_mod.lock().unwrap().get_data()
                        })
                        .await
                        .ok()
                        .and_then(|r| r.ok());
                        if let Some(data) = data {
                            let _ = tx.send(AppEvent::SystemUpdated(data)).await;
                        }
                    }
                    // Sleep longer when monitor is not active
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
    }

    // 3. Tick Loop (Tokio)
    {
        let tx = event_tx.clone();
        let shutdown_tick = shutdown.clone();
        tokio::spawn(async move {
            loop {
                if shutdown_tick.load(Ordering::Relaxed) {
                    break;
                }
                let _ = tx.send(AppEvent::Tick).await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    // 4. Servers.toml Watcher (notify crate) — auto-reload when edited externally
    {
        let tx = event_tx.clone();
        let shutdown_watch = shutdown.clone();
        tokio::spawn(async move {
            use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
            use std::sync::mpsc::channel;

            let Some(toml_path) = crate::servers::servers_toml_path() else {
                return;
            };
            let Some(parent) = toml_path.parent() else {
                return;
            };

            let (watch_tx, watch_rx) = channel::<notify::Result<notify::Event>>();
            let mut watcher = match RecommendedWatcher::new(
                move |res| {
                    let _ = watch_tx.send(res);
                },
                Config::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    crate::app::log_debug(&format!("Failed to create servers.toml watcher: {}", e));
                    return;
                }
            };

            if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                crate::app::log_debug(&format!("Failed to watch servers.toml dir: {}", e));
                return;
            }

            let mut last_modified = std::time::Instant::now();
            loop {
                if shutdown_watch.load(Ordering::Relaxed) {
                    break;
                }
                // Poll with timeout so we can check shutdown flag
                match watch_rx.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(Ok(event)) => {
                        if event.paths.iter().any(|p| p == &toml_path) {
                            // Debounce: ignore events within 500ms of each other
                            if last_modified.elapsed() > std::time::Duration::from_millis(500) {
                                last_modified = std::time::Instant::now();
                                let _ = tx.send(AppEvent::ServersTomlChanged).await;
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        crate::app::log_debug(&format!("servers.toml watch error: {}", e));
                    }
                    Err(_) => {} // Timeout — check shutdown flag next loop
                }
            }
        });
    }

    // Initial State Setup
    let pane_count = {
        let mut app_guard = app.lock();
        app_guard.running = true;
        if let Ok(size) = terminal.size() {
            app_guard.terminal_size = (size.width, size.height);
        }
        app_guard.panes.len()
    };
    for i in 0..pane_count {
        let _ = event_tx.send(AppEvent::RefreshFiles(i)).await;
    }

    // Initial watch sync
    {
        let app_guard = app.lock();
        sync_watches(&app_guard, &mut debouncer);
    }

    crate::app::log_debug("Entering main loop");

    let mut panes_needing_refresh = std::collections::HashSet::new();
    let mut last_self_save: std::collections::HashMap<PathBuf, (std::time::SystemTime, u64)> =
        std::collections::HashMap::new();
    let mut last_watch_sync = std::time::Instant::now();
    const WATCH_SYNC_INTERVAL_MS: u64 = 2000;
    let mut last_activity = std::time::Instant::now();
    const IDLE_THRESHOLD_MS: u64 = 500;

    loop {
        let mut needs_draw = false;

        while let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::Tick => {
                    // Tick no longer syncs file watches — sync is now event-driven
                    // after RefreshFiles, tab changes, and folder expansion
                }
                AppEvent::Raw(raw) => {
                    {
                        let mut app_guard = app.lock();
                        if handle_event(
                            raw,
                            &mut app_guard,
                            event_tx.clone(),
                            &mut panes_needing_refresh,
                        ) {
                            needs_draw = true;
                            last_activity = std::time::Instant::now();
                        }
                        // Note: ui::draw already calls f.render_widget(Clear, f.area())
                        // so terminal.clear() is redundant and can cause flicker/black screen
                        // between view transitions. Removed to prevent black screen bug.
                    }
                }
                AppEvent::Ui(_ui_event) => {}
                AppEvent::SystemUpdated(data) => {
                    let mut app_guard = app.lock();
                    crate::modules::system::SystemModule::update_app_state(&mut app_guard, data);
                    needs_draw = true;
                }
                AppEvent::ServersTomlChanged => {
                    let mut app_guard = app.lock();
                    let new_servers = crate::servers::load_servers();
                    if new_servers != app_guard.servers {
                        let count = new_servers.len();
                        app_guard.servers = new_servers;
                        crate::app::log_debug(&format!(
                            "servers.toml changed externally, reloaded {} servers",
                            count
                        ));
                        needs_draw = true;
                    }
                }
                AppEvent::ConnectToRemote(pane_idx, bookmark_idx) => {
                    let (remote_opt, cached_session) = {
                        let app_guard = app.lock();
                        let remote_opt = app_guard.servers.get(bookmark_idx)
                            .cloned()
                            .map(crate::state::RemoteBookmark::from);
                        let cached = remote_opt.as_ref()
                            .and_then(|r| app_guard.remote_session_pool.get(&r.name).cloned());
                        (remote_opt, cached)
                    };
                    
                    if let Some(session) = cached_session {
                        // Reuse cached connection
                        let mut app_guard = app.lock();
                        if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                            if let Some(fs) = pane.current_state_mut() {
                                fs.remote_session = Some(session);
                                fs.bookmark_idx = Some(bookmark_idx);
                                fs.retry_count = 0;
                                fs.current_path = PathBuf::from("/");
                            }
                        }
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Connected to {} (cached)",
                            remote_opt.as_ref().map(|r| r.display_name()).unwrap_or_default()
                        )));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                    } else if let Some(remote) = remote_opt {
                        // Store bookmark_idx for potential reconnection
                        {
                            let mut app_guard = app.lock();
                            if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                                if let Some(fs) = pane.current_state_mut() {
                                    fs.bookmark_idx = Some(bookmark_idx);
                                    fs.retry_count = 0;
                                }
                            }
                        }
                        let tx = event_tx.clone();
                        let p_idx = pane_idx;
                        let remote_name = remote.name.clone();
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Connecting to {} ({})...",
                            remote.display_name(), remote.host
                        )));

                        tokio::spawn(async move {
                            let connect_result = tokio::task::spawn_blocking(move || {
                                crate::modules::remote::connect_remote(&remote)
                            })
                            .await;

                            match connect_result {
                                Ok(Ok(session)) => {
                                    let _ =
                                        tx.send(AppEvent::RemoteConnected(p_idx, session, remote_name)).await;
                                }
                                Ok(Err(e)) => {
                                    let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(format!(
                                        "Connection failed: {e}"
                                    )));
                                }
                                Err(e) => {
                                    let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(format!(
                                        "Connection task failed: {e}"
                                    )));
                                }
                            }
                        });
                    }
                }
                AppEvent::RemoteConnected(pane_idx, session, remote_name) => {
                    let mut app_guard = app.lock();
                    // Cache the session for reuse
                    app_guard.remote_session_pool.insert(remote_name.clone(), session.clone());
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                        if let Some(fs) = pane.current_state_mut() {
                            fs.remote_session = Some(session);
                            fs.current_path = PathBuf::from("/");
                            fs.retry_count = 0;
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::ReconnectRemote(pane_idx) => {
                    let bookmark_idx = {
                        let mut app_guard = app.lock();
                        if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                            if let Some(fs) = pane.current_state_mut() {
                                fs.retry_count += 1;
                            }
                        }
                        app_guard.panes.get(pane_idx)
                            .and_then(|p| p.current_state())
                            .and_then(|fs| fs.bookmark_idx)
                    };
                    if let Some(idx) = bookmark_idx {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                            "Connection lost. Reconnecting...".to_string()
                        ));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::ConnectToRemote(pane_idx, idx));
                    } else {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                            "Connection lost. Cannot auto-reconnect (no bookmark).".to_string()
                        ));
                    }
                    needs_draw = true;
                }
                AppEvent::RefreshFiles(pane_idx) => {
                    let t_refresh = std::time::Instant::now();
                    let current_path = {
                        let t_lock = std::time::Instant::now();
                        let mut app_guard = app.lock();
                        crate::app::log_debug(&format!("RefreshFiles lock took {:?}", t_lock.elapsed()));
                        let path = app_guard
                            .panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| fs.current_path.clone());
                        if let Some(ref p) = path {
                            app_guard.push_recent_folder(p.clone());
                        }
                        path
                    };
                    crate::app::log_debug(&format!("RefreshFiles handler total {:?}", t_refresh.elapsed()));
                    if current_path.is_none() {
                        continue;
                    }
                    panes_needing_refresh.insert(pane_idx);
                }
                AppEvent::FilesChangedOnDisk(path) => {
                    crate::app::log_debug(&format!("FilesChangedOnDisk: {:?}", path));
                    
                    // Check if this was a self-save by comparing file mtime and size
                    if let Some((saved_mtime, saved_size)) = last_self_save.get(&path) {
                        if let Ok(meta) = std::fs::metadata(&path) {
                            if let Ok(mtime) = meta.modified() {
                                let size: u64 = meta.len();
                                if mtime == *saved_mtime && size == *saved_size {
                                    last_self_save.remove(&path);
                                    continue; // Skip refreshing/reloading for our own saves
                                }
                            }
                        }
                    }

                    let app_guard = app.lock();
                    let mut needs_reload = Vec::new();

                    for (i, pane) in app_guard.panes.iter().enumerate() {
                        if let Some(fs) = pane.current_state() {
                            // Check if the changed path is in or under the current directory
                            let current_path = &fs.current_path;
                            let should_refresh = if let Some(parent) = path.parent() {
                                // File changed - check if parent is current dir or path is in current dir
                                parent == current_path.as_path() || path.starts_with(current_path)
                            } else {
                                // Directory changed
                                path == current_path.as_path() || path.starts_with(current_path)
                            };
                            
                            if should_refresh {
                                crate::app::log_debug(&format!("Refreshing pane {} for path {:?}", i, path));
                                panes_needing_refresh.insert(i);
                            }
                        }
                        if let Some(fs) = pane.current_state() {
                            if let Some(ref preview) = fs.preview {
                                if preview.path == path {
                                    if let Some(editor) = &preview.editor {
                                        let skip_because_active_editor = app_guard
                                            .editor_state
                                            .as_ref()
                                            .map(|e| e.path == path)
                                            .unwrap_or(false);
                                        if !skip_because_active_editor && !editor.modified {
                                            needs_reload.push((i, path.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(preview) = &app_guard.editor_state {
                        if preview.path == path {
                            needs_reload.retain(|(idx, _)| *idx != app_guard.focused_pane_index);
                        }
                    }

                    drop(app_guard);
                    for (p_idx, p_path) in needs_reload {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(p_idx, p_path));
                    }
                    needs_draw = true;
                }
                AppEvent::PreviewRequested(pane_idx, path) => {
                    let tx = event_tx.clone();
                    let path_clone = path.clone();
                    let app_clone = app.clone();
                    let (current_dir, preview_limit_mb, remote_session) = {
                        let app_guard = app.lock();
                        if let Some(pane) = app_guard.panes.get(pane_idx) {
                            if let Some(fs) = pane.current_state() {
                                (
                                    fs.current_path.clone(),
                                    app_guard.preview_max_mb.max(1),
                                    fs.remote_session.clone(),
                                )
                            } else {
                                (PathBuf::from("."), app_guard.preview_max_mb.max(1), None)
                            }
                        } else {
                            (PathBuf::from("."), app_guard.preview_max_mb.max(1), None)
                        }
                    };

                    tokio::spawn(async move {
                        let path = path_clone;
                        let path_str = path.to_string_lossy();
                        let content = if let Some(hash) = path_str.strip_prefix("git://") {
                            let hash_owned = hash.to_string();
                            match tokio::task::spawn_blocking(move || crate::modules::files::show_commit_patch(&current_dir, &hash_owned)).await {
                                Ok(Ok(c)) => c,
                                Ok(Err(e)) => format!("Error fetching commit data: {}", e),
                                Err(_) => "<Internal error>".to_string(),
                            }
                        } else if let Some(file_path) = path_str.strip_prefix("git-diff://") {
                            let file_path_owned = file_path.to_string();
                            if let Some(remote) = &remote_session {
                                match crate::modules::remote::show_file_diff(
                                    remote,
                                    &current_dir,
                                    &file_path_owned,
                                ) {
                                    Ok(content) => content,
                                    Err(e) => format!("Error fetching diff data: {}", e),
                                }
                            } else {
                                match tokio::task::spawn_blocking(move || crate::modules::files::show_file_diff(&current_dir, &file_path_owned)).await {
                                    Ok(Ok(content)) => content,
                                    Ok(Err(e)) => format!("Error fetching diff data: {}", e),
                                    Err(_) => "<Internal error>".to_string(),
                                }
                            }
                        } else if let Some(remote) = &remote_session {
                            match crate::modules::remote::is_dir(remote, &path) {
                                Ok(true) => format!(
                                    "\n\n   << PROJECT VIEW: {} >>\n\n   Select a file from the sidebar to begin editing.",
                                    path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "/".to_string())
                                ),
                                Ok(false) => {
                                    match crate::modules::remote::is_binary_file(remote, &path) {
                                        Ok((true, size_mb)) => {
                                            if size_mb < 50 {
                                                let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(format!(
                                                    "Downloading {} MB binary from remote...", size_mb
                                                )));
                                                match crate::modules::remote::download_remote_file(remote, &path) {
                                                    Ok(local_path) => {
                                                        dracon_terminal_engine::utils::spawn_detached(
                                                            "xdg-open",
                                                            vec![local_path.to_string_lossy().to_string()],
                                                        );
                                                        let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(format!(
                                                            "Opened {} ({} MB) locally", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), size_mb
                                                        )));
                                                        format!("<Binary file: {} MB - opened locally>", size_mb)
                                                    }
                                                    Err(e) => {
                                                        let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(format!(
                                                            "Download failed: {}", e
                                                        )));
                                                        format!("<Binary file: {} MB - download failed: {}>", size_mb, e)
                                                    }
                                                }
                                            } else {
                                                format!("<Binary file: {} MB - too large for auto-download>", size_mb)
                                            }
                                        }
                                        Ok((false, _)) => crate::modules::remote::read_to_string(remote, &path)
                                            .unwrap_or_else(|e| format!("Error reading remote file: {e}")),
                                        Err(e) => format!("Error checking remote file: {e}"),
                                    }
                                }
                                Err(e) => format!("Error probing remote path: {e}"),
                            }
                        } else if path.is_dir() {
                            format!(
                                "\n\n   << PROJECT VIEW: {} >>\n\n   Select a file from the sidebar to begin editing.",
                                path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "/".to_string())
                            )
                        } else {
                            let (is_binary, is_too_large, size_mb) =
                                crate::modules::files::check_file_suitability(
                                    &path,
                                    preview_limit_mb as u64 * 1024 * 1024,
                                );
                            if is_too_large {
                                format!("<File too large: {} MB>", size_mb)
                            } else if is_binary {
                                // Check if it's an image file
                                let ext = path.extension()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                let image_exts = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tiff"];
                                if image_exts.contains(&ext.as_str()) {
                                    // Try to load as image
                                    match image::open(&path) {
                                        Ok(img) => {
                                            let (w, h) = img.dimensions();
                                            format!("<Image: {}x{} {} KB - preview below>", w, h, size_mb)
                                        }
                                        Err(_) => format!("<Binary file: {} MB>", size_mb),
                                    }
                                } else {
                                    format!("<Binary file: {} MB>", size_mb)
                                }
                            } else {
                                std::fs::read_to_string(&path).unwrap_or_else(|e| format!("<Error reading file: {}>", e))
                            }
                        };

                        // Load image for preview if applicable
                        let mut image_data: Option<(Vec<u8>, u32, u32)> = None;
                        {
                            let ext = path.extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            let image_exts = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tiff"];
                            if image_exts.contains(&ext.as_str()) {
                                if let Ok(img) = image::open(&path) {
                                    let (w, h) = img.dimensions();
                                    let rgba = img.to_rgba8().into_raw();
                                    image_data = Some((rgba, w, h));
                                }
                            }
                        }

                        let mut editor = dracon_terminal_engine::widgets::TextEditor::with_content(&content);
                        if path_str.starts_with("git://") || path_str.starts_with("git-diff://") {
                            editor.language = "diff".to_string();
                            editor.read_only = true;
                        } else if let Some(rs) = &remote_session {
                            let is_dir = crate::modules::remote::is_dir(rs, &path).unwrap_or(false);
                            if is_dir {
                                editor.read_only = true;
                            }
                        } else if path.is_dir() {
                            editor.read_only = true;
                        } else {
                            editor.language = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();
                        }

                        {
                            let mut app_guard = app_clone.lock();
                            let preview = PreviewState {
                                path: path.clone(),
                                content,
                                editor: Some(editor),
                                last_saved: None,
                                image_data,
                                highlighted_lines: None,
                            };

                             if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                                 if let Some(fs) = pane.current_state_mut() {
                                     fs.preview = Some(preview.clone());
                                 }
                             }
                            let is_git_url = path_str.starts_with("git://")
                                || path_str.starts_with("git-diff://");
                            if is_git_url
                                || app_guard.current_view == CurrentView::Editor
                                || app_guard.current_view == CurrentView::Commit
                            {
                                app_guard.editor_state = Some(preview);
                                app_guard.sidebar_focus = false;
                            }
                        }
                        let _ = tx.send(AppEvent::Tick).await;
                    });
                }
                AppEvent::SaveFile(path, content) => {
                    let remote_for_save = {
                        let app_guard = app.lock();
                        app_guard
                            .panes
                            .iter()
                            .find_map(|pane| {
                                let fs = pane.current_state()?;
                                let preview = fs.preview.as_ref()?;
                                if preview.path == path {
                                    fs.remote_session.clone()
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                let fs = app_guard.current_file_state()?;
                                app_guard.editor_state.as_ref()?;
                                fs.remote_session.clone()
                            })
                    };

                    let save_res = if let Some(remote) = &remote_for_save {
                        crate::modules::remote::write_string(remote, &path, &content)
                    } else {
                        let tmp_path = path.with_extension(format!("{}.tmp", std::process::id()));
                        let res = std::fs::write(&tmp_path, &content).and_then(|_| std::fs::rename(&tmp_path, &path));
                        if res.is_err() {
                            let _ = std::fs::remove_file(&tmp_path);
                        }
                        res
                    };

                    match save_res {
                        Ok(_) => {
                            if remote_for_save.is_none() {
                                if let Ok(meta) = std::fs::metadata(&path) {
                                    if let Ok(mtime) = meta.modified() {
                                        let size: u64 = meta.len();
                                        if last_self_save.len() > 100 {
                                            last_self_save.clear();
                                        }
                                        last_self_save.insert(path.clone(), (mtime, size));
                                    }
                                }
                            }
                            let mut app_guard = app.lock();
                            if let Some(ref mut preview) = app_guard.editor_state {
                                if preview.path == path {
                                    preview.last_saved = Some(std::time::Instant::now());
                                    if let Some(ref mut editor) = preview.editor {
                                        editor.modified = false;
                                    }
                                    preview.highlighted_lines = None;
                                }
                            }
                            for pane in &mut app_guard.panes {
                                if let Some(fs) = pane.current_state_mut() {
                                    if let Some(ref mut preview) = fs.preview {
                                        if preview.path == path {
                                            preview.last_saved = Some(std::time::Instant::now());
                                            if let Some(ref mut editor) = preview.editor {
                                                editor.modified = false;
                                            }
                                            preview.highlighted_lines = None;
                                        }
                                    }
                                }
                            }

                            // Trigger refresh for panes showing this file's parent
                            if let Some(parent) = path.parent() {
                                for (i, pane) in app_guard.panes.iter().enumerate() {
                                    if let Some(fs) = pane.current_state() {
                                        if fs.current_path == parent {
                                            panes_needing_refresh.insert(i);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let mut app_guard = app.lock();
                            let msg = format!("Failed to save file: {}", e);
                            crate::app::log_debug(&msg);
                            app_guard.last_action_msg = Some((msg, std::time::Instant::now()));
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::CreateFile(path) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let result: Result<(), std::io::Error> = if let Some(remote) = remote {
                        crate::modules::remote::create_file(&remote, &path).map_err(|e| e)
                    } else {
                        std::fs::File::create(&path).map(|_| ())
                    };
                    if let Err(e) = result {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!("Failed to create file: {}", e)));
                    } else {
                        let focused_pane = app.lock().focused_pane_index;
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(focused_pane));
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(focused_pane, path));
                    }
                }
                AppEvent::CreateFolder(path) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let result: Result<(), std::io::Error> = if let Some(remote) = remote {
                        crate::modules::remote::create_dir_all(&remote, &path)
                    } else {
                        std::fs::create_dir_all(&path).map_err(|e| e)
                    };
                    if let Err(e) = result {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!("Failed to create folder: {}", e)));
                    } else {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(
                            app.lock().focused_pane_index,
                        ));
                    }
                }
                AppEvent::Rename(old, new) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let rename_res = if let Some(remote) = &remote {
                        crate::modules::remote::rename(remote, &old, &new)
                    } else {
                        std::fs::rename(&old, &new)
                    };
                    match rename_res {
                        Ok(_) => {
                            let mut app_guard = app.lock();
                            // Undo should move the path back to its original location.
                            app_guard
                                .undo_stack
                                .push(crate::app::UndoAction::Move(new.clone(), old.clone()));
                            app_guard.redo_stack.clear();
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(app_guard.focused_pane_index));
                        }
                        Err(e) => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!("Rename failed: {}", e)));
                        }
                    }
                }
                AppEvent::Delete(path) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let result: Result<(), std::io::Error> = if let Some(remote) = remote {
                        crate::modules::remote::remove_path(&remote, &path)
                    } else if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                    } else {
                        std::fs::remove_file(&path)
                    };
                    let focused = app.lock().focused_pane_index;
                    if let Err(e) = result {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Delete failed: {} - {}",
                            path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                            e
                        )));
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(focused));
                }
                AppEvent::TrashFile(path) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let focused = app.lock().focused_pane_index;
                    if remote.is_some() {
                        // Remote files: fall back to permanent delete since trash doesn't work remotely
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                            "Remote files cannot be trashed. Use Delete for permanent removal.".to_string()
                        ));
                    } else {
                        match trash::delete(&path) {
                            Ok(_) => {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                    "Trashed: {}",
                                    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
                                )));
                            }
                            Err(e) => {
                                let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                    "Trash failed: {} - {}",
                                    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                    e
                                )));
                            }
                        }
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(focused));
                }
                AppEvent::Chmod(path, mode) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    let focused = app.lock().focused_pane_index;
                    let result = if let Some(remote) = remote {
                        crate::modules::remote::chmod(&remote, &path, mode)
                    } else {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = std::fs::metadata(&path)?.permissions();
                        perms.set_mode(mode);
                        std::fs::set_permissions(&path, perms)
                    };
                    match result {
                        Ok(_) => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Permissions set to {:o} for {}",
                                mode,
                                path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
                            )));
                        }
                        Err(e) => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "chmod failed: {}", e
                            )));
                        }
                    }
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(focused));
                }
                AppEvent::ComputeChecksums(path) => {
                    let tx = event_tx.clone();
                    let app_clone = app.clone();
                    tokio::spawn(async move {
                        let remote = {
                            let app_guard = app_clone.lock();
                            app_guard
                                .current_file_state()
                                .and_then(|fs| fs.remote_session.clone())
                        };
                        
                        let result = if let Some(remote) = &remote {
                            crate::modules::remote::compute_checksums(remote, &path)
                        } else {
                            crate::modules::files::compute_checksums(&path)
                        };
                        
                        match result {
                            Ok((md5, sha256)) => {
                                let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                {
                                    let mut app_guard = app_clone.lock();
                                    app_guard.checksum_cache.insert(path.clone(), (md5, sha256));
                                }
                                let _ = tx.send(AppEvent::StatusMsg(format!(
                                    "Checksums computed for {}", file_name
                                ))).await;
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::StatusMsg(format!(
                                    "Checksum failed: {}", e
                                ))).await;
                            }
                        }
                    });
                }
                AppEvent::Copy(src, dest) => {
                    let tx = event_tx.clone();
                    let app_clone = app.clone();
                    let src_name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
                    let task_id = uuid::Uuid::new_v4();

                    // Announce start
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::TaskProgress(task_id, 0.0, format!("Copying {}...", src_name)));

                    tokio::spawn(async move {
                        let remote = {
                            let app_guard = app_clone.lock();
                            app_guard
                                .current_file_state()
                                .and_then(|fs| fs.remote_session.clone())
                        };

                        let copied = if let Some(remote) = &remote {
                            crate::modules::remote::copy_recursive(remote, &src, &dest).is_ok()
                        } else {
                            dracon_terminal_engine::utils::copy_recursive(&src, &dest).is_ok()
                        };

                        if copied {
                            let mut app_guard = app_clone.lock();
                            app_guard
                                .undo_stack
                                .push(crate::app::UndoAction::Copy(src.clone(), dest.clone()));
                            app_guard.redo_stack.clear();
                        }

                        // Finish task
                        let _ = tx.send(AppEvent::TaskFinished(task_id)).await;

                        let mut panes_to_refresh = std::collections::HashSet::new();
                        if let Some(parent) = dest.parent() {
                            let app_guard = app_clone.lock();
                            for (i, pane) in app_guard.panes.iter().enumerate() {
                                if let Some(fs) = pane.current_state() {
                                    if fs.current_path == parent {
                                        panes_to_refresh.insert(i);
                                    }
                                }
                            }
                        }
                        if panes_to_refresh.is_empty() {
                            let _ = tx.send(AppEvent::RefreshFiles(0)).await;
                        } else {
                            for pane_idx in panes_to_refresh {
                                let _ = tx.send(AppEvent::RefreshFiles(pane_idx)).await;
                            }
                        }
                    });
                }
                AppEvent::CompareFiles(path_a, path_b) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    
                    let diff_content = if let Some(remote) = &remote {
                        crate::modules::remote::diff_files(remote, &path_a, &path_b)
                            .unwrap_or_else(|e| format!("Error computing diff: {}", e))
                    } else {
                        crate::modules::files::diff_files(&path_a, &path_b)
                            .unwrap_or_else(|e| format!("Error computing diff: {}", e))
                    };
                    
                    let mut editor = dracon_terminal_engine::widgets::TextEditor::with_content(&diff_content);
                    editor.language = "diff".to_string();
                    editor.read_only = true;
                    
                    let pane_idx = {
                        let app_guard = app.lock();
                        app_guard.focused_pane_index
                    };
                    let mut app_guard = app.lock();
                    if let Some(fs) = app_guard.panes.get_mut(pane_idx).and_then(|p| p.current_state_mut()) {
                        fs.preview = Some(crate::state::PreviewState {
                            path: path_a.clone(),
                            editor: Some(editor),
                            content: diff_content,
                            last_saved: None,
                            image_data: None,
                            highlighted_lines: None,
                        });
                    }
                    needs_draw = true;
                }
                AppEvent::CreateArchive(paths, dest) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        let result = if let Some(remote) = &remote {
                            tokio::task::spawn_blocking({
                                let remote = remote.clone();
                                let paths = paths.clone();
                                let dest = dest.clone();
                                move || crate::modules::remote::create_archive(&remote, &paths, &dest)
                            }).await.unwrap_or(Err(std::io::Error::new(std::io::ErrorKind::Other, "spawn failed")))
                        } else {
                            crate::modules::files::create_archive(&paths, &dest).await
                        };
                        
                        match result {
                            Ok(_) => {
                                let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(
                                    format!("Created archive: {}", dest.display())
                                ));
                            }
                            Err(e) => {
                                let _ = crate::app::try_send_event(&tx, AppEvent::StatusMsg(
                                    format!("Failed to create archive: {}", e)
                                ));
                            }
                        }
                    });
                }
                AppEvent::UploadToRemote(src, dest) => {
                    let tx = event_tx.clone();
                    let app_clone = app.clone();
                    let src_name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
                    let task_id = uuid::Uuid::new_v4();

                    let _ = crate::app::try_send_event(&event_tx, AppEvent::TaskProgress(task_id, 0.0, format!("Uploading {}...", src_name)));

                    tokio::spawn(async move {
                        let remote = {
                            let app_guard = app_clone.lock();
                            app_guard
                                .current_file_state()
                                .and_then(|fs| fs.remote_session.clone())
                        };

                        let uploaded = if let Some(remote) = &remote {
                            crate::modules::remote::upload_file(remote, &src, &dest).is_ok()
                        } else {
                            false
                        };

                        if uploaded {
                            let _ = tx.send(AppEvent::StatusMsg(format!(
                                "Uploaded {}", src_name
                            ))).await;
                        } else {
                            let _ = tx.send(AppEvent::StatusMsg(format!(
                                "Failed to upload {}", src_name
                            ))).await;
                        }

                        let _ = tx.send(AppEvent::TaskFinished(task_id)).await;
                        let _ = tx.send(AppEvent::RefreshFiles(0)).await;
                    });
                }
                AppEvent::Symlink(src, dest) => {
                    let remote = {
                        let app_guard = app.lock();
                        app_guard
                            .current_file_state()
                            .and_then(|fs| fs.remote_session.clone())
                    };
                    if remote.is_some() {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(
                            "Symlink is not supported for remote panes".to_string(),
                        ));
                        continue;
                    }
                    let result = {
                        #[cfg(unix)]
                        {
                            std::os::unix::fs::symlink(&src, &dest)
                        }
                        #[cfg(windows)]
                        {
                            if src.is_dir() {
                                std::os::windows::fs::symlink_dir(&src, &dest)
                            } else {
                                std::os::windows::fs::symlink_file(&src, &dest)
                            }
                        }
                    };

                    match result {
                        Ok(_) => {
                            if let Some(parent) = dest.parent() {
                                let app_guard = app.lock();
                                for (i, pane) in app_guard.panes.iter().enumerate() {
                                    if let Some(fs) = pane.current_state() {
                                        if fs.current_path == parent {
                                            panes_needing_refresh.insert(i);
                                        }
                                    }
                                }
                            }
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                                "Linked {} -> {}",
                                dest.display(),
                                src.display()
                            )));
                        }
                        Err(e) => {
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!("Symlink failed: {}", e)));
                        }
                    }
                }
                AppEvent::FolderSizesUpdated(pane_idx, sizes) => {
                    let mut app_guard = app.lock();
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                        if let Some(fs) = pane.current_state_mut() {
                            fs.folder_sizes.extend(sizes);
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::SpawnTerminal {
                    path,
                    new_tab,
                    remote,
                    command,
                } => {
                    let remote_cmd = remote.as_ref().map(|r| {
                        crate::modules::remote::build_remote_terminal_command(
                            r,
                            &path,
                            command.as_deref(),
                        )
                    });
                    let cmd_str = remote_cmd.as_deref().or(command.as_deref());
                    crate::terminal::spawn_terminal_at(&path, new_tab, cmd_str);
                }
                AppEvent::SpawnDetached { cmd, args } => {
                    dracon_terminal_engine::utils::spawn_detached(&cmd, args);
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
                    let mut app_guard = app.lock();
                    if p_idx >= app_guard.panes.len() {
                        crate::app::log_debug(&format!(
                            "GitHistoryUpdated: pane_idx {} out of bounds (panes: {})",
                            p_idx,
                            app_guard.panes.len()
                        ));
                    } else if let Some(pane) = app_guard.panes.get_mut(p_idx) {
                        // Store git data in the specified tab, or active tab as fallback
                        let tab_idx = if t_idx < pane.tabs.len() { t_idx } else { pane.active_tab_index };
                        if let Some(fs) = pane.tabs.get_mut(tab_idx) {
                            fs.git_history = history;
                            fs.git_pending = pending;
                            fs.git_branch = branch;
                            fs.git_ahead = ahead;
                            fs.git_behind = behind;
                            fs.git_summary = summary;
                            fs.git_remotes = remotes;
                            fs.git_stashes = stashes;
                            fs.git_cache_until = Some(Instant::now() + Duration::from_secs(GIT_CACHE_TTL_SECONDS));
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::TaskProgress(id, progress, status) => {
                    let mut app_guard = app.lock();
                    if let Some(task) = app_guard.background_tasks.iter_mut().find(|t| t.id == id) {
                        task.progress = progress;
                        task.status = status;
                    } else {
                        app_guard.background_tasks.push(crate::app::BackgroundTask {
                            id,
                            name: "Task".to_string(),
                            status,
                            progress,
                        });
                    }
                    needs_draw = true;
                }
                AppEvent::TaskFinished(id) => {
                    let mut app_guard = app.lock();
                    app_guard.background_tasks.retain(|t| t.id != id);
                    needs_draw = true;
                }
                AppEvent::GlobalSearchUpdated(pane_idx, files, _meta) => {
                    let mut app_guard = app.lock();
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                        if let Some(fs) = pane.current_state_mut() {
                            fs.files = files;
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::SystemMonitor => {
                    let mut app_guard = app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.current_view = CurrentView::Processes;
                    needs_draw = true;
                }
                AppEvent::GitHistory => {
                    let mut app_guard = app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.current_view = CurrentView::Git;
                    let pane_idx = app_guard.focused_pane_index;
                    needs_draw = true;
                    drop(app_guard);
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                }
                AppEvent::Editor => {
                    let mut app_guard = app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.current_view = CurrentView::Editor;
                    app_guard.load_view_prefs(CurrentView::Editor);
                    app_guard.apply_split_mode(false);
                    let pane_idx = app_guard.focused_pane_index;
                    let dir_path = app_guard
                        .panes
                        .get(pane_idx)
                        .and_then(|p| p.current_state())
                        .map(|fs| fs.current_path.clone());
                    needs_draw = true;
                    drop(app_guard);
                    if let Some(path) = dir_path {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(pane_idx, path));
                    }
                }
                AppEvent::StatusMsg(msg) => {
                    let mut app_guard = app.lock();
                    app_guard.last_action_msg = Some((msg, std::time::Instant::now()));
                    needs_draw = true;
                }
                AppEvent::AddToFavorites(path) => {
                    let mut app_guard = app.lock();
                    // Only add if path exists and not already in favorites
                    if path.exists() && !app_guard.starred.contains(&path) {
                        app_guard.starred.push(path.clone());
                        // Wrap save_state to prevent crash if serialization fails
                        crate::config::save_state_quiet(&app_guard);
                        let display_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Added to favorites: {}",
                            display_name
                        )));
                    }
                    needs_draw = true;
                }
            }
        }

        // Handle Refreshes
        for pane_idx in panes_needing_refresh.drain() {
            let (path, remote, current_filter, current_generation, git_view, tree_expanded) = {
                let app_guard = app.lock();
                if let Some(pane) = app_guard.panes.get(pane_idx) {
                    if let Some(fs) = pane.current_state() {
                        (
                            fs.current_path.clone(),
                            fs.remote_session.clone(),
                            fs.search_filter.clone(),
                            fs.search_generation,
                            matches!(app_guard.current_view, CurrentView::Git | CurrentView::Commit),
                            app_guard.expanded_folders.clone(),
                        )
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            };

let list_path_for_filter = path.clone();

            let tx = event_tx.clone();
            let app_clone = app.clone();
            let app_clone_for_health = app.clone();
            let expanded_folders = tree_expanded;
            tokio::spawn(async move {
                let list_path = path.clone();
                let list_remote = remote.clone();
                let list_filter = current_filter.clone();
                let start_generation = current_generation;
                let tx_retry = tx.clone();
                let (tree_files, mut metadata, g_files, g_meta): (Vec<(PathBuf, u16)>, std::collections::HashMap<PathBuf, crate::state::FileMetadata>, Vec<PathBuf>, std::collections::HashMap<PathBuf, crate::state::FileMetadata>) =
                    tokio::task::spawn_blocking(move || {
                        let t_dir = std::time::Instant::now();

                        if let Some(session) = &list_remote {
                            // Remote: use SSH directory listing
                            match crate::modules::remote::read_dir_with_metadata(session, &list_path) {
                                Ok((files, meta)) => {
                                    // Mark connection as healthy
                                    let mut app_guard = app_clone_for_health.lock();
                                    app_guard.remote_health.insert(session.name.clone(), (true, std::time::Instant::now()));
                                    drop(app_guard);
                                    
                                    let tree_files: Vec<(PathBuf, u16)> = files.into_iter().map(|p| (p, 0)).collect();
                                    let trimmed_filter = list_filter.trim();
                                    let g_result = if trimmed_filter.len() > 3 {
                                        crate::modules::remote::global_search(
                                            session,
                                            &list_path,
                                            trimmed_filter,
                                        )
                                    } else {
                                        (Vec::new(), std::collections::HashMap::new())
                                    };
                                    crate::app::log_debug(&format!("remote read_dir+search took {:?} for {:?}", t_dir.elapsed(), list_path));
                                    (tree_files, meta, g_result.0, g_result.1)
                                }
                                Err(e) => {
                                    // Mark connection as unhealthy
                                    let mut app_guard = app_clone_for_health.lock();
                                    app_guard.remote_health.insert(session.name.clone(), (false, std::time::Instant::now()));
                                    let retry_count = app_guard.panes.get(pane_idx)
                                        .and_then(|p| p.current_state())
                                        .map(|fs| fs.retry_count)
                                        .unwrap_or(0);
                                    drop(app_guard);
                                    
                                    crate::app::log_debug(&format!("remote read_dir failed for {:?}: {} (retry={})", list_path, e, retry_count));
                                    
                                    // Trigger reconnection if under retry limit
                                    if retry_count < 3 {
                                        let _ = crate::app::try_send_event(&tx_retry, AppEvent::ReconnectRemote(pane_idx));
                                    } else {
                                        let _ = crate::app::try_send_event(&tx_retry, AppEvent::StatusMsg(
                                            format!("Connection to {} failed after 3 retries", session.name)
                                        ));
                                    }
                                    (Vec::new(), std::collections::HashMap::new(), Vec::new(), std::collections::HashMap::new())
                                }
                            }
                        } else {
                            // Local: walk expanded folders (Dolphin-style inline tree)
                            let max_depth = MAX_TREE_DEPTH;
                            let mut tree_files: Vec<(PathBuf, u16)> = Vec::new();
                            fn walk_tree(
                                path: &std::path::Path,
                                depth: u16,
                                max_depth: u16,
                                expanded: &std::collections::HashSet<PathBuf>,
                                hidden: bool,
                                tree_files: &mut Vec<(PathBuf, u16)>,
                            ) {
                                if depth >= max_depth {
                                    return;
                                }
                                let Ok(entries) = std::fs::read_dir(path) else { return };
                                let mut sorted: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                                sorted.sort_by(|a, b| {
                                    let a_is_dir = a.path().is_dir();
                                    let b_is_dir = b.path().is_dir();
                                    if a_is_dir != b_is_dir {
                                        return if a_is_dir {
                                            std::cmp::Ordering::Less
                                        } else {
                                            std::cmp::Ordering::Greater
                                        };
                                    }
                                    a.file_name().cmp(&b.file_name())
                                });
                                for entry in sorted {
                                    let p = entry.path();
                                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                                    if !hidden && name.starts_with('.') {
                                        continue;
                                    }
                                    tree_files.push((p.clone(), depth));
                                    if p.is_dir() && expanded.contains(&p) {
                                        walk_tree(&p, depth + 1, max_depth, expanded, hidden, tree_files);
                                    }
                                }
                            }
                            walk_tree(&list_path, 0, max_depth, &expanded_folders, false, &mut tree_files);
                            // Collect metadata for all tree items
                            let tree_paths: Vec<PathBuf> = tree_files.iter().map(|(p, _)| p.clone()).collect();
                            let (files_meta, g_files, g_meta) = {
                                let meta = crate::modules::files::read_dir_recursive_meta(&tree_paths);
                                let trimmed_filter = list_filter.trim();
                                let g_result = if trimmed_filter.len() > 3 {
                                    let search_root =
                                        dirs::home_dir().unwrap_or_else(|| list_path.clone());
                                    crate::modules::files::global_search(&search_root, trimmed_filter)
                                } else {
                                    (Vec::new(), std::collections::HashMap::new())
                                };
                                (meta.1, g_result.0, g_result.1)
                            };

                            crate::app::log_debug(&format!("read_dir+search took {:?} for {:?}", t_dir.elapsed(), list_path));
                            (tree_files, files_meta, g_files, g_meta)
                        }
                    })
                    .await
                    .unwrap_or_else(|_| {
                        (
                            Vec::new(),
                            std::collections::HashMap::new(),
                            Vec::new(),
                            std::collections::HashMap::new(),
                        )
                    });

                {
                    let t_apply = std::time::Instant::now();
                    let mut app_guard = app_clone.lock();
                    crate::app::log_debug(&format!("apply lock took {:?}", t_apply.elapsed()));
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                            if let Some(fs) = pane.current_state_mut() {
                                // RACE CONDITION CHECK:
                                // If filter changed while we were reading, discard stale results
                                if fs.search_generation != start_generation {
                                    crate::app::log_debug(&format!(
                                        "RefreshFiles: generation mismatch (pane={}), dropping stale results",
                                        pane_idx
                                    ));
                                    return;
                                }

                            // Pre-compute search filter values to avoid repeated allocations
                            let search_filter_lower = fs.search_filter.to_lowercase();
                            let has_search_filter = !fs.search_filter.is_empty();
                            let show_hidden = fs.show_hidden;
                            
                            // tree_files is Vec<(PathBuf, u16)> — keep pairs intact through filter/sort
                            let mut paired: Vec<(PathBuf, u16)> = tree_files.into_iter().filter(|(p, _)| {
                                let is_hidden = p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|s| s.starts_with('.'))
                                    .unwrap_or(false);

                                if !show_hidden && is_hidden {
                                    return false;
                                }

                                if has_search_filter {
                                    let name = p
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    let matches = if FUZZY_SEARCH {
                                        fuzzy_contains(name, &fs.search_filter)
                                    } else {
                                        name.to_lowercase().contains(&search_filter_lower)
                                    };
                                    if !matches {
                                        return false;
                                    }
                                }

                                true
                            }).collect();

                            // Search filter: include ancestor folders so matching children are visible
                            if has_search_filter {
                                use std::collections::HashSet;
                                let filter_lower = fs.search_filter.to_lowercase();
                                let mut keep: HashSet<PathBuf> = HashSet::new();
                                for (p, _) in &paired {
                                    let name = p.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    let matches = if FUZZY_SEARCH {
                                        fuzzy_contains(name, &fs.search_filter)
                                    } else {
                                        name.to_lowercase().contains(&filter_lower)
                                    };
                                    if matches {
                                        keep.insert(p.clone());
                                    }
                                }
                                let mut keep_with_parents = keep.clone();
                                for p in &keep {
                                    let mut current = p.parent();
                                    while let Some(pp) = current {
                                        if pp == list_path_for_filter.as_path() {
                                            break;
                                        }
                                        keep_with_parents.insert(pp.to_path_buf());
                                        current = pp.parent();
                                    }
                                }
                                let mut new_paired: Vec<(PathBuf, u16)> = Vec::new();
                                for (p, d) in paired.into_iter() {
                                    if keep_with_parents.contains(&p) {
                                        new_paired.push((p, d));
                                    }
                                }
paired = new_paired;
                            }

                            // Tree order from walk_tree is already sorted (folders-first, alphabetical).
                            // Do NOT re-sort here — it would scatter children away from parent folders.

                            fs.local_count = paired.len();

                            if !g_files.is_empty() {
                                for gf in &g_files {
                                    if !paired.iter().any(|(p, _)| p == gf) {
                                        paired.push((gf.clone(), 0));
                                    }
                                }
                                metadata.extend(g_meta);
                            }

                            // Split paired into files + depths
                            let tree_file_depths: Vec<u16> = paired.iter().map(|(_, d)| *d).collect();
                            let files: Vec<PathBuf> = paired.into_iter().map(|(p, _)| p).collect();

                            fs.tree_file_depths = tree_file_depths;
                            fs.files = files;
                            fs.metadata = metadata.clone();
                            fs.folder_sizes.clear(); // Clear stale folder sizes

                            // Calculate folder sizes with rate limiting (max once per 5 seconds)
                            // to avoid recursive directory walks on every navigation
                            let should_calc_sizes = fs.last_folder_size_calc
                                .map(|last| last.elapsed() >= Duration::from_secs(5))
                                .unwrap_or(true);
                            
                            if should_calc_sizes {
                                fs.last_folder_size_calc = Some(Instant::now());
                                let dirs_to_size: Vec<PathBuf> = fs.files.iter()
                                    .filter(|p| metadata.get(*p).map(|m| m.is_dir).unwrap_or(false))
                                    .cloned()
                                    .collect();
                                if !dirs_to_size.is_empty() {
                                    let size_remote = remote.clone();
                                    let size_tx = tx.clone();
                                    let size_pane_idx = pane_idx;
                                    tokio::spawn(async move {
                                        let mut sizes = std::collections::HashMap::new();
                                        for dir_path in dirs_to_size {
                                            let size = if let Some(ref session) = size_remote {
                                                crate::modules::remote::folder_size(session, &dir_path).unwrap_or(0)
                                            } else {
                                                crate::modules::files::folder_size(&dir_path)
                                            };
                                            sizes.insert(dir_path, size);
                                        }
                                        let _ = size_tx.send(AppEvent::FolderSizesUpdated(size_pane_idx, sizes)).await;
                                    });
                                }
                            }

                            // Apply pending selection and scroll (e.g., after navigate_up)
                            if let Some((pending_path, pending_scroll)) = fs.pending_select_path.take() {
                                if let Some(idx) = fs.files.iter().position(|p| p == &pending_path)
                                {
                                    fs.selection.selected = Some(idx);
                                    fs.table_state.select(Some(idx));
                                    *fs.table_state.offset_mut() = fs.clamped_scroll(pending_scroll);
                                }
                            }
                        }
                    }
                }
                let _ = tx.send(AppEvent::Tick).await;

                if git_view {
                    let git_path = path.clone();
                    let git_remote = remote.clone();
                    let app_for_git = app_clone.clone();
                    let tx_for_git = tx.clone();
                    let _git_cache_ttl = Duration::from_secs(GIT_CACHE_TTL_SECONDS);
                    let should_fetch = {
                        let app_guard = app_for_git.lock();
                        app_guard
                            .panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| {
                                fs.git_cache_until
                                    .map(|until| Instant::now() >= until)
                                    .unwrap_or(true)
                            })
                            .unwrap_or(false)
                    };
                    if !should_fetch {
                        return;
                    }
                    tokio::spawn(async move {
                    let git_fetch_path = git_path.clone();
                    let git_data = tokio::task::spawn_blocking(move || {
                        if let Some(session) = &git_remote {
                            crate::modules::remote::fetch_git_data(session, &git_fetch_path)
                        } else {
                            crate::modules::files::fetch_git_data(&git_fetch_path)
                        }
                    })
                    .await
                    .ok()
                    .flatten();

                    let path_still_active = {
                        let app_guard = app_for_git.lock();
                        app_guard
                            .panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| fs.current_path == git_path)
                            .unwrap_or(false)
                    };
                    if !path_still_active {
                        return;
                    }

                    // Get the active tab index for this pane so git data lands in the right place
                    let active_tab_idx = {
                        let app_guard = app_for_git.lock();
                        app_guard
                            .panes
                            .get(pane_idx)
                            .map(|p| p.active_tab_index)
                            .unwrap_or(0)
                    };

                    let (history, pending, branch, ahead, behind, summary, remotes, stashes) =
                        git_data.unwrap_or_else(|| {
                            (
                                Vec::new(),
                                Vec::new(),
                                String::new(),
                                0,
                                0,
                                String::new(),
                                Vec::new(),
                                Vec::new(),
                            )
                        });

                    let branch_opt = if branch.is_empty() { None } else { Some(branch) };
                    let summary_opt = if summary.is_empty() {
                        None
                    } else {
                        Some(summary)
                    };

                    let _ = tx_for_git
                        .send(AppEvent::GitHistoryUpdated(
                            pane_idx,
                            active_tab_idx,
                            history,
                            pending,
                            branch_opt,
                            ahead,
                            behind,
                            summary_opt,
                            remotes,
                            stashes,
                        ))
                        .await;
                });
                }
            });
        }

        if needs_draw {
            last_activity = std::time::Instant::now();
            let mut app_guard = app.lock();
            if !app_guard.running {
                shutdown.store(true, Ordering::Release);
                break;
            }
            terminal.draw(|f| ui::draw(f, &mut app_guard))?;
        }

        // Adaptive sleep: 50ms when active, 100ms when idle
        let sleep_ms = if last_activity.elapsed() >= Duration::from_millis(IDLE_THRESHOLD_MS) {
            100
        } else {
            50
        };
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }

    Ok(())
}

fn setup_app(
    tile_queue: Arc<StdMutex<Vec<dracon_terminal_engine::compositor::engine::TilePlacement>>>,
) -> (
    Arc<PLMutex<App>>,
    mpsc::Sender<AppEvent>,
    mpsc::Receiver<AppEvent>,
) {
    let (tx, rx) = mpsc::channel(MPSC_CHANNEL_CAPACITY);
    let mut app = App::new(tile_queue);

    if let Some(state) = crate::config::load_state() {
        if !state.panes.is_empty() {
            app.panes = state.panes;
        }
        for pane in &mut app.panes {
            if pane.tabs.is_empty() {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                pane.tabs.push(crate::state::FileState::new(
                    cwd,
                    None,
                    None,
                    app.default_show_hidden,
                    app.single_columns.clone(),
                    crate::state::FileColumn::Name,
                    true,
                ));
                pane.active_tab_index = 0;
            } else if pane.active_tab_index >= pane.tabs.len() {
                pane.active_tab_index = 0;
            }

            for tab in &mut pane.tabs {
                // Never trust persisted transient tab data; force a clean first refresh.
                tab.files.clear();
                tab.metadata.clear();
                tab.search_filter.clear();
                tab.local_count = 0;
                tab.selection.clear_multi();
                tab.selection.anchor = None;
                tab.selection.selected = None;
                *tab.table_state.offset_mut() = 0;
            }
        }
        app.focused_pane_index = state.focused_pane_index;
        if app.focused_pane_index >= app.panes.len() {
            app.focused_pane_index = 0;
        }

        // Ensure CWD is active on start, keeping history
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(pane) = app.panes.get_mut(0) {
                if let Some(fs) = pane.current_state_mut() {
                    // Always start local on pane 1/tab active, otherwise a persisted
                    // remote_session can make startup refresh return an empty listing.
                    fs.remote_session = None;
                    if fs.current_path != cwd {
                        fs.current_path = cwd.clone();
                        crate::event_helpers::push_history(fs, cwd);
                    }
                }
            }
        }

        // Merge favorites (Defaults + Loaded)
        let mut loaded_starred = state.starred;
        for def in app.starred {
            if !loaded_starred.contains(&def) {
                loaded_starred.push(def);
            }
        }
        app.starred = loaded_starred;

        // Load servers from servers.toml (new primary storage)
        // Migrate legacy remote_bookmarks from state.json if servers.toml doesn't exist
        crate::servers::maybe_migrate_legacy_bookmarks(&state.remote_bookmarks);
        app.servers = crate::servers::load_servers();

        app.path_colors = state.path_colors;
        app.external_tools = state.external_tools;
        if let Some(mode) = state.icon_mode {
            app.icon_mode = mode;
        }
        app.is_split_mode = state.is_split_mode;
        app.semantic_coloring = state.semantic_coloring;
        app.show_sidebar = state.show_sidebar;
        app.sidebar_folders = state.sidebar_folders;
        app.sidebar_favorites = state.sidebar_favorites;
        app.sidebar_recent = state.sidebar_recent;
        app.sidebar_storage = state.sidebar_storage;
        app.sidebar_remotes = state.sidebar_remotes;
        app.show_side_panel = state.show_side_panel;
        app.default_show_hidden = state.default_show_hidden;
        app.auto_save = state.auto_save;
        app.preview_max_mb = state.preview_max_mb.max(1);
        app.expanded_folders = state.expanded_folders.into_iter().collect();
        app.sidebar_width_percent = state.sidebar_width_percent;
        app.recent_folders = state.recent_folders;
        if let Some(theme_style) = state.theme_style {
            // Migration: users who had the previous default "Cool" should move to
            // the new default "Legacy Red", while preserving custom themes.
            if theme_style == crate::ui::theme::ThemeStyle::preset_cool() {
                crate::ui::theme::set_style_settings(
                    crate::ui::theme::ThemeStyle::preset_legacy_red(),
                );
            } else {
                crate::ui::theme::set_style_settings(theme_style);
            }
        }
    }

    // Prime visible tabs synchronously so startup never renders as empty while waiting
    // for async refresh/tick scheduling.
    prime_visible_tabs(&mut app);

    let app_arc = Arc::new(PLMutex::new(app));
    (app_arc, tx, rx)
}

fn handle_event(
    evt: Event,
    app: &mut App,
    event_tx: mpsc::Sender<AppEvent>,
    panes_needing_refresh: &mut std::collections::HashSet<usize>,
) -> bool {
    events::handle_event(evt, app, event_tx, panes_needing_refresh)
}

fn prime_visible_tabs(app: &mut App) {
    for pane in &mut app.panes {
        if let Some(fs) = pane.current_state_mut() {
            prime_local_file_state(fs);
        }
    }
}

fn prime_local_file_state(fs: &mut crate::state::FileState) {
    if fs.remote_session.is_some() {
        return;
    }

    let (files, mut metadata) = crate::modules::files::read_dir_with_metadata(&fs.current_path);
    let mut filtered_files: Vec<_> = files
        .into_iter()
        .filter(|p| {
            let is_hidden = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with('.'))
                .unwrap_or(false);
            fs.show_hidden || !is_hidden
        })
        .collect();

    filtered_files.sort_by(|a, b| {
        let meta_a = metadata.get(a);
        let meta_b = metadata.get(b);
        let is_dir_a = meta_a.map(|m| m.is_dir).unwrap_or(false);
        let is_dir_b = meta_b.map(|m| m.is_dir).unwrap_or(false);
        if is_dir_a != is_dir_b {
            return if is_dir_a {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }

        let ord = match fs.sort_column {
            crate::app::FileColumn::Name => {
                let na = a
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let nb = b
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                na.cmp(&nb)
            }
            crate::app::FileColumn::Size => {
                let sa = meta_a.map(|m| m.size).unwrap_or(0);
                let sb = meta_b.map(|m| m.size).unwrap_or(0);
                sa.cmp(&sb)
            }
            crate::app::FileColumn::Modified => {
                let da = meta_a
                    .map(|m| m.modified)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let db = meta_b
                    .map(|m| m.modified)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                da.cmp(&db)
            }
            crate::app::FileColumn::Created => {
                let da = meta_a
                    .map(|m| m.created)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let db = meta_b
                    .map(|m| m.created)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                da.cmp(&db)
            }
            crate::app::FileColumn::Permissions => {
                let pa = meta_a.map(|m| m.permissions).unwrap_or(0);
                let pb = meta_b.map(|m| m.permissions).unwrap_or(0);
                pa.cmp(&pb)
            }
        };
        if fs.sort_ascending {
            ord
        } else {
            ord.reverse()
        }
    });

    fs.local_count = filtered_files.len();
    fs.files = filtered_files;
    fs.metadata = std::mem::take(&mut metadata);
    if fs.selection.selected.is_none() && !fs.files.is_empty() {
        fs.selection.selected = Some(0);
        fs.table_state.select(Some(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dracon_terminal_engine::compositor::engine::TilePlacement;

    #[test]
    fn startup_prime_populates_first_pane_listing() {
        let tmp = std::env::temp_dir().join(format!("tiles-startup-prime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("example.txt"), "ok").unwrap();

        let queue: Arc<StdMutex<Vec<TilePlacement>>> = Arc::new(StdMutex::new(Vec::new()));
        let mut app = App::new(queue);
        if let Some(fs) = app.current_file_state_mut() {
            fs.current_path = tmp.clone();
            fs.files.clear();
            fs.metadata.clear();
            fs.selection.selected = None;
        }

        prime_visible_tabs(&mut app);

        let fs = app.current_file_state().unwrap();
        assert!(
            !fs.files.is_empty(),
            "startup should hydrate first pane file list"
        );
        assert!(fs
            .files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("example.txt")));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
