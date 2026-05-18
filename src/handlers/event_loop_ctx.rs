//! Event loop context — shared mutable state for the main event loop.
//!
//! This struct bundles all the state that event handlers need access to,
//! replacing the scattered local variables in `run_tty()`. Handlers are
//! gradually being extracted from the `match event { ... }` in main.rs
//! into methods on `EventLoopCtx`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::app::{App, AppEvent};

/// File watcher debouncer type alias.
pub(crate) type FileDebouncer = notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>;

/// Self-save guard: tracks files we just wrote to prevent spurious reloads.
/// Maps path → (mtime, size, instant-of-save).
pub type SelfSaveMap = HashMap<PathBuf, (std::time::SystemTime, u64, Instant)>;

/// Watch sync interval in milliseconds.
const WATCH_SYNC_INTERVAL_MS: u64 = 2000;

/// Shared mutable state for the main event loop.
///
/// Instead of 8+ loose local variables in `run_tty()`, handlers receive
/// `&mut EventLoopCtx` which bundles everything they need.
pub struct EventLoopCtx {
    /// The application state (behind Arc<Mutex> for async access).
    pub app: Arc<Mutex<App>>,
    /// Channel sender for dispatching new events.
    pub event_tx: tokio::sync::mpsc::Sender<AppEvent>,
    /// Set of pane indices that need file list refresh at end of tick.
    pub panes_needing_refresh: HashSet<usize>,
    /// Tracks files we just saved, to ignore inotify echo.
    pub last_self_save: SelfSaveMap,
    /// File watcher debouncer.
    pub debouncer: FileDebouncer,
    /// Paths currently being watched by the file watcher.
    pub watched_paths: HashSet<PathBuf>,
    /// Paths synced in the last watch sync (for fast bail-out).
    pub last_synced_paths: HashSet<PathBuf>,
    /// Instant of last watch sync.
    pub last_watch_sync: Instant,
}

impl EventLoopCtx {
    /// Create a new EventLoopCtx from the components assembled in run_tty().
    pub fn new(
        app: Arc<Mutex<App>>,
        event_tx: tokio::sync::mpsc::Sender<AppEvent>,
        debouncer: FileDebouncer,
        watched_paths: HashSet<PathBuf>,
        last_synced_paths: HashSet<PathBuf>,
    ) -> Self {
        Self {
            app,
            event_tx,
            panes_needing_refresh: HashSet::new(),
            last_self_save: HashMap::new(),
            debouncer,
            watched_paths,
            last_synced_paths,
            last_watch_sync: Instant::now(),
        }
    }

