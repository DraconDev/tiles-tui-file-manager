use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use crate::config::{MAX_RECENT_FOLDERS, PREVIEW_MAX_MB};
use dracon_terminal_engine::compositor::engine::TilePlacement;
use dracon_terminal_engine::widgets::TextInput;

pub use crate::state::{
    AppEvent, AppMode, ClipboardOp, CommandAction, CommandItem, CommitInfo, ContextMenuAction,
    ContextMenuTarget, CurrentView, DropTarget, FileCategory, FileColumn, FileMetadata, FileState,
    GitPendingChange, MonitorSubview, Pane, PreviewState, ProcessColumn,
    RemoteBookmark, SettingsSection, SettingsTarget, SidebarBounds, SidebarTarget,
    SystemState, UndoAction, ViewPreferences, ViewStatePersistence,
};

// Re-export sub-structs
pub use crate::state::app_subtypes::{
    AppCore, SidebarState, MonitorState, EditorGlobalState, UndoState, SettingsState,
    LayoutState, OutputState, DragState, NavState, RemoteState, MouseState, SelectionState2,
};

// Re-export BackgroundTask from state so we don't duplicate it in app.rs
pub use crate::state::BackgroundTask;

pub struct App {
    // --- Sub-structs ---
    pub core: AppCore,
    pub sidebar: SidebarState,
    pub monitor: MonitorState,
    pub editor_global: EditorGlobalState,
    pub undo_state: UndoState,
    pub settings: SettingsState,
    pub layout: LayoutState,
    pub output: OutputState,
    pub drag: DragState,
    pub nav: NavState,
    pub remote: RemoteState,
    pub mouse: MouseState,
    pub selection: SelectionState2,

    // --- Remaining fields ---
    pub panes: Vec<Pane>,
    pub focused_pane_index: usize,
    pub system_state: SystemState,
    pub preview_max_mb: u16,
    #[allow(dead_code)]
    pub tile_queue: Arc<StdMutex<Vec<TilePlacement>>>,
    pub saved_pane: Option<Pane>,
    pub show_main_stage: bool,
}

impl App {
    /// Create a new App instance with default state.
    ///
    /// Initializes with a single pane showing the current working directory,
    /// default sidebar settings, and the Files view active.
    pub fn new(tile_queue: Arc<StdMutex<Vec<TilePlacement>>>) -> Self {
        let start_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let initial_fs = FileState::new(
            start_path,
            None,
            false,
            vec![
                FileColumn::Name,
                FileColumn::Size,
                FileColumn::Modified,
                FileColumn::Permissions,
            ],
            FileColumn::Name,
            true,
        );

        Self {
            core: AppCore {
                running: true,
                current_view: CurrentView::Files,
                mode: AppMode::Normal,
                previous_mode: AppMode::Normal,
                input: TextInput::default(),
                icon_mode: crate::icons::IconMode::Nerd,
                is_split_mode: false,
                terminal_size: (80, 24),
                mouse_pos: (0, 0),
            },
            sidebar: SidebarState {
                show_sidebar: true,
                sidebar_focus: false,
                sidebar_index: 0,
                sidebar_folders: true,
                sidebar_favorites: true,
                sidebar_recent: true,
                sidebar_storage: true,
                sidebar_remotes: true,
                show_side_panel: true,
                sidebar_width_percent: 15,
                sidebar_bounds: Vec::new(),
                sidebar_scroll_offset: 0,
                tree_expanded_folders: HashSet::new(),
                sidebar_tree_cache: None,
                sidebar_tree_cache_key: 0,
                editor_sidebar_cache: None,
                editor_sidebar_cache_key: 0,
            },
            monitor: MonitorState {
                monitor_subview: MonitorSubview::Overview,
                monitor_subview_bounds: Vec::new(),
                overview_scroll_offset: 0,
                process_sort_col: ProcessColumn::Cpu,
                process_sort_asc: false,
                process_column_bounds: Vec::new(),
                process_selected_idx: None,
                process_table_state: TableState::default(),
                process_search_filter: String::new(),
                process_tree_view: false,
            },
            editor_global: EditorGlobalState {
                editor_state: None,
                scroll_positions: HashMap::new(),
                replace_buffer: String::new(),
                editor_clipboard: None,
            },
            undo_state: UndoState::default(),
            settings: SettingsState {
                settings_index: 0,
                settings_section: SettingsSection::General,
                settings_target: SettingsTarget::SingleMode,
                settings_scroll: 0,
                open_with_index: 0,
                confirm_delete: true,
                smart_date: true,
                semantic_coloring: true,
                auto_save: true,
                default_show_hidden: false,
            },
            layout: LayoutState {
                single_columns: vec![
                    FileColumn::Name,
                    FileColumn::Size,
                    FileColumn::Modified,
                    FileColumn::Permissions,
                ],
                split_columns: vec![FileColumn::Name, FileColumn::Size],
                header_icon_bounds: Vec::new(),
                tab_bounds: Vec::new(),
                hovered_header_icon: None,
                expanded_folders: HashSet::new(),
                pane_rects: Vec::new(),
            },
            output: OutputState::default(),
            drag: DragState::default(),
            nav: NavState {
                starred: [
                    dirs::home_dir(),
                    dirs::desktop_dir(),
                    dirs::document_dir(),
                    dirs::download_dir(),
                    dirs::audio_dir(),
                    dirs::picture_dir(),
                    dirs::video_dir(),
                    dirs::public_dir(),
                ]
                .into_iter()
                .flatten()
                .collect(),
                recent_folders: Vec::new(),
                command_index: 0,
                filtered_commands: Vec::new(),
                view_prefs: ViewStatePersistence {
                    files: ViewPreferences {
                        show_sidebar: true,
                        is_split_mode: false,
                    },
                    editor: ViewPreferences {
                        show_sidebar: true,
                        is_split_mode: false,
                    },
                },
                closed_tabs: std::collections::VecDeque::new(),
            },
            remote: RemoteState {
                remote_bookmarks: Vec::new(),
                pending_remote: RemoteBookmark {
                    name: String::new(),
                    host: String::new(),
                    user: String::new(),
                    port: 22,
                    last_path: PathBuf::from("/"),
                    key_path: None,
                },
                external_tools: HashMap::new(),
            },
            mouse: MouseState {
                mouse_last_click: std::time::Instant::now(),
                mouse_click_pos: (0, 0),
                mouse_click_count: 0,
                is_resizing_sidebar: false,
            },
            selection: SelectionState2 {
                selection_mode: false,
                prevent_mouse_up_selection_cleanup: false,
                rename_selected: false,
                clipboard: None,
                path_colors: HashMap::new(),
                folder_selections: HashMap::new(),
            },
            panes: vec![Pane::new(initial_fs)],
            focused_pane_index: 0,
            system_state: SystemState::default(),
            preview_max_mb: PREVIEW_MAX_MB,
            tile_queue,
            saved_pane: None,
            show_main_stage: true,
        }
    }

