use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use crate::config::{MAX_RECENT_FOLDERS, PREVIEW_MAX_MB};
use dracon_terminal_engine::compositor::engine::TilePlacement;
use dracon_terminal_engine::widgets::TextInput;

pub use crate::state::{
    AppEvent, AppMode, ClipboardOp, CommandAction, CommandItem, CommitInfo, ContextMenuAction,
    ContextMenuTarget, CurrentView, DropTarget, FileCategory, FileColumn, FileMetadata, FileState,
    GitPendingChange, MonitorSubview, Pane, PreviewState, ProcessColumn, RemoteSession,
    SettingsSection, SettingsTarget, SidebarBounds, SidebarTarget,
    SystemState, UndoAction, ViewPreferences, ViewStatePersistence,
};

pub struct BackgroundTask {
    pub id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub progress: f32,
}

pub struct App {
    pub running: bool,
    pub current_view: CurrentView,
    pub mode: AppMode,
    pub previous_mode: AppMode,
    pub input: TextInput,
    pub icon_mode: crate::icons::IconMode,
    pub panes: Vec<Pane>,
    pub focused_pane_index: usize,
    pub is_split_mode: bool,
    pub terminal_size: (u16, u16),
    pub mouse_pos: (u16, u16),
    pub system_state: SystemState,
    pub sidebar_focus: bool,
    pub sidebar_index: usize,
    pub starred: Vec<PathBuf>,
    pub recent_folders: Vec<PathBuf>,
    pub servers: Vec<crate::servers::ServerConfig>,
    pub pending_server: crate::servers::ServerConfig,
    pub external_tools: HashMap<String, Vec<crate::config::ExternalTool>>,
    pub show_sidebar: bool,
    pub sidebar_folders: bool,
    pub sidebar_favorites: bool,
    pub sidebar_recent: bool,
    pub sidebar_storage: bool,
    pub sidebar_remotes: bool,
    pub show_side_panel: bool,
    pub show_main_stage: bool,
    pub sidebar_width_percent: u16,
    pub sidebar_bounds: Vec<SidebarBounds>,
    pub sidebar_scroll_offset: usize,
    pub drag_start_pos: Option<(u16, u16)>,
    pub drag_source: Option<PathBuf>,
    pub is_dragging: bool,
    pub hovered_drop_target: Option<DropTarget>,
    pub last_action_msg: Option<(String, std::time::Instant)>,
    /// Stores per-folder state: (selected_index, scroll_offset).
    /// When re-entering a folder, both selection and scroll are restored.
    pub folder_memory: HashMap<PathBuf, (usize, usize)>,
    pub path_colors: HashMap<PathBuf, u8>,
    pub confirm_delete: bool,
    pub smart_date: bool,
    pub semantic_coloring: bool,
    pub auto_save: bool,
    pub default_show_hidden: bool,
    pub preview_max_mb: u16,
    pub single_columns: Vec<FileColumn>,
    pub split_columns: Vec<FileColumn>,
    pub monitor_subview: MonitorSubview,
    pub monitor_subview_bounds: Vec<(ratatui::layout::Rect, MonitorSubview)>,
    pub process_sort_col: ProcessColumn,
    pub process_sort_asc: bool,
    pub process_column_bounds: Vec<(ratatui::layout::Rect, ProcessColumn)>,
    pub process_selected_idx: Option<usize>,
    pub process_table_state: TableState,
    pub process_search_filter: String,
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
    pub header_icon_bounds: Vec<(ratatui::layout::Rect, String)>,
    pub tab_bounds: Vec<(ratatui::layout::Rect, usize, usize)>,
    pub hovered_header_icon: Option<String>,
    pub tab_tooltip: Option<(String, u16, u16)>, // (text, x, y)
    /// Folders expanded in the main file pane view.
    /// Controls expand/collapse in the file listing.
    pub expanded_folders: HashSet<PathBuf>,
    /// Folders expanded in the sidebar Tree view.
/// Independent of `expanded_folders` — Tree scope maintains its own expansion state.
    pub tree_expanded_folders: HashSet<PathBuf>,
    /// Cached sidebar tree items to avoid re-reading directories every frame.
    pub sidebar_tree_cache: Option<Vec<(PathBuf, u16, bool)>>,
    /// Cache key for invalidation (hash of tree_expanded_folders + show_hidden).
    pub sidebar_tree_cache_key: u64,
    /// Cached editor sidebar tree items.
    pub editor_sidebar_cache: Option<Vec<(PathBuf, u16, bool)>>,
    /// Cache key for editor sidebar (includes base_path hash).
    pub editor_sidebar_cache_key: u64,
    pub mouse_last_click: std::time::Instant,
    pub mouse_click_pos: (u16, u16),
    pub mouse_click_count: usize,
    pub file_manager_last_click: std::time::Instant,
    pub file_manager_click_pos: (u16, u16),
    /// Tracks remote connection health: server name -> (is_healthy, last_check_time)
    pub remote_health: HashMap<String, (bool, std::time::Instant)>,
    /// Cached checksums: path -> (md5, sha256)
    pub checksum_cache: HashMap<PathBuf, (String, String)>,
    pub is_resizing_sidebar: bool,
    pub editor_clipboard: Option<String>,
    pub clipboard: Option<(PathBuf, ClipboardOp)>,
    pub rename_selected: bool,
    pub editor_state: Option<PreviewState>,
    pub selection_mode: bool,
    pub prevent_mouse_up_selection_cleanup: bool,
    pub input_shield_until: Option<std::time::Instant>,
    pub command_index: usize,
    pub filtered_commands: Vec<CommandItem>,
    pub view_prefs: ViewStatePersistence,
    pub keybindings: crate::keybindings::Keybindings,
    pub settings_index: usize,
    pub settings_section: SettingsSection,
    pub settings_target: SettingsTarget,
    pub settings_scroll: u16,
    pub open_with_index: usize,
    pub replace_buffer: String,
    pub background_tasks: Vec<BackgroundTask>,
    /// Queue for compositor tile placements. Written by app, read by terminal engine.
    #[allow(dead_code)]
    pub tile_queue: Arc<StdMutex<Vec<TilePlacement>>>,
    pub saved_pane: Option<Pane>,
    /// Remote connection metadata cache: bookmark name -> (RemoteSession, last_used_time).
    /// NOTE: This caches connection PARAMETERS (host, user, port, key_path) — NOT an actual
    /// SSH session handle. Every remote operation (read_dir, exec, upload, etc.) still creates
    /// a fresh TCP connection + SSH handshake through the dracon_system providers. True
    /// connection pooling would require caching ssh2::Session objects at the provider level.
    /// Stale entries are cleaned up after 5 minutes of inactivity.
    pub remote_session_pool: HashMap<String, (RemoteSession, Instant)>,
    /// Detected terminal graphics protocol for inline image rendering.
    pub graphics_protocol: crate::term_graphics::GraphicsProtocol,
    /// Image queued for rendering after the next terminal.draw() call.
    pub pending_image_render: Option<crate::term_graphics::PendingImageRender>,
    /// Active content search query and results.
    pub content_search_query: String,
    pub content_search_results: Vec<crate::modules::rg::ContentSearchResult>,
    pub content_search_selected: usize,
    pub content_search_scroll: usize,
    /// Fzf-style quick filter state.
    pub quick_filter_input: String,
    /// Previous selection index to restore on quick filter cancel.
    pub quick_filter_prev_selection: Option<usize>,
    /// Shell command output buffer for CommandOutput mode.
    pub command_output: Vec<String>,
    pub command_output_scroll: usize,
    pub command_output_status: Option<String>,
    /// Custom user commands loaded from commands.toml
    pub user_commands: Vec<crate::user_commands::UserCommand>,
}

