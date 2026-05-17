pub mod app_subtypes;
pub mod file_subtypes;

use crate::config::MAX_TABS;
use dracon_terminal_engine::contracts::UiEvent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use dracon_terminal_engine::widgets::TextEditor;

pub use dracon_terminal_engine::system::{DiskInfo, ProcessInfo};
pub use dracon_terminal_engine::utils::{FileCategory, FileColumn, IconMode, SelectionState};

// Sub-struct re-exports
#[allow(unused_imports)]
pub use app_subtypes::{
    AppCore, SidebarState, MonitorState, EditorGlobalState, UndoState, SettingsState,
    LayoutState, OutputState, DragState, NavState, RemoteState, MouseState, SelectionState2,
};
#[allow(unused_imports)]
pub use file_subtypes::{
    FileNavState, FileListState, FileViewState, FileGitState,
};

#[derive(Clone, Debug)]
pub struct DiskIo {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_rate_mbps: f64,
    pub write_rate_mbps: f64,
}

#[derive(Clone, Debug)]
pub struct NetInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: u64,
    pub tx_rate: u64,
    pub rx_history: VecDeque<u64>,
    pub tx_history: VecDeque<u64>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum AppEvent {
    Tick,
    RefreshFiles(usize),
    CreateFile(PathBuf),
    CreateFolder(PathBuf),
    Rename(PathBuf, PathBuf),
    Delete(PathBuf),
    TrashFile(PathBuf),
    Copy(PathBuf, PathBuf),
    Symlink(PathBuf, PathBuf),
    StatusMsg(String),
    FilesChangedOnDisk(PathBuf),
    PreviewRequested(usize, PathBuf),
    SaveFile(PathBuf, String),
    GitHistory,
    SystemMonitor,
    AddToFavorites(PathBuf),
    ConnectToRemote(usize, usize),
    RemoteConnected(usize, RemoteSession),
    SystemUpdated(dracon_system_lib::SystemSnapshot),

    KillProcess(u32),
    GitHistoryUpdated(
        usize,
        usize,
        Vec<CommitInfo>,
        Vec<GitPendingChange>,
        Option<String>,
        usize,
        usize,
        Option<String>,
        Vec<String>, // Remotes
        Vec<String>, // Stashes
    ),
    TaskProgress(uuid::Uuid, f32, String),
    TaskFinished(uuid::Uuid),
    GlobalSearchUpdated(usize, Vec<PathBuf>, HashMap<PathBuf, FileMetadata>),
    SpawnTerminal {
        path: PathBuf,
        new_tab: bool,
        remote: Option<RemoteSession>,
        command: Option<String>,
    },
    SpawnDetached {
        cmd: String,
        args: Vec<String>,
    },
    Editor,
    Ui(UiEvent),
    Raw(dracon_terminal_engine::contracts::InputEvent),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum CurrentView {
    #[default]
    Files,
    Editor,
    Commit,
    Git,
    Processes,
    Debug,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContextMenuTarget {
    File(usize),
    Folder(usize),
    EmptySpace,
    SidebarFavorite(PathBuf),
    SidebarRemote(usize),
    SidebarStorage(usize),
    ProjectTree(PathBuf),
    Process(u32),
    Editor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContextMenuAction {
    Open,
    OpenNewTab,
    OpenWith,
    Edit,
    Run,
    RunTerminal,
    ExtractHere,
    NewFolder,
    NewFile,
    Cut,
    Copy,
    CopyPath,
    CopyName,
    Paste,
    Rename,
    Duplicate,
    Compress,
    Delete,
    AddToFavorites,
    RemoveFromFavorites,
    Properties,
    TerminalWindow,
    TerminalTab,
    Refresh,
    SelectAll,
    ToggleHidden,
    ConnectRemote,
    DeleteRemote,
    Mount,
    Unmount,
    SetWallpaper,
    GitInit,
    GitStatus,
    SystemMonitor,
    Drag,
    SetColor(Option<u8>),
    SortBy(FileColumn),
    Separator,
    Save,
    EditorCut,
    EditorCopy,
    EditorPaste,
    EditorUndo,
    EditorRedo,
    EditorSelectAll,
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Columns,
    Tabs,
    Remotes,
    Shortcuts,
    Style,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum SettingsTarget {
    #[default]
    SingleMode,
    SplitMode,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    #[default]
    Normal,
    Editor,
    EditorSearch,
    EditorGoToLine,
    EditorReplace,
    Settings,
    Properties,
    Rename,
    NewFile,
    NewFolder,
    Delete(String),
    DeleteFile(PathBuf),
    Search,
    PathInput,
    SaveAs(PathBuf),
    CommandPalette,
    StyleColorInput,
    ResetSettingsConfirm,
    AddRemote(usize),
    ImportServers,
    Viewer,
    Hotkeys,
    Header(usize),
    Highlight,
    BulkRename {
        files: Vec<PathBuf>,
        pattern: String,
        replacement: String,
        matched_indices: Vec<usize>,
        selected_index: Option<usize>,
    },
    OpenWith(PathBuf),
    DragDropMenu {
        sources: Vec<PathBuf>,
        target: PathBuf,
    },
    ContextMenu {
        x: u16,
        y: u16,
        target: ContextMenuTarget,
        actions: Vec<ContextMenuAction>,
        selected_index: Option<usize>,
    },
    SignalSelect {
        pid: u32,
        name: String,
        selected_index: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DropTarget {
    Favorites,
    Folder(PathBuf),
    ReorderFavorite(usize),
}

#[derive(Clone, Debug, Default)]
pub struct FileRowBounds {
    pub file_idx: usize,
    pub arrow_end_x: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SidebarBounds {
    pub y: u16,
    pub index: usize,
    pub target: SidebarTarget,
    pub arrow_end_x: u16,
}

impl Default for SidebarBounds {
    fn default() -> Self {
        Self {
            y: 0,
            index: 0,
            target: SidebarTarget::Header(String::new()),
            arrow_end_x: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandItem {
    pub key: String,
    pub desc: String,
    pub action: CommandAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SidebarTarget {
    Favorite(PathBuf),
    Recent(PathBuf),
    Remote(usize),
    Storage(usize),
    Project(PathBuf),
    Header(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandAction {
    Quit,
    ToggleZoom,
    SwitchView(CurrentView),
    AddRemote,
    ConnectToRemote(usize),
    CommandPalette,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub created: std::time::SystemTime,
    pub permissions: u32,
    pub is_dir: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RemoteBookmark {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub last_path: PathBuf,
    pub key_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoteSession {
    pub host: String,
    pub user: String,
    pub name: String,
    pub port: u16,
    pub key_path: Option<PathBuf>,
}

impl std::fmt::Debug for RemoteSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteSession")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileState {
    pub nav: FileNavState,
    pub list: FileListState,
    pub view: FileViewState,
    pub git: FileGitState,
}

impl FileState {
    pub fn new(
        path: PathBuf,
        remote: Option<RemoteSession>,
        show_hidden: bool,
        columns: Vec<FileColumn>,
        sort_col: FileColumn,
        sort_asc: bool,
    ) -> Self {
        Self {
            nav: FileNavState {
                current_path: path.clone(),
                remote_session: remote,
                show_hidden,
                search_filter: String::new(),
                search_generation: 0,
                history: vec![path],
                history_index: 0,
                sort_column: sort_col,
                sort_ascending: sort_asc,
                search_debounce_until: None,
            },
            list: FileListState {
                files: Vec::new(),
                selection: SelectionState::default(),
                columns,
                local_count: 0,
                tree_file_depths: Vec::new(),
                metadata: HashMap::new(),
                path_colors: HashMap::new(),
            },
            view: FileViewState {
                preview: None,
                view_height: 0,
                table_state: ratatui::widgets::TableState::default(),
                column_bounds: Vec::new(),
                breadcrumb_bounds: Vec::new(),
                breadcrumb_header_bounds: None,
                pending_select_path: None,
                file_row_bounds: Vec::new(),
            },
            git: FileGitState {
                git_history: Vec::new(),
                git_history_state: ratatui::widgets::TableState::default(),
                git_pending_state: ratatui::widgets::TableState::default(),
                git_branch: None,
                git_ahead: 0,
                git_behind: 0,
                git_pending: Vec::new(),
                git_summary: None,
                git_remotes: Vec::new(),
                git_stashes: Vec::new(),
                git_cache_until: None,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct SystemState {
    #[allow(dead_code)]
    pub last_update: std::time::Instant,
    pub disks: Vec<DiskInfo>,
    pub disk_io: HashMap<String, DiskIo>,
    pub last_disk_io: HashMap<String, DiskIo>,
    pub disk_read_history: VecDeque<u64>,
    pub disk_write_history: VecDeque<u64>,
    pub processes: Vec<ProcessInfo>,
    pub process_ppid: HashMap<u32, u32>,
    pub cpu_usage: f32,
    pub cpu_cores: Vec<f32>,
    pub cpu_temperature: Option<f32>,
    pub cpu_frequency: Option<f32>,
    pub mem_usage: f32,
    pub total_mem: f32,
    pub swap_usage: f32,
    pub total_swap: f32,
    pub cpu_history: VecDeque<u64>,
    pub core_history: Vec<VecDeque<u64>>,
    pub mem_history: VecDeque<u64>,
    pub swap_history: VecDeque<u64>,
    pub net_in: u64,
    pub net_out: u64,
    pub net_in_history: VecDeque<u64>,
    pub net_out_history: VecDeque<u64>,
    pub last_net_in: u64,
    pub last_net_out: u64,
    pub net_interfaces: Vec<NetInterface>,
    pub last_net_interfaces: Vec<NetInterface>,
    pub uptime: u64,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            last_update: std::time::Instant::now(),
            disks: Vec::new(),
            disk_io: HashMap::new(),
            last_disk_io: HashMap::new(),
            disk_read_history: VecDeque::from(vec![0; 100]),
            disk_write_history: VecDeque::from(vec![0; 100]),
            processes: Vec::new(),
            process_ppid: HashMap::new(),
            cpu_usage: 0.0,
            cpu_cores: Vec::new(),
            cpu_temperature: None,
            cpu_frequency: None,
            mem_usage: 0.0,
            total_mem: 0.0,
            swap_usage: 0.0,
            total_swap: 0.0,
            cpu_history: VecDeque::from(vec![0; 100]),
            core_history: Vec::new(),
            mem_history: VecDeque::from(vec![0; 100]),
            swap_history: VecDeque::from(vec![0; 100]),
            net_in: 0,
            net_out: 0,
            net_in_history: VecDeque::from(vec![0; 100]),
            net_out_history: VecDeque::from(vec![0; 100]),
            last_net_in: 0,
            last_net_out: 0,
            net_interfaces: Vec::new(),
            last_net_interfaces: Vec::new(),
            uptime: 0,
            os_name: String::new(),
            os_version: String::new(),
            kernel_version: String::new(),
            hostname: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreviewState {
    pub path: PathBuf,
    pub content: String,
    pub editor: Option<TextEditor>,
    pub last_saved: Option<std::time::Instant>,
    pub highlighted_lines: Option<Vec<ratatui::text::Line<'static>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ViewPreferences {
    pub show_sidebar: bool,
    pub is_split_mode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ViewStatePersistence {
    pub files: ViewPreferences,
    pub editor: ViewPreferences,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pane {
    pub tabs: Vec<FileState>,
    pub active_tab_index: usize,
}

impl Pane {
    pub fn new(initial_fs: FileState) -> Self {
        Self {
            tabs: vec![initial_fs],
            active_tab_index: 0,
        }
    }
    pub fn current_state(&self) -> Option<&FileState> {
        self.tabs.get(self.active_tab_index)
    }
    pub fn current_state_mut(&mut self) -> Option<&mut FileState> {
        self.tabs.get_mut(self.active_tab_index)
    }
    pub fn open_tab(&mut self, fs: FileState) {
        if self.tabs.len() >= MAX_TABS {
            return;
        }
        self.tabs.push(fs);
        self.active_tab_index = self.tabs.len() - 1;
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct BackgroundTask {
    pub id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub progress: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum UndoAction {
    Rename(PathBuf, PathBuf),
    Move(PathBuf, PathBuf),
    Copy(PathBuf, PathBuf),
    Delete(PathBuf),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub decorations: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitPendingChange {
    pub status: String,
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum MonitorSubview {
    #[default]
    Overview,
    Processes,
    Applications,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum ProcessColumn {
    #[default]
    Pid,
    Name,
    Cpu,
    Mem,
    User,
    Status,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClipboardOp {
    Copy,
    Cut,
}
