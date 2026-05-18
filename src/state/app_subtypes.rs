//! App sub-structs — grouped fields for maintainability.
//!
//! Each sub-struct owns a logical slice of App state.

use ratatui::widgets::TableState;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::state::{
    ClipboardOp, FileColumn, MonitorSubview, ProcessColumn, SettingsSection, SettingsTarget,
    SidebarBounds, UndoAction, ViewStatePersistence,
};

// ---------------------------------------------------------------------------
// AppCore — core app lifecycle and input
// ---------------------------------------------------------------------------

pub struct AppCore {
    pub running: bool,
    pub current_view: crate::state::CurrentView,
    pub mode: crate::state::AppMode,
    pub previous_mode: crate::state::AppMode,
    pub input: dracon_terminal_engine::widgets::TextInput,
    pub icon_mode: crate::icons::IconMode,
    pub is_split_mode: bool,
    pub terminal_size: (u16, u16),
    pub mouse_pos: (u16, u16),
}

impl Default for AppCore {
    fn default() -> Self {
        Self {
            running: false,
            current_view: crate::state::CurrentView::default(),
            mode: crate::state::AppMode::default(),
            previous_mode: crate::state::AppMode::default(),
            input: dracon_terminal_engine::widgets::TextInput::default(),
            icon_mode: crate::icons::IconMode::Nerd,
            is_split_mode: false,
            terminal_size: (80, 24),
            mouse_pos: (0, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// SidebarState — all sidebar / tree / cache fields
// ---------------------------------------------------------------------------

pub struct SidebarState {
    pub show_sidebar: bool,
    pub sidebar_focus: bool,
    pub sidebar_index: usize,
    pub sidebar_folders: bool,
    pub sidebar_favorites: bool,
    pub sidebar_recent: bool,
    pub sidebar_storage: bool,
    pub sidebar_remotes: bool,
    pub show_side_panel: bool,
    pub sidebar_width_percent: u16,
    pub sidebar_bounds: Vec<SidebarBounds>,
    pub sidebar_scroll_offset: usize,
    pub tree_expanded_folders: HashSet<PathBuf>,
    pub sidebar_tree_cache: Option<Vec<(PathBuf, u16, bool)>>,
    pub sidebar_tree_cache_key: u64,
    pub editor_sidebar_cache: Option<Vec<(PathBuf, u16, bool)>>,
    pub editor_sidebar_cache_key: u64,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
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
        }
    }
}

// ---------------------------------------------------------------------------
// MonitorState — system monitor / process list fields
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct MonitorState {
    pub monitor_subview: MonitorSubview,
    pub monitor_subview_bounds: Vec<(ratatui::layout::Rect, MonitorSubview)>,
    pub overview_scroll_offset: u16,
    pub process_sort_col: ProcessColumn,
    pub process_sort_asc: bool,
    pub process_column_bounds: Vec<(ratatui::layout::Rect, ProcessColumn)>,
    pub process_selected_idx: Option<usize>,
    pub process_table_state: TableState,
    pub process_search_filter: String,
    pub process_tree_view: bool,
}


// ---------------------------------------------------------------------------
// EditorGlobalState — global editor state (not per-file)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct EditorGlobalState {
    pub editor_state: Option<crate::state::PreviewState>,
    pub scroll_positions: HashMap<PathBuf, (usize, usize, usize, usize)>,
    pub replace_buffer: String,
    pub editor_clipboard: Option<String>,
}


// ---------------------------------------------------------------------------
// UndoState — undo/redo stacks
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct UndoState {
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
}


// ---------------------------------------------------------------------------
// SettingsState — settings modal state
// ---------------------------------------------------------------------------

pub struct SettingsState {
    pub settings_index: usize,
    pub settings_section: SettingsSection,
    pub settings_target: SettingsTarget,
    pub settings_scroll: u16,
    pub open_with_index: usize,
    pub confirm_delete: bool,
    pub smart_date: bool,
    pub semantic_coloring: bool,
    pub auto_save: bool,
    pub default_show_hidden: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            settings_index: 0,
            settings_section: SettingsSection::default(),
            settings_target: SettingsTarget::default(),
            settings_scroll: 0,
            open_with_index: 0,
            confirm_delete: true,
            smart_date: true,
            semantic_coloring: true,
            auto_save: true,
            default_show_hidden: false,
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutState — column configs, header/tab bounds, expanded folders
// ---------------------------------------------------------------------------

pub struct LayoutState {
    pub single_columns: Vec<FileColumn>,
    pub split_columns: Vec<FileColumn>,
    pub header_icon_bounds: Vec<(ratatui::layout::Rect, String)>,
    pub tab_bounds: Vec<(ratatui::layout::Rect, usize, usize)>,
    pub hovered_header_icon: Option<String>,
    pub expanded_folders: HashSet<PathBuf>,
    /// Pane rects from last render: (pane_index, Rect)
    pub pane_rects: Vec<ratatui::layout::Rect>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
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
        }
    }
}

