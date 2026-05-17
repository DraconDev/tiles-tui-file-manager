#![allow(unused_imports)]

//! App setup and initialization.
//! Extracted from main.rs (Phase 4).

use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::collections::HashSet;

use crate::app::{App, AppEvent};
use crate::config::{fuzzy_contains, FILE_WATCH_DEBOUNCE_MS, FUZZY_SEARCH, GIT_CACHE_TTL_SECONDS, MAX_TREE_DEPTH, MPSC_CHANNEL_CAPACITY};
use crate::events;
use crate::state::FileState;
use dracon_terminal_engine::compositor::engine::TilePlacement;
use dracon_terminal_engine::contracts::InputEvent as Event;
use tokio::sync::mpsc;

pub type PLMutex<T> = parking_lot::Mutex<T>;

pub fn setup_app(
    tile_queue: Arc<StdMutex<Vec<dracon_terminal_engine::compositor::engine::TilePlacement>>>,
) -> (
    Arc<PLMutex<App>>,
    mpsc::Sender<AppEvent>,
    mpsc::Receiver<AppEvent>,
) {
    let (tx, rx) = mpsc::channel(MPSC_CHANNEL_CAPACITY);
    let mut app = App::new(tile_queue);

    if let Some(state) = crate::config::load_state() {
        if !state.panes.is_empty() {
            app.panes = state.panes;
        }
        for pane in &mut app.panes {
            if pane.tabs.is_empty() {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                pane.tabs.push(crate::state::FileState::new(
                    cwd,
                    None,
                    app.settings.default_show_hidden,
                    app.layout.single_columns.clone(),
                    crate::state::FileColumn::Name,
                    true,
                ));
                pane.active_tab_index = 0;
            } else if pane.active_tab_index >= pane.tabs.len() {
                pane.active_tab_index = 0;
            }

            for tab in &mut pane.tabs {
                // Never trust persisted transient tab data; force a clean first refresh.
                tab.list.files.clear();
                tab.list.metadata.clear();
                tab.nav.search_filter.clear();
                tab.list.local_count = 0;
                tab.list.selection.clear_multi();
                tab.list.selection.anchor = None;
                tab.list.selection.selected = None;
                *tab.view.table_state.offset_mut() = 0;
            }
        }
        app.focused_pane_index = state.focused_pane_index;
        if app.focused_pane_index >= app.panes.len() {
            app.focused_pane_index = 0;
        }

        // Ensure CWD is active on start, keeping history
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(pane) = app.panes.get_mut(0) {
                if let Some(fs) = pane.current_state_mut() {
                    // Always start local on pane 1/tab active, otherwise a persisted
                    // remote_session can make startup refresh return an empty listing.
                    fs.nav.remote_session = None;
                    if fs.nav.current_path != cwd {
                        fs.nav.current_path = cwd.clone();
                        crate::event_helpers::push_history(fs, cwd);
                    }
                }
            }
        }

        // Merge favorites (Defaults + Loaded)
        let mut loaded_starred = state.starred;
        for def in app.nav.starred {
            if !loaded_starred.contains(&def) {
                loaded_starred.push(def);
            }
        }
        app.nav.starred = loaded_starred;

        app.remote.remote_bookmarks = state.remote_bookmarks;
        crate::config::merge_ssh_config_bookmarks(&mut app.remote.remote_bookmarks);
        app.selection.path_colors = state.path_colors;
        app.remote.external_tools = state.external_tools;
        if let Some(mode) = state.icon_mode {
            app.core.icon_mode = mode;
        }
        app.core.is_split_mode = state.is_split_mode;
        app.settings.semantic_coloring = state.semantic_coloring;
        app.sidebar.show_sidebar = state.show_sidebar;
        app.sidebar.sidebar_folders = state.sidebar_folders;
        app.sidebar.sidebar_favorites = state.sidebar_favorites;
        app.sidebar.sidebar_recent = state.sidebar_recent;
        app.sidebar.sidebar_storage = state.sidebar_storage;
        app.sidebar.sidebar_remotes = state.sidebar_remotes;
        app.sidebar.show_side_panel = state.show_side_panel;
        app.settings.default_show_hidden = state.default_show_hidden;
        app.settings.auto_save = state.auto_save;
        app.preview_max_mb = state.preview_max_mb.max(1);
        app.layout.expanded_folders = state.expanded_folders.into_iter().collect();
        app.sidebar.sidebar_width_percent = state.sidebar_width_percent;
        app.nav.recent_folders = state.recent_folders;
        if let Some(theme_style) = state.theme_style {
            crate::ui::theme::set_style_settings(theme_style);
        }
    }

    // Prime visible tabs synchronously so startup never renders as empty while waiting
    // for async refresh/tick scheduling.
    prime_visible_tabs(&mut app);

    let app_arc = Arc::new(PLMutex::new(app));
    (app_arc, tx, rx)
}

pub fn handle_event(
    evt: Event,
    app: &mut App,
    event_tx: mpsc::Sender<AppEvent>,
    panes_needing_refresh: &mut std::collections::HashSet<usize>,
) -> bool {
    events::handle_event(evt, app, event_tx, panes_needing_refresh)
}

pub fn prime_visible_tabs(app: &mut App) {
    for pane in &mut app.panes {
        if let Some(fs) = pane.current_state_mut() {
            prime_local_file_state(fs);
        }
    }
}

pub fn prime_local_file_state(fs: &mut crate::state::FileState) {
    if fs.nav.remote_session.is_some() {
        return;
    }

    let (files, mut metadata) = crate::modules::files::read_dir_with_metadata(&fs.nav.current_path);
    let mut filtered_files: Vec<_> = files
        .into_iter()
        .filter(|p| {
            let is_hidden = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with('.'))
                .unwrap_or(false);
            fs.nav.show_hidden || !is_hidden
        })
        .collect();

    filtered_files.sort_by(|a, b| {
        let meta_a = metadata.get(a);
        let meta_b = metadata.get(b);
        let is_dir_a = meta_a.map(|m| m.is_dir).unwrap_or(false);
        let is_dir_b = meta_b.map(|m| m.is_dir).unwrap_or(false);
        if is_dir_a != is_dir_b {
            return if is_dir_a {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }

        let ord = match fs.nav.sort_column {
            crate::app::FileColumn::Name => {
                let na = a
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let nb = b
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                na.cmp(&nb)
            }
            crate::app::FileColumn::Size => {
                let sa = meta_a.map(|m| m.size).unwrap_or(0);
                let sb = meta_b.map(|m| m.size).unwrap_or(0);
                sa.cmp(&sb)
            }
            crate::app::FileColumn::Modified => {
                let da = meta_a
                    .map(|m| m.modified)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let db = meta_b
                    .map(|m| m.modified)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                da.cmp(&db)
            }
            crate::app::FileColumn::Created => {
                let da = meta_a
                    .map(|m| m.created)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let db = meta_b
                    .map(|m| m.created)
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                da.cmp(&db)
            }
            crate::app::FileColumn::Permissions => {
                let pa = meta_a.map(|m| m.permissions).unwrap_or(0);
                let pb = meta_b.map(|m| m.permissions).unwrap_or(0);
                pa.cmp(&pb)
            }
        };
        if fs.nav.sort_ascending {
            ord
        } else {
            ord.reverse()
        }
    });

    fs.list.local_count = filtered_files.len();
    fs.list.files = filtered_files;
    fs.list.metadata = std::mem::take(&mut metadata);
    if fs.list.selection.selected.is_none() && !fs.list.files.is_empty() {
        fs.list.selection.selected = Some(0);
        fs.view.table_state.select(Some(0));
    }
}