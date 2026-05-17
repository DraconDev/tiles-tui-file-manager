//! FileState sub-structs — grouped fields for maintainability.
//!
//! FileState was ~42 fields split across: core nav, file list, view layout,
//! and git state. This module decomposes it into 4 focused sub-structs.
//! NOTE: All structs are #[allow(dead_code)] because FileState is not yet
//! migrated to use them — they are available for the future migration.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::state::{
    CommitInfo, FileColumn, FileMetadata, GitPendingChange, RemoteSession,
};

// ---------------------------------------------------------------------------
// FileNavState — navigation core: path, history, filters, sorting
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct FileNavState {
    pub current_path: PathBuf,
    pub remote_session: Option<RemoteSession>,
    pub show_hidden: bool,
    pub search_filter: String,
    pub search_generation: u64,
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    pub sort_column: FileColumn,
    pub sort_ascending: bool,
    pub search_debounce_until: Option<std::time::Instant>,
}

impl Default for FileNavState {
    fn default() -> Self {
        Self {
            current_path: PathBuf::new(),
            remote_session: None,
            show_hidden: false,
            search_filter: String::new(),
            search_generation: 0,
            history: Vec::new(),
            history_index: 0,
            sort_column: FileColumn::Name,
            sort_ascending: true,
            search_debounce_until: None,
        }
    }
}

// ---------------------------------------------------------------------------
// FileListState — file listing and selection (non-serialized)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct FileListState {
    pub files: Vec<PathBuf>,
    pub selection: dracon_terminal_engine::utils::SelectionState,
    pub columns: Vec<FileColumn>,
    pub local_count: usize,
    pub tree_file_depths: Vec<u16>,
    pub metadata: HashMap<PathBuf, FileMetadata>,
    pub path_colors: HashMap<PathBuf, u8>,
}

impl Default for FileListState {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            selection: dracon_terminal_engine::utils::SelectionState::default(),
            columns: vec![
                FileColumn::Name,
                FileColumn::Size,
                FileColumn::Modified,
                FileColumn::Permissions,
            ],
            local_count: 0,
            tree_file_depths: Vec::new(),
            metadata: HashMap::new(),
            path_colors: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// FileViewState — rendering/layout state (non-serialized UI state)
// ---------------------------------------------------------------------------
// NOTE: No #[derive(Default)] because PreviewState contains Instant (no Default).
// Explicit impl below uses PreviewState::default() which fails — we use None instead.

#[allow(dead_code)]
pub struct FileViewState {
    pub preview: Option<crate::state::PreviewState>,
    pub view_height: usize,
    pub table_state: ratatui::widgets::TableState,
    pub column_bounds: Vec<(ratatui::layout::Rect, FileColumn)>,
    pub breadcrumb_bounds: Vec<(ratatui::layout::Rect, PathBuf)>,
    pub breadcrumb_header_bounds: Option<ratatui::layout::Rect>,
    pub pending_select_path: Option<(PathBuf, usize)>,
    pub file_row_bounds: Vec<crate::state::FileRowBounds>,
}

// NOTE: Cannot derive Default — PreviewState contains Instant (no Default). Manual impl.
#[allow(clippy::derivable_impls)]
impl Default for FileViewState {
    fn default() -> Self {
        Self {
            preview: None,
            view_height: 0,
            table_state: ratatui::widgets::TableState::default(),
            column_bounds: Vec::new(),
            breadcrumb_bounds: Vec::new(),
            breadcrumb_header_bounds: None,
            pending_select_path: None,
            file_row_bounds: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// FileGitState — git integration state (non-serialized)
// ---------------------------------------------------------------------------
// NOTE: No #[derive(Default)] because git_cache_until: Option<Instant>
// can't derive Default. Explicit impl below.

#[allow(dead_code)]
pub struct FileGitState {
    pub git_history: Vec<CommitInfo>,
    pub git_history_state: ratatui::widgets::TableState,
    pub git_pending_state: ratatui::widgets::TableState,
    pub git_branch: Option<String>,
    pub git_ahead: usize,
    pub git_behind: usize,
    pub git_pending: Vec<GitPendingChange>,
    pub git_summary: Option<String>,
    pub git_remotes: Vec<String>,
    pub git_stashes: Vec<String>,
    pub git_cache_until: Option<std::time::Instant>,
}

// NOTE: Cannot derive Default — Instant has no Default impl. Manual impl is required.
// clippy: suppress "this impl can be derived" since derive isn't possible here.
#[allow(clippy::derivable_impls)]
impl Default for FileGitState {
    fn default() -> Self {
        Self {
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
        }
    }
}