    /// Set the input shield cooldown to ignore keyboard events for `duration_ms` milliseconds.
    /// Used after mode transitions to prevent stray keypresses.
    pub fn set_input_shield(&mut self, duration_ms: u64) {
        let now = std::time::Instant::now();
        self.output.input_shield_until = Some(now + std::time::Duration::from_millis(duration_ms));
        self.output.input_shield_active_until =
            Some(now + std::time::Duration::from_millis(duration_ms + 100));
    }

    /// Check if we're currently in the soft input shield cooldown period.
    pub fn in_soft_shield(&self) -> bool {
        self.output
            .input_shield_active_until
            .map(|until| std::time::Instant::now() < until)
            .unwrap_or(false)
    }

    /// Add a folder to the recent folders list, trimming to MAX_HISTORY.
    pub fn push_recent_folder(&mut self, path: PathBuf) {
        self.nav.recent_folders.retain(|p| p != &path);
        self.nav.recent_folders.insert(0, path);
        if self.nav.recent_folders.len() > MAX_RECENT_FOLDERS {
            self.nav.recent_folders.truncate(MAX_RECENT_FOLDERS);
        }
    }

    /// Get a reference to the currently focused pane's FileState.
    pub fn current_file_state(&self) -> Option<&FileState> {
        self.panes
            .get(self.focused_pane_index)
            .and_then(|p| p.current_state())
    }

    /// Get a mutable reference to the currently focused pane's FileState.
    pub fn current_file_state_mut(&mut self) -> Option<&mut FileState> {
        self.panes
            .get_mut(self.focused_pane_index)
            .and_then(|p| p.current_state_mut())
    }

    /// Calculate the sidebar width in character columns based on the percentage setting.
    pub fn sidebar_width(&self) -> u16 {
        if !self.sidebar.show_sidebar {
            return 0;
        }
        (self.core.terminal_size.0 as f32 * (self.sidebar.sidebar_width_percent as f32 / 100.0)) as u16
    }

    /// Toggle between single-pane and dual-pane (split) mode.
    pub fn toggle_split(&mut self) {
        self.core.is_split_mode = !self.core.is_split_mode;
        if self.core.is_split_mode && self.panes.len() == 1 {
            let initial_fs = self.panes[0].tabs[0].clone();
            self.panes.push(Pane::new(initial_fs));
        } else if !self.core.is_split_mode && self.panes.len() > 1 {
            self.panes.truncate(1);
            self.focused_pane_index = 0;
        }
    }