impl App {
    pub fn new(tile_queue: Arc<StdMutex<Vec<TilePlacement>>>) -> Self {
        let start_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let initial_fs = FileState::new(
            start_path,
            None,
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

        let system_state = SystemState {
            last_update: std::time::Instant::now(),
            disks: Vec::new(),
            processes: Vec::new(),
            cpu_usage: 0.0,
            cpu_cores: Vec::new(),
            mem_usage: 0.0,
            total_mem: 0.0,
            swap_usage: 0.0,
            total_swap: 0.0,
            cpu_history: VecDeque::with_capacity(100),
            core_history: Vec::new(),
            mem_history: VecDeque::with_capacity(100),
            swap_history: VecDeque::with_capacity(100),
            net_in: 0,
            net_out: 0,
            net_in_history: VecDeque::with_capacity(100),
            net_out_history: VecDeque::with_capacity(100),
            last_net_in: 0,
            last_net_out: 0,
            uptime: 0,
            os_name: String::new(),
            os_version: String::new(),
            kernel_version: String::new(),
            hostname: String::new(),
        };

        Self {
            running: true,
            current_view: CurrentView::Files,
            mode: AppMode::Normal,
            previous_mode: AppMode::Normal,
            input: TextInput::default(),
            icon_mode: crate::icons::IconMode::Nerd,
            panes: vec![Pane::new(initial_fs)],
            focused_pane_index: 0,
            is_split_mode: false,
            terminal_size: (80, 24),
            mouse_pos: (0, 0),
            system_state,
            sidebar_focus: false,
            sidebar_index: 0,
            starred: vec![
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
            servers: Vec::new(),
            pending_server: crate::servers::ServerConfig {
                name: String::new(),
                alias: None,
                host: String::new(),
                user: String::new(),
                port: 22,
                last_path: PathBuf::from("/"),
                key_path: None,
            },
            external_tools: HashMap::new(),
            show_sidebar: true,
            sidebar_folders: true,
            sidebar_favorites: true,
            sidebar_recent: true,
            sidebar_storage: true,
            sidebar_remotes: true,
            show_side_panel: true,
            show_main_stage: true,
            sidebar_width_percent: 15,
            sidebar_bounds: Vec::new(),
            sidebar_scroll_offset: 0,
            drag_start_pos: None,
            drag_source: None,
            is_dragging: false,
            hovered_drop_target: None,
            last_action_msg: None,
            folder_memory: HashMap::new(),
            path_colors: HashMap::new(),
            confirm_delete: true,
            smart_date: true,
            semantic_coloring: true,
            auto_save: true,
            default_show_hidden: false,
            preview_max_mb: PREVIEW_MAX_MB,
            single_columns: vec![
                FileColumn::Name,
                FileColumn::Size,
                FileColumn::Modified,
                FileColumn::Permissions,
            ],
            split_columns: vec![FileColumn::Name, FileColumn::Size],
            monitor_subview: MonitorSubview::Overview,
            monitor_subview_bounds: Vec::new(),
            process_sort_col: ProcessColumn::Cpu,
            process_sort_asc: false,
            process_column_bounds: Vec::new(),
            process_selected_idx: None,
            process_table_state: TableState::default(),
            process_search_filter: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            header_icon_bounds: Vec::new(),
            tab_bounds: Vec::new(),
            hovered_header_icon: None,
            tab_tooltip: None,
            expanded_folders: HashSet::new(),
            tree_expanded_folders: HashSet::new(),
            sidebar_tree_cache: None,
            sidebar_tree_cache_key: 0,
            editor_sidebar_cache: None,
            editor_sidebar_cache_key: 0,
            mouse_last_click: std::time::Instant::now(),
            mouse_click_pos: (0, 0),
            mouse_click_count: 0,
            file_manager_last_click: std::time::Instant::now(),
            file_manager_click_pos: (0, 0),
            remote_health: HashMap::new(),
            checksum_cache: HashMap::new(),
            is_resizing_sidebar: false,
            editor_clipboard: None,
            clipboard: None,
            rename_selected: false,
            editor_state: None,
            selection_mode: false,
            prevent_mouse_up_selection_cleanup: false,
            input_shield_until: None,
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
            settings_index: 0,
            settings_section: SettingsSection::General,
            settings_target: SettingsTarget::SingleMode,
            settings_scroll: 0,
            open_with_index: 0,
            replace_buffer: String::new(),
            background_tasks: Vec::new(),
            tile_queue,
            saved_pane: None,
            remote_session_pool: HashMap::new(),
            keybindings: crate::keybindings::Keybindings::load(),
            graphics_protocol: crate::term_graphics::detect_protocol(),
            pending_image_render: None,
            content_search_query: String::new(),
            content_search_results: Vec::new(),
            content_search_selected: 0,
            content_search_scroll: 0,
            quick_filter_input: String::new(),
            quick_filter_prev_selection: None,
            command_output: Vec::new(),
            command_output_scroll: 0,
            command_output_status: None,
            user_commands: Vec::new(),
        }
    }

    pub fn push_recent_folder(&mut self, path: PathBuf) {
        // Don't call is_dir() here — it blocks on slow filesystems.
        // Trust that paths coming from navigation are valid directories.
        self.recent_folders.retain(|p| p != &path);
        self.recent_folders.insert(0, path);
        if self.recent_folders.len() > MAX_RECENT_FOLDERS {
            self.recent_folders.truncate(MAX_RECENT_FOLDERS);
        }
    }

    pub fn current_file_state(&self) -> Option<&FileState> {
        self.panes
            .get(self.focused_pane_index)
            .and_then(|p| p.current_state())
    }

    pub fn current_tab_index(&self) -> usize {
        self.panes
            .get(self.focused_pane_index)
            .map(|p| p.active_tab_index)
            .unwrap_or(0)
    }

    pub fn current_file_state_mut(&mut self) -> Option<&mut FileState> {
        self.panes
            .get_mut(self.focused_pane_index)
            .and_then(|p| p.current_state_mut())
    }

    pub fn sidebar_width(&self) -> u16 {
        if !self.show_sidebar {
            return 0;
        }
        (self.terminal_size.0 as f32 * (self.sidebar_width_percent as f32 / 100.0)) as u16
    }

    pub fn toggle_split(&mut self) {
        self.is_split_mode = !self.is_split_mode;
        if self.is_split_mode && self.panes.len() == 1 {
            let initial_fs = self.panes[0].tabs[0].clone();
            self.panes.push(Pane::new(initial_fs));
        } else if !self.is_split_mode && self.panes.len() > 1 {
            self.panes.truncate(1);
            self.focused_pane_index = 0;
        }
    }

    pub fn save_current_view_prefs(&mut self) {
        match self.current_view {
            CurrentView::Files => {
                self.view_prefs.files.show_sidebar = self.show_sidebar;
                self.view_prefs.files.is_split_mode = self.is_split_mode;
            }
            CurrentView::Editor => {
                self.view_prefs.editor.show_sidebar = self.show_sidebar;
                self.view_prefs.editor.is_split_mode = self.is_split_mode;
            }
            _ => {}
        }
    }

    pub fn load_view_prefs(&mut self, view: CurrentView) {
        let prefs = match view {
            CurrentView::Files => &self.view_prefs.files,
            CurrentView::Editor => &self.view_prefs.editor,
            _ => return,
        };
        self.show_sidebar = prefs.show_sidebar;
        self.apply_split_mode(prefs.is_split_mode);
    }

    pub fn apply_split_mode(&mut self, split: bool) {
        if split && self.panes.len() == 1 {
            // Restore saved pane if available, otherwise clone the current one
            if let Some(saved) = self.saved_pane.take() {
                self.panes.push(saved);
            } else {
                let initial_fs = self.panes[0].tabs[0].clone();
                self.panes.push(Pane::new(initial_fs));
            }
        } else if !split && self.panes.len() > 1 {
            // Save the second pane before removing it
            self.saved_pane = self.panes.pop();
            self.focused_pane_index = 0;
        }
        self.is_split_mode = split;
    }

    pub fn toggle_hidden(&mut self) -> usize {
        if let Some(fs) = self.current_file_state_mut() {
            fs.show_hidden = !fs.show_hidden;
        }
        self.focused_pane_index
    }

    pub fn move_to_other_pane(&mut self) {
        if self.panes.len() > 1 {
            self.focused_pane_index = if self.focused_pane_index == 0 { 1 } else { 0 };
        }
    }

    pub fn resize_sidebar(&mut self, delta: i16) {
        let mut val = self.sidebar_width_percent as i16 + delta;
        val = val.clamp(5, 50);
        self.sidebar_width_percent = val as u16;
    }

    /// Insert a checksum into the cache, capping at 256 entries.
    /// When full, removes an arbitrary entry to make room.
    pub fn insert_checksum(&mut self, path: PathBuf, md5: String, sha256: String) {
        const MAX_CHECKSUM_CACHE: usize = 256;
        if self.checksum_cache.len() >= MAX_CHECKSUM_CACHE && !self.checksum_cache.contains_key(&path) {
            if let Some(first_key) = self.checksum_cache.keys().next().cloned() {
                self.checksum_cache.remove(&first_key);
            }
        }
        self.checksum_cache.insert(path, (md5, sha256));
    }

    /// Insert folder memory, capping at 500 entries.
    /// When full, clears half the entries to avoid repeated churn.
    pub fn insert_folder_memory(&mut self, path: PathBuf, idx: usize, scroll: usize) {
        const MAX_FOLDER_MEMORY: usize = 500;
        if self.folder_memory.len() >= MAX_FOLDER_MEMORY && !self.folder_memory.contains_key(&path) {
            // Remove half the entries to amortize cost
            let to_remove: Vec<_> = self.folder_memory.keys().take(MAX_FOLDER_MEMORY / 2).cloned().collect();
            for key in to_remove {
                self.folder_memory.remove(&key);
            }
        }
        self.folder_memory.insert(path, (idx, scroll));
    }

    /// Prune remote health entries older than 1 hour, with a max of 100 entries.
    pub fn prune_remote_health(&mut self) {
        const STALE_HEALTH_THRESHOLD: Duration = Duration::from_secs(3600);
        const MAX_REMOTE_HEALTH: usize = 100;
        let now = std::time::Instant::now();
        self.remote_health.retain(|_, (_, last_check)| {
            now.duration_since(*last_check) < STALE_HEALTH_THRESHOLD
        });
        // Also cap the total number of entries
        if self.remote_health.len() > MAX_REMOTE_HEALTH {
            let to_remove: Vec<_> = self.remote_health.keys().take(self.remote_health.len() - MAX_REMOTE_HEALTH).cloned().collect();
            for key in to_remove {
                self.remote_health.remove(&key);
            }
        }
    }

    pub fn move_up(&mut self, shift: bool) {
        if let Some(fs) = self.current_file_state_mut() {
            if let Some(sel) = fs.selection.selected {
                if sel > 0 {
                    let mut next = sel - 1;
                    // Skip divider
                    while next > 0
                        && fs
                            .files
                            .get(next)
                            .map(|p| p.to_string_lossy() == "__DIVIDER__")
                            .unwrap_or(false)
                    {
                        next -= 1;
                    }
                    // Final check if the skipped-to item is still a divider (shouldn't be at index 0, but safety first)
                    if fs
                        .files
                        .get(next)
                        .map(|p| p.to_string_lossy() == "__DIVIDER__")
                        .unwrap_or(false)
                    {
                        return;
                    }

                    fs.selection.handle_move(next, shift);
                    fs.table_state.select(fs.selection.selected);
                    if next < fs.table_state.offset() {
                        *fs.table_state.offset_mut() = next;
                    }
                }
            }
        }
    }

    pub fn move_down(&mut self, shift: bool) {
        if let Some(fs) = self.current_file_state_mut() {
            let capacity = fs.view_height.saturating_sub(3);
            if let Some(sel) = fs.selection.selected {
                if sel + 1 < fs.files.len() {
                    let mut next = sel + 1;
                    // Skip divider
                    while next < fs.files.len() - 1
                        && fs
                            .files
                            .get(next)
                            .map(|p| p.to_string_lossy() == "__DIVIDER__")
                            .unwrap_or(false)
                    {
                        next += 1;
                    }
                    // Final check
                    if fs
                        .files
                        .get(next)
                        .map(|p| p.to_string_lossy() == "__DIVIDER__")
                        .unwrap_or(false)
                    {
                        return;
                    }

                    fs.selection.handle_move(next, shift);
                    fs.table_state.select(fs.selection.selected);
                    if next >= fs.table_state.offset() + capacity {
                        let keep_visible = capacity.saturating_sub(1);
                        *fs.table_state.offset_mut() = next.saturating_sub(keep_visible);
                    }
                }
            }
        }
    }

    pub fn sidebar_move_up(&mut self) {
        if self.sidebar_index > 0 {
            self.sidebar_index -= 1;
        }
    }

    pub fn sidebar_move_down(&mut self, max: usize) {
        if self.sidebar_index < max.saturating_sub(1) {
            self.sidebar_index += 1;
        }
    }

    pub fn apply_process_sort(&mut self) {
        use crate::state::ProcessColumn;
        let col = self.process_sort_col;
        let asc = self.process_sort_asc;
        
        // Remember selected PID before sorting
        let selected_pid = self.process_selected_idx
            .and_then(|idx| self.system_state.processes.get(idx))
            .map(|p| p.pid);
        
        self.system_state.processes.sort_by(|a, b| {
            let ord = match col {
                ProcessColumn::Pid => a.pid.cmp(&b.pid),
                ProcessColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                ProcessColumn::Cpu => a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal),
                ProcessColumn::Mem => a.mem.partial_cmp(&b.mem).unwrap_or(std::cmp::Ordering::Equal),
                ProcessColumn::User => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
                ProcessColumn::Status => a.status.cmp(&b.status),
            };
            if asc { ord } else { ord.reverse() }
        });
        
        // Restore selection by PID
        if let Some(pid) = selected_pid {
            if let Some(new_idx) = self.system_state.processes.iter().position(|p| p.pid == pid) {
                self.process_selected_idx = Some(new_idx);
                self.process_table_state.select(Some(new_idx));
            }
        }
    }
}

const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024;

pub fn log_debug(msg: &str) {
    if !debug_logging_enabled() {
        return;
    }

    use std::io::Write;
    static LOG_FILE: std::sync::LazyLock<
        parking_lot::Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    > = std::sync::LazyLock::new(|| {
        let path = "debug.log";
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > MAX_LOG_SIZE_BYTES {
                let _ = std::fs::rename(path, format!("debug.log.1"));
                let _ = std::fs::remove_file("debug.log.2");
                let _ = std::fs::rename("debug.log.1", "debug.log.2");
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

pub fn debug_logging_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("TILES_DEBUG_LOG")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false)
    })
}

use tokio::sync::mpsc::Sender;

pub fn try_send_event(tx: &Sender<AppEvent>, evt: AppEvent) -> bool {
    if tx.try_send(evt).is_err() {
        log_debug("Channel send failed — event dropped");
        false
    } else {
        true
    }
}
