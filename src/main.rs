use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use dracon_terminal_engine::input::parser::Parser as TuiParser;
use dracon_terminal_engine::input::mapping::to_ui_event;
use dracon_terminal_engine::integration::ratatui::RatatuiBackend as EngineBackend;

// Ratatui Imports
use ratatui::Terminal;

use crate::app::{App, AppEvent, CurrentView, PreviewState};
use crate::config::{fuzzy_contains, FILE_WATCH_DEBOUNCE_MS, FUZZY_SEARCH, GIT_CACHE_TTL_SECONDS, MAX_TREE_DEPTH};
use image::GenericImageView;
mod tree_walk;
mod setup;
mod app;
mod config;
mod event;
mod event_helpers;
mod events;
mod handlers;
mod icons;
mod modules;
mod state;
mod ui;

struct TreeScanResult {
    tree_files: Vec<(PathBuf, u16)>,
    tree_metadata: std::collections::HashMap<PathBuf, crate::state::FileMetadata>,
    git_files: Vec<PathBuf>,
    git_metadata: std::collections::HashMap<PathBuf, crate::state::FileMetadata>,
}

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
                    let mut app_guard = ctx.app.lock();
                    crate::modules::system::SystemModule::update_app_state(&mut app_guard, data);
                    needs_draw = true;
                }
                AppEvent::ConnectToRemote(pane_idx, bookmark_idx) => {
                    let remote_opt = {
                        let app_guard = ctx.app.lock();
                        app_guard.remote.remote_bookmarks.get(bookmark_idx).cloned()
                    };
                    if let Some(remote) = remote_opt {
                        let tx = ctx.event_tx.clone();
                        let p_idx = pane_idx;
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::StatusMsg(format!(
                            "Connecting to {} ({})...",
                            remote.name, remote.host
                        )));

                        tokio::spawn(async move {
                            let connect_result = tokio::task::spawn_blocking(move || {
                                crate::modules::remote::connect_remote(&remote)
                            })
                            .await;

                            match connect_result {
                                Ok(Ok(session)) => {
                                    let _ =
                                        tx.send(AppEvent::RemoteConnected(p_idx, session)).await;
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
                AppEvent::RemoteConnected(pane_idx, session) => {
                    let mut app_guard = ctx.app.lock();
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                        if let Some(fs) = pane.current_state_mut() {
                            fs.nav.remote_session = Some(session);
                            fs.nav.current_path = PathBuf::from("/");
                            let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                        }
                    }
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
                    let tx = ctx.event_tx.clone();
                    let app_clone = app.clone();
                    let (current_dir, preview_limit_mb, remote_session) = {
                        let app_guard = ctx.app.lock();
                        if let Some(pane) = app_guard.panes.get(pane_idx) {
                            if let Some(fs) = pane.current_state() {
                                (
                                    fs.nav.current_path.clone(),
                                    app_guard.preview_max_mb.max(1),
                                    fs.nav.remote_session.clone(),
                                )
                            } else {
                                (PathBuf::from("."), app_guard.preview_max_mb.max(1), None)
                            }
                        } else {
                            (PathBuf::from("."), app_guard.preview_max_mb.max(1), None)
                        }
                    };

                    tokio::spawn(async move {
                        let path_str = path.to_string_lossy();
                        let content = if let Some(hash) = path_str.strip_prefix("git://") {
                            match crate::modules::files::show_commit_patch(&current_dir, hash) {
                                Ok(c) => c,
                                Err(e) => format!("Error fetching commit data: {}", e),
                            }
                        } else if let Some(file_path) = path_str.strip_prefix("git-diff://") {
                            if let Some(remote) = &remote_session {
                                match crate::modules::remote::show_file_diff(
                                    remote,
                                    &current_dir,
                                    file_path,
                                ) {
                                    Ok(content) => content,
                                    Err(e) => format!("Error fetching diff data: {}", e),
                                }
                            } else {
                                match crate::modules::files::show_file_diff(&current_dir, file_path)
                                {
                                    Ok(content) => content,
                                    Err(e) => format!("Error fetching diff data: {}", e),
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
                                Ok(false) => crate::modules::remote::read_to_string(remote, &path)
                                    .unwrap_or_else(|e| format!("Error reading remote file: {e}")),
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
                                std::fs::read_to_string(&path)
                                    .unwrap_or_else(|e| format!("Error reading file: {}", e))
                            }
                        };

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

                            let old_scroll = app_guard.editor_global.scroll_positions.get(&path).copied();

                            let fallback_scroll = app_guard.editor_global.editor_state.as_ref()
                                .and_then(|p| p.editor.as_ref())
                                .filter(|e| !e.modified)
                                .map(|e| (e.scroll_row, e.scroll_col, e.cursor_row, e.cursor_col))
                                .or_else(|| {
                                    app_guard.panes.get(pane_idx)
                                        .and_then(|p| p.current_state())
                                        .and_then(|fs| fs.view.preview.as_ref())
                                        .and_then(|p| p.editor.as_ref())
                                        .filter(|e| !e.modified)
                                        .map(|e| (e.scroll_row, e.scroll_col, e.cursor_row, e.cursor_col))
                                });

                            let (sr, sc, cr, cc) = old_scroll.or(fallback_scroll).unwrap_or((0, 0, 0, 0));
                            editor.scroll_row = sr;
                            editor.scroll_col = sc;
                            editor.cursor_row = cr;
                            editor.cursor_col = cc;

                            let preview = PreviewState {
                                path: path.clone(),
                                content,
                                editor: Some(editor),
                                last_saved: None,
                                highlighted_lines: None,
                            };

                             if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                                 if let Some(fs) = pane.current_state_mut() {
                                     fs.view.preview = Some(preview.clone());
                                 }
                             }
                            let is_git_url = path_str.starts_with("git://")
                                || path_str.starts_with("git-diff://");
                            if is_git_url
                                || app_guard.core.current_view == CurrentView::Editor
                                || app_guard.core.current_view == CurrentView::Commit
                            {
                                app_guard.editor_global.editor_state = Some(preview);
                                app_guard.sidebar.sidebar_focus = false;
                            }
                        }
                        let _ = tx.send(AppEvent::Tick).await;
                    });
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
                    let tx = ctx.event_tx.clone();
                    let app_clone = app.clone();
                    let src_name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
                    let task_id = uuid::Uuid::new_v4();

                    // Announce start
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::TaskProgress(task_id, 0.0, format!("Copying {}...", src_name)));

                    tokio::spawn(async move {
                        let remote = {
                            let app_guard = app_clone.lock();
                            app_guard.current_file_state()
                                .and_then(|fs| fs.nav.remote_session.clone())
                        };

                        let copied = if let Some(remote) = &remote {
                            crate::modules::remote::copy_recursive(remote, &src, &dest).is_ok()
                        } else {
                            dracon_terminal_engine::utils::copy_recursive(&src, &dest).is_ok()
                        };

                        if copied {
                            let mut app_guard = app_clone.lock();
                            app_guard.undo_state.undo_stack
                                .push(crate::app::UndoAction::Copy(src.clone(), dest.clone()));
                            app_guard.undo_state.redo_stack.clear();
                        }

                        // Finish task
                        let _ = tx.send(AppEvent::TaskFinished(task_id)).await;

                        let mut panes_to_refresh = std::collections::HashSet::new();
                        if let Some(parent) = dest.parent() {
                            let app_guard = app_clone.lock();
                            for (i, pane) in app_guard.panes.iter().enumerate() {
                                if let Some(fs) = pane.current_state() {
                                    if fs.nav.current_path == parent {
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
                    let remote_cmd = remote.as_ref().map(|r| {
                        crate::modules::remote::build_remote_terminal_command(
                            r,
                            &path,
                            command.as_deref(),
                        )
                    });
                    let cmd_str = remote_cmd.as_deref().or(command.as_deref());
                    crate::modules::terminal::spawn_terminal(&path, new_tab, cmd_str);
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
                    let mut app_guard = ctx.app.lock();
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
                            fs.git.git_history = history;
                            fs.git.git_pending = pending;
                            fs.git.git_branch = branch;
                            fs.git.git_ahead = ahead;
                            fs.git.git_behind = behind;
                            fs.git.git_summary = summary;
                            fs.git.git_remotes = remotes;
                            fs.git.git_stashes = stashes;
                            fs.git.git_cache_until = Some(Instant::now() + Duration::from_secs(GIT_CACHE_TTL_SECONDS));
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::TaskProgress(id, progress, status) => {
                    let mut app_guard = ctx.app.lock();
                    if let Some(task) = app_guard.output.background_tasks.iter_mut().find(|t| t.id == id) {
                        task.progress = progress;
                        task.status = status;
                    } else {
                        app_guard.output.background_tasks.push(crate::app::BackgroundTask {
                            id,
                            name: "Task".to_string(),
                            status,
                            progress,
                        });
                    }
                    needs_draw = true;
                }
                AppEvent::TaskFinished(id) => {
                    let mut app_guard = ctx.app.lock();
                    app_guard.output.background_tasks.retain(|t| t.id != id);
                    needs_draw = true;
                }
                AppEvent::GlobalSearchUpdated(pane_idx, files, _meta) => {
                    let mut app_guard = ctx.app.lock();
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                        if let Some(fs) = pane.current_state_mut() {
                            fs.list.files = files;
                        }
                    }
                    needs_draw = true;
                }
                AppEvent::SystemMonitor => {
                    let mut app_guard = ctx.app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.core.current_view = CurrentView::Processes;
                    needs_draw = true;
                }
                AppEvent::GitHistory => {
                    let mut app_guard = ctx.app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.core.current_view = CurrentView::Git;
                    let pane_idx = app_guard.focused_pane_index;
                    needs_draw = true;
                    drop(app_guard);
                    let _ = crate::app::try_send_event(&event_tx, AppEvent::RefreshFiles(pane_idx));
                }
                AppEvent::Editor => {
                    let mut app_guard = ctx.app.lock();
                    app_guard.save_current_view_prefs();
                    app_guard.core.current_view = CurrentView::Editor;
                    app_guard.load_view_prefs(CurrentView::Editor);
                    app_guard.apply_split_mode(false);
                    let pane_idx = app_guard.focused_pane_index;
                    let dir_path = app_guard.panes
                        .get(pane_idx)
                        .and_then(|p| p.current_state())
                        .map(|fs| fs.nav.current_path.clone());
                    needs_draw = true;
                    drop(app_guard);
                    if let Some(path) = dir_path {
                        let _ = crate::app::try_send_event(&event_tx, AppEvent::PreviewRequested(pane_idx, path));
                    }
                }
                AppEvent::StatusMsg(msg) => {
                    let mut app_guard = ctx.app.lock();
                    app_guard.output.last_action_msg = Some((msg, std::time::Instant::now()));
                    needs_draw = true;
                }
                AppEvent::AddToFavorites(path) => {
                    let mut app_guard = ctx.app.lock();
                    // Only add if path exists and not already in favorites
                    if path.exists() && !app_guard.nav.starred.contains(&path) {
                        app_guard.nav.starred.push(path.clone());
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
        for pane_idx in ctx.panes_needing_refresh.drain() {
            let (path, remote, current_filter, current_generation, git_view, tree_expanded, sort_column, sort_ascending, show_hidden) = {
                let app_guard = ctx.app.lock();
                if let Some(pane) = app_guard.panes.get(pane_idx) {
                    if let Some(fs) = pane.current_state() {
                        (
                            fs.nav.current_path.clone(),
                            fs.nav.remote_session.clone(),
                            fs.nav.search_filter.clone(),
                            fs.nav.search_generation,
                            matches!(app_guard.core.current_view, CurrentView::Files | CurrentView::Git | CurrentView::Commit),
                            app_guard.layout.expanded_folders.clone(),
                            fs.nav.sort_column,
                            fs.nav.sort_ascending,
                            fs.nav.show_hidden,
                        )
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            };

let list_path_for_filter = path.clone();

            let tx = ctx.event_tx.clone();
            let app_clone = app.clone();
            let expanded_folders = tree_expanded;
            tokio::spawn(async move {
                let list_path = path.clone();
                let list_remote = remote.clone();
                let list_filter = current_filter.clone();
                let start_generation = current_generation;
                let TreeScanResult { tree_files, tree_metadata: mut metadata, git_files: g_files, git_metadata: g_meta } =
                    tokio::task::spawn_blocking(move || {
                        let t_dir = std::time::Instant::now();
                        if let Some(session) = &list_remote {
                            let _ = crate::modules::remote::read_dir_with_metadata(session, &list_path);
                        } else {
                            let _ = crate::modules::files::read_dir_with_metadata(&list_path);
                        }

                        // Always walk expanded folders (Dolphin-style inline tree)
                        // Keep files and depths as pairs throughout the entire pipeline
                        // to avoid index misalignment after filtering/sorting
                        let max_depth = MAX_TREE_DEPTH;
                        let mut tree_files: Vec<(PathBuf, u16)> = Vec::new();
                        #[allow(clippy::too_many_arguments)]
                        tree_walk::walk_tree(&list_path, 0, max_depth, &expanded_folders, show_hidden, &mut tree_files, sort_column, sort_ascending);
                        // Collect metadata for all tree items
                        let tree_paths: Vec<PathBuf> = tree_files.iter().map(|(p, _)| p.clone()).collect();
                        let (files_meta, g_files, g_meta) = {
                            let meta = crate::modules::files::read_dir_recursive_meta(&tree_paths);
                            // meta = (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) — we only need the HashMap
                            let trimmed_filter = list_filter.trim();
                            let g_result = if trimmed_filter.len() > 3 {
                                if let Some(session) = &list_remote {
                                    crate::modules::remote::global_search(
                                        session,
                                        &list_path,
                                        trimmed_filter,
                                    )
                                } else {
                                    let search_root =
                                        dirs::home_dir().unwrap_or_else(|| list_path.clone());
                                    crate::modules::files::global_search(&search_root, trimmed_filter)
                                }
                            } else {
                                (Vec::new(), std::collections::HashMap::new())
                            };
                            (meta.1, g_result.0, g_result.1)
                        };

                        crate::app::log_debug(&format!("read_dir+search took {:?} for {:?}", t_dir.elapsed(), list_path));
TreeScanResult { tree_files, tree_metadata: files_meta, git_files: g_files, git_metadata: g_meta }
                    })
                    .await
                    .unwrap_or_else(|_| {
                        TreeScanResult {
                            tree_files: Vec::new(),
                            tree_metadata: std::collections::HashMap::new(),
                            git_files: Vec::new(),
                            git_metadata: std::collections::HashMap::new(),
                        }
                    });

                {
                    let t_apply = std::time::Instant::now();
                    let mut app_guard = app_clone.lock();
                    crate::app::log_debug(&format!("apply lock took {:?}", t_apply.elapsed()));
                    if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                            if let Some(fs) = pane.current_state_mut() {
                                // RACE CONDITION CHECK:
                                // If filter changed while we were reading, discard stale results
                                if fs.nav.search_generation != start_generation {
                                    crate::app::log_debug(&format!(
                                        "RefreshFiles: generation mismatch (pane={}), dropping stale results",
                                        pane_idx
                                    ));
                                    return;
                                }

                            // tree_files is Vec<(PathBuf, u16)> — keep pairs intact through filter/sort
                            let mut paired: Vec<(PathBuf, u16)> = tree_files.into_iter().filter(|(p, _)| {
                                let is_hidden = p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|s| s.starts_with('.'))
                                    .unwrap_or(false);

                                if !fs.nav.show_hidden && is_hidden {
                                    return false;
                                }

                                if !fs.nav.search_filter.is_empty() {
                                    let name = p
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    let matches = if FUZZY_SEARCH {
                                        fuzzy_contains(name, &fs.nav.search_filter)
                                    } else {
                                        name.to_lowercase().contains(&fs.nav.search_filter.to_lowercase())
                                    };
                                    if !matches {
                                        return false;
                                    }
                                }

                                true
                            }).collect();

                            // Search filter: include ancestor folders so matching children are visible
                            if !fs.nav.search_filter.is_empty() {
                                use std::collections::HashSet;
                                let filter_lower = fs.nav.search_filter.to_lowercase();
                                let mut keep: HashSet<PathBuf> = HashSet::new();
                                for (p, _) in &paired {
                                    let name = p.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("");
                                    let matches = if FUZZY_SEARCH {
                                        fuzzy_contains(name, &fs.nav.search_filter)
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

                            // Tree order from walk_tree is already sorted according to the user's
                            // sort_column/sort_ascending (applied within each directory level).
                            // Do NOT re-sort the flat list — it would scatter children away
                            // from parent folders and break the tree structure.

                            fs.list.local_count = paired.len();

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

                            fs.list.tree_file_depths = tree_file_depths;
                            let prev_selected_path = fs.list.selection.selected
                                .and_then(|idx| fs.list.files.get(idx).cloned());
                            fs.list.files = files;
                            fs.list.metadata = metadata;

                            // Re-find the previously selected file by path after file list update
                            if let Some(path) = prev_selected_path {
                                if let Some(new_idx) = fs.list.files.iter().position(|p| p == &path) {
                                    fs.list.selection.selected = Some(new_idx);
                                    fs.list.selection.anchor = Some(new_idx);
                                    fs.view.table_state.select(Some(new_idx));
                                    let capacity = fs.view.view_height.saturating_sub(3).max(1);
                                    let offset = fs.view.table_state.offset();
                                    if new_idx < offset {
                                        *fs.view.table_state.offset_mut() = new_idx;
                                    } else if new_idx >= offset + capacity {
                                        *fs.view.table_state.offset_mut() = new_idx.saturating_sub(capacity - 1);
                                    }
                                } else {
                                    let max_idx = fs.list.files.len().saturating_sub(1);
                                    fs.list.selection.selected = Some(max_idx);
                                    fs.view.table_state.select(Some(max_idx));
                                }
                            }
                            let max_offset = fs.list.files.len().saturating_sub(fs.view.view_height.saturating_sub(3).max(1));
                            if fs.view.table_state.offset() > max_offset {
                                *fs.view.table_state.offset_mut() = max_offset;
                            }

                            // Apply pending selection and scroll (e.g., after navigate_up)
                            if let Some((pending_path, pending_scroll)) = fs.view.pending_select_path.take() {
                                if let Some(idx) = fs.list.files.iter().position(|p| p == &pending_path)
                                {
                                    fs.list.selection.selected = Some(idx);
                                    fs.view.table_state.select(Some(idx));
                                    *fs.view.table_state.offset_mut() = pending_scroll;
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
                        app_guard.panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| {
                                fs.git.git_cache_until
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
                        app_guard.panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| fs.nav.current_path == git_path)
                            .unwrap_or(false)
                    };
                    if !path_still_active {
                        return;
                    }

                    // Get the active tab index for this pane so git data lands in the right place
                    let active_tab_idx = {
                        let app_guard = app_for_git.lock();
                        app_guard.panes
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
