//! Event loop context — shared mutable state for the main event loop.
//!
//! This struct bundles all the state that event handlers need access to,
//! replacing the scattered local variables in `run_tty()`. Once fully
//! extracted, each `AppEvent` match arm becomes a method on `EventLoopCtx`
//! or a call to a handler module function.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::app::{App, AppEvent};

/// File watcher debouncer type alias.
type FileDebouncer = notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>;

/// Self-save guard: tracks files we just wrote to prevent spurious reloads.
/// Maps path → (mtime, size, instant_of-save).
pub type SelfSaveMap = HashMap<PathBuf, (std::time::SystemTime, u64, Instant)>;

/// Shared mutable state for the main event loop.
///
/// Instead of 5+ loose local variables in `run_tty()`, handlers receive
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
}

impl EventLoopCtx {
    /// Lock the app for synchronous access.
    pub fn app(&self) -> parking_lot::MutexGuard<'_, App> {
        self.app.lock()
    }

    /// Send an event to the event loop (non-blocking, logs on failure).
    pub fn send_event(&self, event: AppEvent) {
        let _ = crate::app::try_send_event(&self.event_tx, event);
    }

    /// Prune expired self-save entries (called each tick).
    pub fn prune_self_save(&mut self) {
        self.last_self_save
            .retain(|_, (_, _, at)| at.elapsed() < Duration::from_secs(5));
    }

    /// Mark a pane for refresh at end of tick.
    pub fn mark_refresh(&mut self, pane_idx: usize) {
        self.panes_needing_refresh.insert(pane_idx);
    }

    /// Drain all panes needing refresh, returning them.
    pub fn drain_refreshes(&mut self) -> HashSet<usize> {
        std::mem::take(&mut self.panes_needing_refresh)
    }
}