    pub fn save_current_view_prefs(&mut self) {
        match self.core.current_view {
            CurrentView::Files => {
                self.nav.view_prefs.files.show_sidebar = self.sidebar.show_sidebar;
                self.nav.view_prefs.files.is_split_mode = self.core.is_split_mode;
            }
            CurrentView::Editor => {
                self.nav.view_prefs.editor.show_sidebar = self.sidebar.show_sidebar;
                self.nav.view_prefs.editor.is_split_mode = self.core.is_split_mode;
            }
            _ => {}
        }
    }

    pub fn load_view_prefs(&mut self, view: CurrentView) {
        let prefs = match view {
            CurrentView::Files => &self.nav.view_prefs.files,
            CurrentView::Editor => &self.nav.view_prefs.editor,
            _ => return,
        };
        self.sidebar.show_sidebar = prefs.show_sidebar;
        self.apply_split_mode(prefs.is_split_mode);
    }

    pub fn apply_split_mode(&mut self, split: bool) {
        if split && self.panes.len() == 1 {
            if let Some(saved) = self.saved_pane.take() {
                self.panes.push(saved);
            } else {
                let initial_fs = self.panes[0].tabs[0].clone();
                self.panes.push(Pane::new(initial_fs));
            }
        } else if !split && self.panes.len() > 1 {
            self.saved_pane = self.panes.pop();
            self.focused_pane_index = 0;
        }
        self.core.is_split_mode = split;
    }

    pub fn toggle_hidden(&mut self) -> usize {
        if let Some(fs) = self.current_file_state_mut() {
            fs.nav.show_hidden = !fs.nav.show_hidden;
        }
        self.focused_pane_index
    }

    pub fn move_to_other_pane(&mut self) {
        if self.panes.len() > 1 {
            self.focused_pane_index = if self.focused_pane_index == 0 { 1 } else { 0 };
        }
    }

    pub fn resize_sidebar(&mut self, delta: i16) {
        let mut val = self.sidebar.sidebar_width_percent as i16 + delta;
        val = val.clamp(5, 50);
        self.sidebar.sidebar_width_percent = val as u16;
    }

    pub fn move_up(&mut self, shift: bool) {
        if let Some(fs) = self.current_file_state_mut() {
            if let Some(sel) = fs.list.selection.selected {
                if sel > 0 {
                    let mut next = sel - 1;
                    while next > 0
                        && fs
                            .list.files
                            .get(next)
                            .map(|p| p.to_string_lossy() == "__DIVIDER__")
                            .unwrap_or(false)
                    {
                        next -= 1;
                    }
                    if fs
                        .list.files
                        .get(next)
                        .map(|p| p.to_string_lossy() == "__DIVIDER__")
                        .unwrap_or(false)
                    {
                        return;
                    }
                    fs.list.selection.handle_move(next, shift);
                    fs.view.table_state.select(fs.list.selection.selected);
                    if next < fs.view.table_state.offset() {
                        *fs.view.table_state.offset_mut() = next;
                    }
                }
            }
        }
    }

    pub fn move_down(&mut self, shift: bool) {
        if let Some(fs) = self.current_file_state_mut() {
            let capacity = fs.view.view_height.saturating_sub(3);
            if let Some(sel) = fs.list.selection.selected {
                if sel + 1 < fs.list.files.len() {
                    let mut next = sel + 1;
                    while next < fs.list.files.len() - 1
                        && fs
                            .list.files
                            .get(next)
                            .map(|p| p.to_string_lossy() == "__DIVIDER__")
                            .unwrap_or(false)
                    {
                        next += 1;
                    }
                    if fs
                        .list.files
                        .get(next)
                        .map(|p| p.to_string_lossy() == "__DIVIDER__")
                        .unwrap_or(false)
                    {
                        return;
                    }
                    fs.list.selection.handle_move(next, shift);
                    fs.view.table_state.select(fs.list.selection.selected);
                    if next >= fs.view.table_state.offset() + capacity {
                        let keep_visible = capacity.saturating_sub(1);
                        *fs.view.table_state.offset_mut() = next.saturating_sub(keep_visible);
                    }
                }
            }
        }
    }

    pub fn sidebar_move_up(&mut self) {
        if self.sidebar.sidebar_index > 0 {
            self.sidebar.sidebar_index -= 1;
        }
    }

    pub fn sidebar_move_down(&mut self, max: usize) {
        if self.sidebar.sidebar_index < max.saturating_sub(1) {
            self.sidebar.sidebar_index += 1;
        }
    }

