use crate::config::MAX_TABS;
use std::time::Instant;
use dracon_terminal_engine::contracts::UiEvent;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use dracon_terminal_engine::widgets::TextEditor;

pub use dracon_terminal_engine::system::{DiskInfo, ProcessInfo};
pub use dracon_terminal_engine::utils::{FileCategory, FileColumn, IconMode, SelectionState};

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
    Chmod(PathBuf, u32),
    CreateArchive(Vec<PathBuf>, PathBuf, usize),
    ComputeChecksums(PathBuf),
    Copy(PathBuf, PathBuf),
    UploadToRemote(PathBuf, PathBuf),
    FolderSizesUpdated(usize, std::collections::HashMap<std::path::PathBuf, u64>),
    CompareFiles(PathBuf, PathBuf),
    Symlink(PathBuf, PathBuf),
    StatusMsg(String),
    FilesChangedOnDisk(PathBuf),
    PreviewRequested(usize, PathBuf),
    SaveFile(PathBuf, String),
    GitHistory,
    SystemMonitor,
    AddToFavorites(PathBuf),
    ConnectToRemote(usize, usize),
    RemoteConnected(usize, RemoteSession, String),
    ReconnectRemote(usize),
    SystemUpdated(dracon_system::SystemSnapshot),

    KillProcess(u32),
    GitHistoryUpdated(Box<GitHistoryData>),
    GitDiffFetched(usize, usize, String),
    GitStageFile(usize, usize, String),
    GitUnstageFile(usize, usize, String),
    GitStageAll(usize, usize),
    GitUnstageAll(usize, usize),
    GitCommit(usize, usize, String),
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
    ServersTomlChanged,
    ContentSearchStart(String, PathBuf),
    ContentSearchResults(Vec<crate::modules::rg::ContentSearchResult>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CurrentView {
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
    CollapseAll,
    Drag,
    Compare,
    Download,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SettingsSection {
    General,
    Columns,
    Tabs,
    Remotes,
    Shortcuts,
    Style,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SettingsTarget {
    SingleMode,
    SplitMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    Normal,
    Editor,
    EditorSearch,
    EditorGoToLine,
    EditorReplace,
    Settings,
    Properties,
    EditPermissions(PathBuf),
    Rename,
    NewFile,
    NewFolder,
    Delete(String),
    DeleteFile(PathBuf),
    KillProcessConfirm(u32, String),
    ProcessSearch,
    Search,
    PathInput,
    SaveAs(PathBuf),
    CommandPalette,
    StyleColorInput,
    ResetSettingsConfirm,
    AddRemote(usize),
    ImportServers,
    ImportSshConfig,
    CreateArchive(Vec<PathBuf>, usize),
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
        target_is_remote: bool,
    },
    ContextMenu {
        x: u16,
        y: u16,
        target: ContextMenuTarget,
        actions: Vec<ContextMenuAction>,
        selected_index: Option<usize>,
    },
    ContentSearch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DropTarget {
    Favorites,
    Folder(PathBuf),
    ReorderFavorite(usize),
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
    CollapseFolders,
    ContentSearch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub created: std::time::SystemTime,
    pub permissions: u32,
    pub is_dir: bool,
    #[serde(default)]
    pub is_symlink: bool,
    #[serde(default)]
    pub link_target: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteBookmark {
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub last_path: PathBuf,
    pub key_path: Option<PathBuf>,
}

impl RemoteBookmark {
    /// Returns the alias if set, otherwise the name
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoteSession {
    pub host: String,
    pub user: String,
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
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

impl RemoteSession {
    /// Returns the alias if set, otherwise the name
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileState {
    pub current_path: PathBuf,
    pub remote_session: Option<RemoteSession>,
    pub bookmark_idx: Option<usize>,
    #[serde(skip)]
    pub retry_count: u8,
    pub files: Vec<PathBuf>,
    pub selection: SelectionState,
    pub show_hidden: bool,
    pub search_filter: String,
    #[serde(skip)]
    pub search_generation: u64,
    pub columns: Vec<FileColumn>,
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    pub sort_column: FileColumn,
    pub sort_ascending: bool,
    #[serde(skip)]
    pub metadata: HashMap<PathBuf, FileMetadata>,
    #[serde(skip)]
    pub folder_sizes: HashMap<PathBuf, u64>,
    #[serde(skip)]
    pub preview: Option<PreviewState>,
    #[serde(skip)]
    pub view_height: usize,
    #[serde(skip)]
    pub table_state: ratatui::widgets::TableState,
    #[serde(skip)]
    pub column_bounds: Vec<(ratatui::layout::Rect, FileColumn)>,
    #[serde(skip)]
    pub breadcrumb_bounds: Vec<(ratatui::layout::Rect, PathBuf)>,
    #[serde(skip)]
    pub breadcrumb_header_bounds: Option<ratatui::layout::Rect>,
    #[serde(skip)]
    pub local_count: usize,
    #[serde(skip)]
    pub pending_select_path: Option<(PathBuf, usize)>,
    #[serde(skip)]
    pub git_history: Vec<CommitInfo>,
    #[serde(skip)]
    pub git_history_state: ratatui::widgets::TableState,
    #[serde(skip)]
    pub git_pending_state: ratatui::widgets::TableState,
    #[serde(skip)]
    pub git_branch: Option<String>,
    #[serde(skip)]
    pub git_ahead: usize,
    #[serde(skip)]
    pub git_behind: usize,
    #[serde(skip)]
    pub git_pending: Vec<GitPendingChange>,
    #[serde(skip)]
    pub git_summary: Option<String>,
    #[serde(skip)]
    pub git_remotes: Vec<String>,
    #[serde(skip)]
    pub git_stashes: Vec<String>,
    #[serde(skip)]
    pub git_search_filter: String,
    #[serde(skip)]
    pub git_pending_diff: Option<String>,
    #[serde(skip)]
    pub git_diff_for_path: Option<String>,
    #[serde(skip)]
    pub git_cache_until: Option<Instant>,
    #[serde(skip)]
    pub last_folder_size_calc: Option<Instant>,
    #[serde(skip)]
    pub search_debounce_until: Option<std::time::Instant>,
    #[serde(default)]
    pub tree_file_depths: Vec<u16>,
    #[serde(default)]
    pub loading: bool,
}

impl FileState {
    /// Clamps a scroll offset to the valid range for the current file list.
    /// Prevents restoring a scroll position that's beyond the end of the list.
    pub fn clamped_scroll(&self, scroll: usize) -> usize {
        let max = self.files.len().saturating_sub(self.view_height.saturating_sub(3));
        scroll.min(max)
    }

    pub fn new(
        path: PathBuf,
        remote: Option<RemoteSession>,
        bookmark_idx: Option<usize>,
        show_hidden: bool,
        columns: Vec<FileColumn>,
        sort_col: FileColumn,
        sort_asc: bool,
    ) -> Self {
        Self {
            current_path: path.clone(),
            remote_session: remote,
            bookmark_idx,
            files: Vec::new(),
            selection: SelectionState::default(),
            show_hidden,
            search_filter: String::new(),
            search_generation: 0,
            columns,
            history: vec![path],
            history_index: 0,
            sort_column: sort_col,
            sort_ascending: sort_asc,
            metadata: HashMap::new(),
            folder_sizes: HashMap::new(),
            preview: None,
            view_height: 0,
            table_state: ratatui::widgets::TableState::default(),
            column_bounds: Vec::new(),
            breadcrumb_bounds: Vec::new(),
            breadcrumb_header_bounds: None,
            local_count: 0,
            retry_count: 0,
            pending_select_path: None,
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
            git_search_filter: String::new(),
            git_pending_diff: None,
            git_diff_for_path: None,
            git_cache_until: None,
            last_folder_size_calc: None,
            search_debounce_until: None,
            tree_file_depths: Vec::new(),
            loading: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SystemState {
    #[allow(dead_code)]
    pub last_update: std::time::Instant,
    pub disks: Vec<DiskInfo>,
    pub processes: Vec<ProcessInfo>,
    pub cpu_usage: f32,
    pub cpu_cores: Vec<f32>,
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
    pub uptime: u64,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
}

#[derive(Clone, Debug)]
pub struct PreviewState {
    pub path: PathBuf,
    pub content: String,
    pub editor: Option<TextEditor>,
    pub last_saved: Option<std::time::Instant>,
    pub image_data: Option<(Vec<u8>, u32, u32)>,
    pub highlighted_lines: Option<Vec<ratatui::text::Line<'static>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewPreferences {
    pub show_sidebar: bool,
    pub is_split_mode: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub graph: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitPendingChange {
    pub status: String,
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug)]
pub struct GitHistoryData {
    pub pane_idx: usize,
    pub tab_idx: usize,
    pub history: Vec<CommitInfo>,
    pub pending: Vec<GitPendingChange>,
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub summary: Option<String>,
    pub remotes: Vec<String>,
    pub stashes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MonitorSubview {
    Overview,
    Cpu,
    Memory,
    Disk,
    Network,
    Processes,
    Applications,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_scroll_returns_zero_for_empty_files() {
        let mut fs = FileState::new(
            PathBuf::from("/test"),
            None,
            None,
            false,
            vec![FileColumn::Name],
            FileColumn::Name,
            true,
        );
        fs.files = vec![];
        fs.view_height = 20;

        assert_eq!(fs.clamped_scroll(0), 0);
        assert_eq!(fs.clamped_scroll(100), 0);
    }

    #[test]
    fn clamped_scroll_returns_min_of_scroll_and_max() {
        let mut fs = FileState::new(
            PathBuf::from("/test"),
            None,
            None,
            false,
            vec![FileColumn::Name],
            FileColumn::Name,
            true,
        );
        fs.files = vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")];
        fs.view_height = 10;

        assert_eq!(fs.clamped_scroll(0), 0);
        assert_eq!(fs.clamped_scroll(1), 0);
    }

    #[test]
    fn clamped_scroll_handles_view_height_larger_than_files() {
        let mut fs = FileState::new(
            PathBuf::from("/test"),
            None,
            None,
            false,
            vec![FileColumn::Name],
            FileColumn::Name,
            true,
        );
        fs.files = vec![PathBuf::from("a"), PathBuf::from("b")];
        fs.view_height = 100;

        assert_eq!(fs.clamped_scroll(0), 0);
        assert_eq!(fs.clamped_scroll(50), 0);
    }

    #[test]
    fn clamped_scroll_clamps_large_scroll_values() {
        let mut fs = FileState::new(
            PathBuf::from("/test"),
            None,
            None,
            false,
            vec![FileColumn::Name],
            FileColumn::Name,
            true,
        );
        fs.files = vec![PathBuf::from("a"); 20];
        fs.view_height = 10;

        let max = fs.files.len().saturating_sub(fs.view_height.saturating_sub(3));
        assert_eq!(fs.clamped_scroll(100), max);
        assert_eq!(fs.clamped_scroll(max), max);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProcessColumn {
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
