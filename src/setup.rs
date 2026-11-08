#![allow(unused_imports)]

//! App setup and initialization.
//! Extracted from main.rs (Phase 4).

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashSet;

use crate::app::{App, AppEvent};
use crate::config::{fuzzy_contains, FILE_WATCH_DEBOUNCE_MS, FUZZY_SEARCH, GIT_CACHE_TTL_SECONDS, MAX_TREE_DEPTH, MPSC_CHANNEL_CAPACITY};
use crate::events;
use crate::state::FileState;
use dracon_terminal_engine::contracts::InputEvent as Event;
use tokio::sync::mpsc;

pub type PLMutex<T> = parking_lot::Mutex<T>;

pub fn setup_app() -> (
    Arc<PLMutex<App>>,
    mpsc::Sender<AppEvent>,
    mpsc::Receiver<AppEvent>,
) {
    let (tx, rx) = mpsc::channel(MPSC_CHANNEL_CAPACITY);
    let mut app = App::new();

    if let Some(state) = crate::config::load_state() {
        if !state.panes.is_empty() {
            app.panes = state.panes;
        }
        // Never trust persisted state with structure sizes: a crafted or
        // corrupt state.json could force thousands of panes/tabs at startup.
        app.panes.truncate(2);
        for pane in &mut app.panes {
            pane.tabs.truncate(crate::config::MAX_TABS);
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
                // Sanitize history indices (a corrupt state could carry an
                // out-of-bounds history_index).
                if tab.nav.history_index >= tab.nav.history.len() {
                    tab.nav.history_index = tab.nav.history.len().saturating_sub(1);
                }
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
                        crate::nav_helpers::push_history(fs, cwd);
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
        // Migration: skip stale default themes from previous eras.
        // If the saved theme matches an old default (cool/purple or warm/amber),
        // discard it so the current default (Legacy Red) takes effect.
        if let Some(theme_style) = state.theme_style {
            if theme_style != crate::ui::theme::ThemeStyle::preset_cool()
                && theme_style != crate::ui::theme::ThemeStyle::preset_warm()
            {
                crate::ui::theme::set_style_settings(theme_style);
            }
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
) -> bool {
    events::handle_event(evt, app, event_tx)
}

pub fn prime_visible_tabs(app: &mut App) {
    let expanded = app.layout.expanded_folders.clone();
    for pane in &mut app.panes {
        if let Some(fs) = pane.current_state_mut() {
            prime_local_file_state(fs, &expanded);
        }
    }
}

pub fn prime_local_file_state(
    fs: &mut crate::state::FileState,
    expanded: &HashSet<PathBuf>,
) {
    if fs.nav.remote_session.is_some() {
        return;
    }

    // Use the same tree walk as the async refresh path so startup renders
    // expanded folders with correct indentation (tree_file_depths) instead
    // of depth-0 rows until the first refresh lands.
    let mut paired: Vec<(PathBuf, u16)> = Vec::new();
    crate::tree_walk::walk_tree(
        &fs.nav.current_path,
        0,
        MAX_TREE_DEPTH,
        expanded,
        fs.nav.show_hidden,
        &mut paired,
        fs.nav.sort_column,
        fs.nav.sort_ascending,
    );
    let tree_paths: Vec<PathBuf> = paired.iter().map(|(p, _)| p.clone()).collect();
    let (_, metadata) = crate::modules::files::read_dir_recursive_meta(&tree_paths);

    fs.list.local_count = paired.len();
    fs.list.tree_file_depths = paired.iter().map(|(_, d)| *d).collect();
    fs.list.files = tree_paths;
    fs.list.metadata = metadata;
    if fs.list.selection.selected.is_none() && !fs.list.files.is_empty() {
        fs.list.selection.selected = Some(0);
        fs.view.table_state.select(Some(0));
    }
}