    pub fn apply_process_sort(&mut self) {
        let col = self.monitor.process_sort_col;
        let asc = self.monitor.process_sort_asc;
        self.system_state.processes.sort_by(|a, b| {
            let ord = match col {
                ProcessColumn::Pid => a.pid.cmp(&b.pid),
                ProcessColumn::Name => a.name.cmp(&b.name),
                ProcessColumn::Cpu => a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal),
                ProcessColumn::Mem => a.mem.partial_cmp(&b.mem).unwrap_or(std::cmp::Ordering::Equal),
                ProcessColumn::User => a.user.cmp(&b.user),
                ProcessColumn::Status => a.status.cmp(&b.status),
            };
            if asc { ord } else { ord.reverse() }
        });
    }
}

const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024;

/// Log a debug message to the XDG data directory log file.
///
/// Messages are written to `$XDG_DATA_HOME/tiles/debug.log` with ISO timestamps.
/// No-op if debug logging is disabled.
pub fn log_debug(msg: &str) {
    if !debug_logging_enabled() {
        return;
    }

    use std::io::Write;
    static LOG_FILE: std::sync::LazyLock<
        parking_lot::Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    > = std::sync::LazyLock::new(|| {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let log_dir = log_dir.join("tiles");
        let _ = std::fs::create_dir_all(&log_dir);
        let path = log_dir.join("debug.log");
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > MAX_LOG_SIZE_BYTES {
                let _ = std::fs::rename(&path, log_dir.join("debug.log.1"));
                let _ = std::fs::remove_file(log_dir.join("debug.log.2"));
                let _ = std::fs::rename(log_dir.join("debug.log.1"), log_dir.join("debug.log.2"));
            }
        }
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .ok();
        parking_lot::Mutex::new(file.map(std::io::BufWriter::new))
    });

    let mut guard = LOG_FILE.lock();
    if let Some(ref mut w) = *guard {
        let _ = writeln!(w, "[{}] DEBUG: {}", chrono::Utc::now(), msg);
        let _ = w.flush();
    }
}

/// Check if debug logging is enabled via the TILES_DEBUG environment variable.
pub fn debug_logging_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("TILES_DEBUG_LOG")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false)
    })
}

use tokio::sync::mpsc::Sender;

#[allow(clippy::needless_borrow, clippy::collapsible_match, clippy::manual_checked_ops)]
/// Attempt to send an event through the channel without blocking.
///
/// Returns `true` if the event was sent successfully, `false` if the channel
/// is full (event is dropped and a log message is written).
#[must_use = "try_send_event returns false if the channel is full"]
pub fn try_send_event(tx: &Sender<AppEvent>, evt: AppEvent) -> bool {
    if tx.try_send(evt).is_err() {
        log_debug("Channel send failed — event dropped");
        false
    } else {
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use dracon_terminal_engine::compositor::engine::TilePlacement;
    use std::sync::{Arc, Mutex};

    fn test_app() -> App {
        let queue: Arc<Mutex<Vec<TilePlacement>>> = Arc::new(Mutex::new(Vec::new()));
        App::new(queue)
    }

    #[test]
    fn app_new_defaults() {
        let app = test_app();
        assert!(app.core.running);
        assert_eq!(app.core.current_view, CurrentView::Files);
        assert_eq!(app.core.mode, AppMode::Normal);
        assert_eq!(app.core.terminal_size, (80, 24));
        assert!(!app.core.is_split_mode);
    }

    #[test]
    fn app_new_has_single_pane() {
        let app = test_app();
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.focused_pane_index, 0);
    }

    #[test]
    fn app_new_has_file_state() {
        let app = test_app();
        let fs = app.current_file_state();
        assert!(fs.is_some());
        let fs = fs.unwrap();
        assert!(!fs.nav.current_path.as_os_str().is_empty());
        assert!(fs.list.files.is_empty()); // not yet populated
    }

    #[test]
    fn toggle_split_mode() {
        let mut app = test_app();
        assert!(!app.core.is_split_mode);
        assert_eq!(app.panes.len(), 1);
        app.toggle_split();
        assert!(app.core.is_split_mode);
        assert_eq!(app.panes.len(), 2);
        app.toggle_split();
        assert!(!app.core.is_split_mode);
        assert_eq!(app.panes.len(), 1);
    }

    #[test]
    fn sidebar_defaults() {
        let app = test_app();
        assert!(app.sidebar.show_sidebar);
        assert!(!app.sidebar.sidebar_focus);
        assert_eq!(app.sidebar.sidebar_width_percent, 15);
    }

    #[test]
    fn input_shield_mechanism() {
        let mut app = test_app();
        assert!(app.output.input_shield_until.is_none());
        app.set_input_shield(100);
        assert!(app.output.input_shield_until.is_some());
    }
}