// ---------------------------------------------------------------------------
// OutputState — background tasks, last action message, input shields
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct OutputState {
    pub background_tasks: Vec<crate::state::BackgroundTask>,
    pub last_action_msg: Option<(String, std::time::Instant)>,
    pub input_shield_until: Option<std::time::Instant>,
    pub input_shield_active_until: Option<std::time::Instant>,
}


// ---------------------------------------------------------------------------
// DragState — drag & drop
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct DragState {
    pub drag_start_pos: Option<(u16, u16)>,
    pub drag_source: Option<PathBuf>,
    pub is_dragging: bool,
    pub hovered_drop_target: Option<crate::state::DropTarget>,
/// Marquee selection: separate from file drag-drop
    pub is_marquee: bool,
    pub marquee_start: Option<(u16, u16)>,
    pub marquee_end: Option<(u16, u16)>,
    /// Pending click index — deferred handle_click on mouseUp if no drag/marquee occurred.
    pub pending_click_idx: Option<usize>,
}

/// Normalized marquee rect: (min_col, min_row, max_col, max_row)
#[derive(Clone, Copy, Debug)]
pub struct MarqueeRect {
    pub min_col: u16,
    pub min_row: u16,
    pub max_col: u16,
    pub max_row: u16,
}

impl DragState {
    /// Returns the normalized marquee rect, or None if not active.
    #[must_use]
    pub fn marquee_rect(&self) -> Option<MarqueeRect> {
        if !self.is_marquee {
            return None;
        }
        let (sx, sy) = self.marquee_start?;
        let (ex, ey) = self.marquee_end.unwrap_or((sx, sy));
        Some(MarqueeRect {
            min_col: sx.min(ex),
            min_row: sy.min(ey),
            max_col: sx.max(ex),
            max_row: sy.max(ey),
        })
    }

    /// Resets all marquee state.
    pub fn clear_marquee(&mut self) {
        self.is_marquee = false;
        self.marquee_start = None;
        self.marquee_end = None;
    }
}


/// Represents a closed tab that can be restored.
#[derive(Clone, Debug, Default)]
pub struct ClosedTab {
    pub path: PathBuf,
    pub pane_index: usize,
}

// ---------------------------------------------------------------------------
// NavState — starred, recent folders, command palette
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct NavState {
    pub starred: Vec<PathBuf>,
    pub recent_folders: Vec<PathBuf>,
    pub command_index: usize,
    pub filtered_commands: Vec<crate::state::CommandItem>,
    pub view_prefs: ViewStatePersistence,
/// Recently closed tabs — max 10 entries. Ctrl+Shift+T reopens the last one.
    pub closed_tabs: VecDeque<ClosedTab>,
}


// ---------------------------------------------------------------------------
// RemoteState — SSH bookmarks, pending connection, external tools
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct RemoteState {
    pub remote_bookmarks: Vec<crate::state::RemoteBookmark>,
    pub pending_remote: crate::state::RemoteBookmark,
    pub external_tools: HashMap<String, Vec<crate::config::ExternalTool>>,
}


// ---------------------------------------------------------------------------
// MouseState — click tracking, sidebar resize
// ---------------------------------------------------------------------------

pub struct MouseState {
    pub mouse_last_click: std::time::Instant,
    pub mouse_click_pos: (u16, u16),
    pub mouse_click_count: usize,
    pub is_resizing_sidebar: bool,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            mouse_last_click: std::time::Instant::now(),
            mouse_click_pos: (0, 0),
            mouse_click_count: 0,
            is_resizing_sidebar: false,
        }
    }
}

// ---------------------------------------------------------------------------
// SelectionState2 — selection mode, clipboard
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct SelectionState2 {
    pub selection_mode: bool,
    pub prevent_mouse_up_selection_cleanup: bool,
    pub rename_selected: bool,
    pub clipboard: Option<(PathBuf, ClipboardOp)>,
    pub path_colors: HashMap<PathBuf, u8>,
    pub folder_selections: HashMap<PathBuf, (usize, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_state_marquee_rect_normalizes() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((10, 5));
        drag.marquee_end = Some((3, 8));
        let rect = drag.marquee_rect().unwrap();
        assert_eq!(rect.min_col, 3);
        assert_eq!(rect.min_row, 5);
        assert_eq!(rect.max_col, 10);
        assert_eq!(rect.max_row, 8);
    }

    #[test]
    fn drag_state_marquee_rect_none_when_inactive() {
        let drag = DragState::default();
        assert!(drag.marquee_rect().is_none());
    }

    #[test]
    fn drag_state_clear_marquee() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((5, 5));
        drag.marquee_end = Some((10, 10));
        drag.clear_marquee();
        assert!(!drag.is_marquee);
        assert!(drag.marquee_start.is_none());
        assert!(drag.marquee_end.is_none());
    }

    #[test]
    fn drag_state_marquee_same_point() {
        let mut drag = DragState::default();
        drag.is_marquee = true;
        drag.marquee_start = Some((5, 5));
        drag.marquee_end = Some((5, 5));
        let rect = drag.marquee_rect().unwrap();
        assert_eq!(rect.min_col, 5);
        assert_eq!(rect.max_col, 5);
    }
}

