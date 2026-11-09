use ratatui::widgets::TableState;

use std::collections::{HashMap, HashSet};

use std::path::{Path, PathBuf};

use crate::config::{MAX_RECENT_FOLDERS, PREVIEW_MAX_MB};

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

    pub saved_pane: Option<Pane>,

    pub show_main_stage: bool,

}



impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {

    /// Create a new App instance with default state.
    ///
    /// Initializes with a single pane showing the current working directory,
    /// default sidebar settings, and the Files view active.
    pub fn new() -> Self {

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
                application_table_state: TableState::default(),

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

                open_with_scroll: 0,

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
    /// Delegates to `apply_split_mode` so toggling preserves the second
    /// pane's tabs/nav state (via `saved_pane`) instead of truncating it.
    pub fn toggle_split(&mut self) {
        let split = !self.core.is_split_mode;
        self.apply_split_mode(split);
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

            } else if let Some(initial_fs) = self.panes.first().and_then(|p| p.tabs.first().cloned()) {

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

                    // Check if NEW selection is within viewport before scrolling

                    let capacity = fs.view.view_height.saturating_sub(3);

                    let current_offset = fs.view.table_state.offset();

                    

                    // Calculate screen row (0-indexed) for the new selection

                    // Screen row = selection index - offset (clamped to 0)

                    let new_sel = fs.list.selection.selected.unwrap_or(0);

                    let new_screen_row = new_sel.saturating_sub(current_offset);

                    

                    // Only scroll if the new selection would go out of viewport

                    if new_sel + capacity < current_offset {

                        // Selection is more than one full page above the viewport, jump to show it

                        *fs.view.table_state.offset_mut() = new_sel;

                    } else if new_screen_row == 0 {

                        // New selection is at the top, scroll up to keep it visible

                        if current_offset > 0 {

                            *fs.view.table_state.offset_mut() = current_offset.saturating_sub(1);

                        }

                    } else if new_screen_row >= capacity {

                        // New selection is out of viewport (bottom), scroll down to keep it visible

                        let new_offset = new_sel.saturating_sub(capacity.saturating_sub(1));

                        *fs.view.table_state.offset_mut() = new_offset;

                    }

                } else if fs.view.table_state.offset() > 0 {
                    // At first item but offset > 0: scroll up past the first item
                    let current_offset = fs.view.table_state.offset();
                    let new_offset = current_offset.saturating_sub(1);
                    crate::app::log_debug(&format!("move_up EDGE: offset {} → {}", current_offset, new_offset));
                    *fs.view.table_state.offset_mut() = new_offset;
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

                    

                    // Check if NEW selection is within viewport before scrolling

                    let current_offset = fs.view.table_state.offset();

                    

                    // Calculate screen row (0-indexed) for the new selection

                    // Screen row = selection index - offset (clamped to 0)

                    let new_sel = fs.list.selection.selected.unwrap_or(0);

                    let new_screen_row = new_sel.saturating_sub(current_offset);

                    

                    // Only scroll if the new selection would go out of viewport

                    if new_screen_row >= capacity {

                        // Selection is out of viewport (bottom), scroll to keep it visible

                        let new_offset = new_sel.saturating_sub(capacity.saturating_sub(1));

                        *fs.view.table_state.offset_mut() = new_offset;

                    }

                } else {
                    // At last item: scroll down past the last item
                    let current_offset = fs.view.table_state.offset();
                    let new_offset = current_offset.saturating_add(1);
                    crate::app::log_debug(&format!("move_down EDGE: offset {} → {}", current_offset, new_offset));
                    *fs.view.table_state.offset_mut() = new_offset;
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

/// Rotated generations kept besides the active log.
const ROTATED_LOG_GENERATIONS: usize = 2;

/// Directory holding the debug log: `$XDG_DATA_HOME/tiles`, or the working
/// directory when the platform exposes no data dir.
fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tiles")
}

fn rotated_log_path(dir: &Path, generation: usize) -> PathBuf {
    dir.join(format!("debug.log.{generation}"))
}

/// Rotate the active log one generation down, then open a fresh active log.
///
/// The oldest generation is removed BEFORE the shift, so the file that was
/// just rotated is never the one deleted. Rotating used to rename the active
/// log to `.1`, shift `.1` to `.2`, then delete `.2` — throwing away the whole
/// log it had just preserved.
fn rotate_active_log(dir: &Path) -> std::io::Result<std::io::BufWriter<std::fs::File>> {
    std::fs::create_dir_all(dir)?;
    // Oldest first: only a generation that is already expired is removed.
    let _ = std::fs::remove_file(rotated_log_path(dir, ROTATED_LOG_GENERATIONS));
    for generation in (1..ROTATED_LOG_GENERATIONS).rev() {
        let from = rotated_log_path(dir, generation);
        let to = rotated_log_path(dir, generation + 1);
        if from.exists() {
            std::fs::rename(&from, &to)?;
        }
    }
    let active = dir.join("debug.log");
    if active.exists() {
        std::fs::rename(&active, rotated_log_path(dir, 1))?;
    }
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&active)
        .map(std::io::BufWriter::new)
}

/// Open the active log, rotating it first when it has passed the size cap.
fn open_active_log(dir: &Path) -> Option<std::io::BufWriter<std::fs::File>> {
    let active = dir.join("debug.log");
    let oversized = std::fs::metadata(&active)
        .map(|meta| meta.len() > MAX_LOG_SIZE_BYTES)
        .unwrap_or(false);
    if oversized {
        if let Ok(writer) = rotate_active_log(dir) {
            return Some(writer);
        }
        // Rotation failed (read-only dir, permissions, ...): appending to the
        // oversized log still beats dropping the message.
    }
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&active)
        .ok()
        .map(std::io::BufWriter::new)
}