    /// Lock the app for synchronous access.
    #[allow(dead_code)]
    pub fn app_lock(&self) -> parking_lot::MutexGuard<'_, App> {
        self.app.lock()
    }

    /// Send an event to the event loop (non-blocking, logs on failure).
    #[allow(dead_code)]
    pub fn send_event(&self, event: AppEvent) {
        let _ = crate::app::try_send_event(&self.event_tx, event);
    }

    /// Prune expired self-save entries (called each tick).
    pub fn prune_self_save(&mut self) {
        self.last_self_save
            .retain(|_, (_, _, at)| at.elapsed() < Duration::from_secs(5));
    }

    /// Mark a pane for refresh at end of tick.
    #[allow(dead_code)]
    pub fn mark_refresh(&mut self, pane_idx: usize) {
        self.panes_needing_refresh.insert(pane_idx);
    }

    /// Drain all panes needing refresh, returning them.
    #[allow(dead_code)]
    pub fn drain_refreshes(&mut self) -> HashSet<usize> {
        std::mem::take(&mut self.panes_needing_refresh)
    }

    /// Synchronize file watches with current app state.
    /// Adds watches for new paths, removes watches for paths no longer in use.
    pub fn sync_watches(&mut self) {
        let mut current_paths = HashSet::new();

        // Collect all paths from panes
        {
            let app_guard = self.app.lock();
            for pane in &app_guard.panes {
                for tab in &pane.tabs {
                    current_paths.insert(tab.nav.current_path.clone());
                }
            }
            // Also watch expanded folders in sidebar for editor view
            for path in &app_guard.layout.expanded_folders {
                current_paths.insert(path.clone());
            }
        }

        // Fast bail-out: skip if nothing changed since last sync
        if current_paths == self.last_synced_paths {
            return;
        }
        self.last_synced_paths = current_paths.clone();

        // Add paths that aren't being watched yet
        for path in &current_paths {
            if !self.watched_paths.contains(path) {
                crate::app::log_debug(&format!("Starting file watch for: {:?}", path));
                if let Ok(()) = self
                    .debouncer
                    .watcher()
                    .watch(path, notify::RecursiveMode::NonRecursive)
                {
                    self.watched_paths.insert(path.clone());
                    crate::app::log_debug(&format!("Now watching: {:?}", path));
                } else {
                    crate::app::log_debug(&format!("Failed to watch: {:?}", path));
                }
            }
        }

        // Remove paths that are no longer current
        let to_remove: Vec<_> = self
            .watched_paths
            .iter()
            .filter(|p| !current_paths.contains(*p))
            .cloned()
            .collect();
        for path in to_remove {
            // Note: notify_debouncer_mini doesn't support unwatch
            self.watched_paths.remove(&path);
        }
    }

    /// Handle the Tick event: prune self-save, sync watches periodically.
    /// Returns true if a redraw is needed.
    pub fn handle_tick(&mut self) -> bool {
        self.prune_self_save();
        if self.last_watch_sync.elapsed() >= Duration::from_millis(WATCH_SYNC_INTERVAL_MS) {
            self.sync_watches();
            self.last_watch_sync = Instant::now();
        }
        true // Tick always needs a draw
    }

    /// Handle the RefreshFiles event: push recent folder + mark pane for refresh.
    /// Returns true if the pane was valid (needs a draw).
    pub fn handle_refresh_files(&mut self, pane_idx: usize) -> bool {
        let t_refresh = std::time::Instant::now();
        let current_path = {
            let t_lock = std::time::Instant::now();
            let mut app_guard = self.app.lock();
            crate::app::log_debug(&format!("RefreshFiles lock took {:?}", t_lock.elapsed()));
            let path = app_guard
                .panes
                .get(pane_idx)
                .and_then(|pane| pane.current_state())
                .map(|fs| fs.nav.current_path.clone());
            if let Some(ref p) = path {
                app_guard.push_recent_folder(p.clone());
            }
            path
        };
        crate::app::log_debug(&format!("RefreshFiles handler total {:?}", t_refresh.elapsed()));
        if current_path.is_some() {
            self.mark_refresh(pane_idx);
        }
        current_path.is_some()
    }

    /// Handle the FilesChangedOnDisk event: self-save guard, refresh affected panes,
    /// and trigger editor reloads for open preview files.
    /// Returns (needs_draw, should_continue) — continue skips this event entirely.
    pub fn handle_files_changed_on_disk(&mut self, path: PathBuf) -> (bool, bool) {
        crate::app::log_debug(&format!("FilesChangedOnDisk: {:?}", path));

        // Self-save guard: skip events for files we just wrote.
        if let Some((_saved_mtime, _saved_size, saved_at)) = self.last_self_save.get(&path) {
            let exact_match = std::fs::metadata(&path).ok().and_then(|meta| {
                meta.modified().ok().map(|mtime| {
                    let size: u64 = meta.len();
                    mtime == *_saved_mtime && size == *_saved_size
                })
            }).unwrap_or(false);

            if exact_match || saved_at.elapsed() < Duration::from_secs(5) {
                return (false, true); // continue (skip this event)
            }
        }

        let app_guard = self.app.lock();
        let mut needs_reload = Vec::new();

        for (i, pane) in app_guard.panes.iter().enumerate() {
            if let Some(fs) = pane.current_state() {
                let current_path = &fs.nav.current_path;
                let should_refresh = if let Some(parent) = path.parent() {
                    parent == current_path.as_path() || path.starts_with(current_path)
                } else {
                    path == current_path.as_path() || path.starts_with(current_path)
                };

                if should_refresh {
                    let is_self_save_dir = path.parent().map(|parent| {
                        self.last_self_save.keys().any(|sp| sp.parent() == Some(parent))
                    }).unwrap_or(false)
                        && self.last_self_save.values().any(|(_, _, at)| at.elapsed() < Duration::from_secs(2));

                    if !is_self_save_dir {
                        crate::app::log_debug(&format!("Refreshing pane {} for path {:?}", i, path));
                        self.panes_needing_refresh.insert(i);
                    }
                }
            }
            if let Some(fs) = pane.current_state() {
                if let Some(ref preview) = fs.view.preview {
                    if preview.path == path {
                        if let Some(editor) = &preview.editor {
                            if !editor.modified && !self.last_self_save.contains_key(&path) {
                                needs_reload.push((i, path.clone()));
                            }
                        }
                    }
                }
            }
        }

        if let Some(preview) = &app_guard.editor_global.editor_state {
            if preview.path == path {
                if let Some(editor) = &preview.editor {
                    if !editor.modified && !self.last_self_save.contains_key(&path) {
                        needs_reload.push((app_guard.focused_pane_index, path.clone()));
                    }
                }
            }
        }

        drop(app_guard);
        for (p_idx, p_path) in needs_reload {
            let _ = crate::app::try_send_event(&self.event_tx, AppEvent::PreviewRequested(p_idx, p_path));
        }
        (true, false) // needs_draw = true, don't continue
    }

    /// Handle the SaveFile event: write content to disk, update self-save guard,
    /// and update editor preview state.
    pub fn handle_save_file(&mut self, path: PathBuf, content: String) {
        let remote_for_save = {
            let app_guard = self.app.lock();
            app_guard.panes
                .iter()
                .find_map(|pane| {
                    let fs = pane.current_state()?;
                    let preview = fs.view.preview.as_ref()?;
                    if preview.path == path {
                        fs.nav.remote_session.clone()
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    let fs = app_guard.current_file_state()?;
                    app_guard.editor_global.editor_state.as_ref()?;
                    fs.nav.remote_session.clone()
                })
        };

        let save_res = if let Some(remote) = &remote_for_save {
            crate::modules::remote::write_string(remote, &path, &content)
        } else {
            std::fs::write(&path, &content)
        };

        match save_res {
            Ok(_) => {
                if remote_for_save.is_none() {
                    let now = Instant::now();
                    let meta_ok = std::fs::metadata(&path).ok().and_then(|meta| {
                        meta.modified().ok().map(|mtime| {
                            let size: u64 = meta.len();
                            if self.last_self_save.len() > 100 {
                                self.last_self_save.retain(|_, (_, _, at)| at.elapsed() < Duration::from_secs(5));
                            }
                            self.last_self_save.insert(path.clone(), (mtime, size, now));
                            true
                        })
                    });
                    if meta_ok.is_none() {
                        self.last_self_save.insert(path.clone(), (std::time::SystemTime::UNIX_EPOCH, 0, now));
                    }
                }
                let mut app_guard = self.app.lock();
                if let Some(ref mut preview) = app_guard.editor_global.editor_state {
                    if preview.path == path {
                        preview.last_saved = Some(Instant::now());
                        if let Some(ref mut editor) = preview.editor {
                            editor.modified = false;
                        }
                        preview.highlighted_lines = None;
                    }
                }
                for pane in &mut app_guard.panes {
                    if let Some(fs) = pane.current_state_mut() {
                        if let Some(ref mut preview) = fs.view.preview {
                            if preview.path == path {
                                preview.last_saved = Some(Instant::now());
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
                            if fs.nav.current_path == parent {
                                self.panes_needing_refresh.insert(i);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let mut app_guard = self.app.lock();
                let msg = format!("Failed to save file: {}", e);
                crate::app::log_debug(&msg);
                app_guard.output.last_action_msg = Some((msg, Instant::now()));
            }
        }
    }
}