fn write_log_line(writer: &mut Option<std::io::BufWriter<std::fs::File>>, msg: &str) {
    use std::io::Write;
    if let Some(w) = writer {
        let _ = writeln!(w, "[{}] DEBUG: {}", chrono::Utc::now(), msg);
        let _ = w.flush();
    }
}

/// Log a debug message to the XDG data directory log file.
///
/// Messages are written to `$XDG_DATA_HOME/tiles/debug.log` with ISO timestamps.
/// No-op if debug logging is disabled.
pub fn log_debug(msg: &str) {
    if !debug_logging_enabled() {
        return;
    }

    static LOG_FILE: std::sync::LazyLock<
        parking_lot::Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    > = std::sync::LazyLock::new(|| parking_lot::Mutex::new(open_active_log(&log_dir())));

    let mut guard = LOG_FILE.lock();

    // Rotate during the session too (the lazy init only checks at startup),
    // throttled to at most one stat per second.
    static LAST_ROTATE_CHECK: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);
    let now_secs = chrono::Utc::now().timestamp().max(0) as u64;
    let last_check = LAST_ROTATE_CHECK.swap(now_secs, std::sync::atomic::Ordering::Relaxed);
    if now_secs.saturating_sub(last_check) >= 1 {
        let dir = log_dir();
        let oversized = std::fs::metadata(dir.join("debug.log"))
            .map(|meta| meta.len() > MAX_LOG_SIZE_BYTES)
            .unwrap_or(false);
        if oversized {
            // Close the writer before renaming: a live handle would keep
            // appending into the rotated generation (and Windows refuses to
            // rename an open file).
            guard.take();
            *guard = open_active_log(&dir);
            write_log_line(&mut guard, msg);
            return;
        }
    }

    write_log_line(&mut guard, msg);
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

    if tx.is_closed() {

        return false;

    }

    match tx.try_send(evt) {

        Ok(()) => true,

        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => false,

        Err(tokio::sync::mpsc::error::TrySendError::Full(evt)) => {

            // Bounded retry: the consumer loop drains every ~33ms, so a
            // brief burst should clear. Never silently drop a user
            // operation (save/refresh/status) under load.
            let mut evt = evt;

            for _ in 0..20 {

                std::thread::sleep(std::time::Duration::from_millis(5));

                match tx.try_send(evt) {

                    Ok(()) => return true,

                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => return false,

                    Err(tokio::sync::mpsc::error::TrySendError::Full(returned)) => {

                        evt = returned;

                    }

                }

            }

            log_debug("Channel send failed after retries — event dropped");

            false

        }

    }

}

#[cfg(test)]
mod tests {

    use super::*;



    fn test_app() -> App {

        App::new()

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

    fn log_fixture(tag: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tiles-logrot-{tag}-{unique}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create fixture dir");
        dir
    }

    fn read_or_empty(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    }

    /// The rotation used to rename the active log to `.1`, shift `.1` to `.2`,
    /// then delete `.2` — deleting the very file it had just preserved, so a
    /// rotation lost the whole log. The log that is being rotated out must
    /// survive as `.1`.
    #[test]
    fn rotate_active_log_keeps_the_log_it_just_rotated() {
        let dir = log_fixture("keep");
        std::fs::write(dir.join("debug.log"), "ACTIVE-CONTENT").expect("seed active");
        std::fs::write(dir.join("debug.log.1"), "GEN1-CONTENT").expect("seed gen1");
        std::fs::write(dir.join("debug.log.2"), "GEN2-EXPIRED").expect("seed gen2");

        let mut writer = rotate_active_log(&dir).expect("rotate");
        drop(&mut writer);

        assert_eq!(
            read_or_empty(&dir.join("debug.log.1")),
            "ACTIVE-CONTENT",
            "the log being rotated out must become generation 1, not be deleted"
        );
        assert_eq!(
            read_or_empty(&dir.join("debug.log.2")),
            "GEN1-CONTENT",
            "generation 1 must shift to generation 2"
        );
        assert_eq!(
            read_or_empty(&dir.join("debug.log")),
            "",
            "rotation must start a fresh active log"
        );
        assert!(
            dir.join("debug.log").exists(),
            "the fresh active log must exist"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Rotation is bounded: repeated rotations keep exactly the configured
    /// generations, never growing an unbounded `.N` chain.
    #[test]
    fn rotate_active_log_stays_bounded_across_repeated_rotations() {
        let dir = log_fixture("bounded");
        for round in 0..4 {
            std::fs::write(dir.join("debug.log"), format!("ROUND-{round}")).expect("seed active");
            rotate_active_log(&dir).expect("rotate");
        }

        let generations: Vec<String> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            !generations.iter().any(|n| n == "debug.log.3"),
            "no generation beyond the configured count: {generations:?}"
        );
        assert!(
            generations.iter().any(|n| n == "debug.log.1"),
            "at least generation 1 survives: {generations:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A small active log opens for appending without rotating anything.
    #[test]
    fn open_active_log_does_not_rotate_a_small_log() {
        let dir = log_fixture("small");
        std::fs::write(dir.join("debug.log"), "small").expect("seed active");

        let _writer = open_active_log(&dir).expect("open");

        assert_eq!(read_or_empty(&dir.join("debug.log")), "small", "content preserved");
        assert!(
            !dir.join("debug.log.1").exists(),
            "a log below the size cap must not be rotated"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An oversized active log is rotated on open, and the oversized content
    /// is preserved as generation 1 rather than dropped.
    #[test]
    fn open_active_log_rotates_an_oversized_log_and_preserves_it() {
        let dir = log_fixture("oversized");
        let big = vec![b'x'; (MAX_LOG_SIZE_BYTES as usize) + 64];
        std::fs::write(dir.join("debug.log"), &big).expect("seed oversized");

        let mut writer = open_active_log(&dir).expect("open");
        let mut slot = Some(writer);
        write_log_line(&mut slot, "after rotation");
        drop(slot);

        let rotated = std::fs::metadata(dir.join("debug.log.1")).expect("rotated generation exists");
        assert_eq!(
            rotated.len(),
            big.len() as u64,
            "the oversized log must survive rotation intact as generation 1"
        );

        let active = read_or_empty(&dir.join("debug.log"));
        assert!(
            active.contains("after rotation"),
            "the new line must land in the fresh log, got {active:?}"
        );
        assert!(
            !active.contains("xxxx"),
            "the rotated content must not leak into the fresh log"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

}
