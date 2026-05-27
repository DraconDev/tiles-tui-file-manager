//! File list refresh handler — extracted from main.rs event loop.
//!
//! Handles the async file tree walking, filtering, sorting, git data fetching,
//! and UI state restoration that runs after a `RefreshFiles` event.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

use crate::app::{App, AppEvent};
use crate::config::{fuzzy_contains, FUZZY_SEARCH, MAX_TREE_DEPTH};
use crate::tree_walk;

/// Result of a tree walk + git search scan.
pub(crate) struct TreeScanResult {
    pub tree_files: Vec<(PathBuf, u16)>,
    pub tree_metadata: HashMap<PathBuf, crate::state::FileMetadata>,
    pub git_files: Vec<PathBuf>,
    pub git_metadata: HashMap<PathBuf, crate::state::FileMetadata>,
}

/// Handle all pending file list refreshes.
/// Drains `panes_needing_refresh`, spawns async tasks for each pane,
/// and awaits them before continuing the event loop.
pub async fn handle_refreshes(
    panes_needing_refresh: &mut std::collections::HashSet<usize>,
    app: Arc<Mutex<App>>,
    event_tx: tokio::sync::mpsc::Sender<AppEvent>,
) {
    for pane_idx in panes_needing_refresh.drain() {
        let (path, remote, current_filter, current_generation, git_view, tree_expanded, sort_column, sort_ascending, show_hidden) = {
            let app_guard = app.lock();
            if let Some(pane) = app_guard.panes.get(pane_idx) {
                if let Some(fs) = pane.current_state() {
                    (
                        fs.nav.current_path.clone(),
                        fs.nav.remote_session.clone(),
                        fs.nav.search_filter.clone(),
                        fs.nav.search_generation,
                        matches!(app_guard.core.current_view, crate::app::CurrentView::Files | crate::app::CurrentView::Git | crate::app::CurrentView::Commit),
                        app_guard.layout.expanded_folders.clone(),
                        fs.nav.sort_column,
                        fs.nav.sort_ascending,
                        fs.nav.show_hidden,
                    )
                } else {
                    continue;
                }
            } else {
                continue;
            }
        };

        let list_path_for_filter = path.clone();

        let tx = event_tx.clone();
        let app_clone = app.clone();
        let expanded_folders = tree_expanded;
        tokio::spawn(async move {
            let list_path = path.clone();
            let list_remote = remote.clone();
            let list_filter = current_filter.clone();
            let start_generation = current_generation;
            let TreeScanResult { tree_files, tree_metadata: mut metadata, git_files: g_files, git_metadata: g_meta } =
                tokio::task::spawn_blocking(move || {
                    let t_dir = std::time::Instant::now();
                    if let Some(session) = &list_remote {
                        let _ = crate::modules::remote::read_dir_with_metadata(session, &list_path);
                    } else {
                        let _ = crate::modules::files::read_dir_with_metadata(&list_path);
                    }

                    let max_depth = MAX_TREE_DEPTH;
                    let mut tree_files: Vec<(PathBuf, u16)> = Vec::new();
                    #[allow(clippy::too_many_arguments)]
                    tree_walk::walk_tree(&list_path, 0, max_depth, &expanded_folders, show_hidden, &mut tree_files, sort_column, sort_ascending);
                    let tree_paths: Vec<PathBuf> = tree_files.iter().map(|(p, _)| p.clone()).collect();
                    let (files_meta, g_files, g_meta) = {
                        let meta = crate::modules::files::read_dir_recursive_meta(&tree_paths);
                        let trimmed_filter = list_filter.trim();
                        let g_result = if trimmed_filter.len() > 3 {
                            if let Some(session) = &list_remote {
                                crate::modules::remote::global_search(
                                    session,
                                    &list_path,
                                    trimmed_filter,
                                )
                            } else {
                                let search_root =
                                    dirs::home_dir().unwrap_or_else(|| list_path.clone());
                                crate::modules::files::global_search(&search_root, trimmed_filter)
                            }
                        } else {
                            (Vec::new(), std::collections::HashMap::new())
                        };
                        (meta.1, g_result.0, g_result.1)
                    };

                    crate::app::log_debug(&format!("read_dir+search took {:?} for {:?}", t_dir.elapsed(), list_path));
                    TreeScanResult { tree_files, tree_metadata: files_meta, git_files: g_files, git_metadata: g_meta }
                })
                .await
                .unwrap_or_else(|_| {
                    TreeScanResult {
                        tree_files: Vec::new(),
                        tree_metadata: std::collections::HashMap::new(),
                        git_files: Vec::new(),
                        git_metadata: std::collections::HashMap::new(),
                    }
                });

            {
                let t_apply = std::time::Instant::now();
                let mut app_guard = app_clone.lock();
                crate::app::log_debug(&format!("apply lock took {:?}", t_apply.elapsed()));
                if let Some(pane) = app_guard.panes.get_mut(pane_idx) {
                    if let Some(fs) = pane.current_state_mut() {
                        // RACE CONDITION CHECK: discard stale results
                        if fs.nav.search_generation != start_generation {
                            crate::app::log_debug(&format!(
                                "RefreshFiles: generation mismatch (pane={}), dropping stale results",
                                pane_idx
                            ));
                            return;
                        }

                        let mut paired: Vec<(PathBuf, u16)> = tree_files.into_iter().filter(|(p, _)| {
                            let is_hidden = p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .map(|s| s.starts_with('.'))
                                .unwrap_or(false);

                            if !fs.nav.show_hidden && is_hidden {
                                return false;
                            }

                            if !fs.nav.search_filter.is_empty() {
                                let name = p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                let matches = if FUZZY_SEARCH {
                                    fuzzy_contains(name, &fs.nav.search_filter)
                                } else {
                                    name.to_lowercase().contains(&fs.nav.search_filter.to_lowercase())
                                };
                                if !matches {
                                    return false;
                                }
                            }

                            true
                        }).collect();

                        // Search filter: include ancestor folders
                        if !fs.nav.search_filter.is_empty() {
                            use std::collections::HashSet;
                            let filter_lower = fs.nav.search_filter.to_lowercase();
                            let mut keep: HashSet<PathBuf> = HashSet::new();
                            for (p, _) in &paired {
                                let name = p.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                let matches = if FUZZY_SEARCH {
                                    fuzzy_contains(name, &fs.nav.search_filter)
                                } else {
                                    name.to_lowercase().contains(&filter_lower)
                                };
                                if matches {
                                    keep.insert(p.clone());
                                }
                            }
                            let mut keep_with_parents = keep.clone();
                            for p in &keep {
                                let mut current = p.parent();
                                while let Some(pp) = current {
                                    if pp == list_path_for_filter.as_path() {
                                        break;
                                    }
                                    keep_with_parents.insert(pp.to_path_buf());
                                    current = pp.parent();
                                }
                            }
                            let mut new_paired: Vec<(PathBuf, u16)> = Vec::new();
                            for (p, d) in paired.into_iter() {
                                if keep_with_parents.contains(&p) {
                                    new_paired.push((p, d));
                                }
                            }
                            paired = new_paired;
                        }

                        fs.list.local_count = paired.len();

                        if !g_files.is_empty() {
                            for gf in &g_files {
                                if !paired.iter().any(|(p, _)| p == gf) {
                                    paired.push((gf.clone(), 0));
                                }
                            }
                            metadata.extend(g_meta);
                        }

                        let tree_file_depths: Vec<u16> = paired.iter().map(|(_, d)| *d).collect();
                        let files: Vec<PathBuf> = paired.into_iter().map(|(p, _)| p).collect();

                        fs.list.tree_file_depths = tree_file_depths;
                        let prev_selected_path = fs.list.selection.selected
                            .and_then(|idx| fs.list.files.get(idx).cloned());
                        let was_selected_in_view = fs.list.selection.selected.map_or(false, |old_idx| {
                            let capacity = fs.view.view_height.saturating_sub(3).max(1);
                            let offset = fs.view.table_state.offset();
                            old_idx >= offset && old_idx < offset + capacity
                        });
                        fs.list.files = files;
                        fs.list.metadata = metadata;

                        if let Some(path) = prev_selected_path {
                            if let Some(new_idx) = fs.list.files.iter().position(|p| p == &path) {
                                fs.list.selection.selected = Some(new_idx);
                                fs.list.selection.anchor = Some(new_idx);
                                fs.view.table_state.select(Some(new_idx));
                                if was_selected_in_view {
                                    let capacity = fs.view.view_height.saturating_sub(3).max(1);
                                    let offset = fs.view.table_state.offset();
                                    if new_idx < offset {
                                        *fs.view.table_state.offset_mut() = new_idx;
                                    } else if new_idx >= offset + capacity {
                                        *fs.view.table_state.offset_mut() = new_idx.saturating_sub(capacity - 1);
                                    }
                                }
                            } else {
                                let max_idx = fs.list.files.len().saturating_sub(1);
                                fs.list.selection.selected = Some(max_idx);
                                fs.view.table_state.select(Some(max_idx));
                            }
                        }
                        let max_offset = fs.list.files.len().saturating_sub(fs.view.view_height.saturating_sub(3).max(1));
                        // Allow scrolling one full page past the last item
                        let allowed_max = max_offset.saturating_add(fs.view.view_height.saturating_sub(3));
                        if fs.view.table_state.offset() > allowed_max {
                            let old = fs.view.table_state.offset();
                            crate::app::log_debug(&format!("refresh CLAMP: offset {} → {} (max_off={}, allowed={})", old, allowed_max, max_offset, allowed_max));
                            *fs.view.table_state.offset_mut() = allowed_max;
                        }

                        if let Some((pending_path, pending_scroll)) = fs.view.pending_select_path.take() {
                            if let Some(idx) = fs.list.files.iter().position(|p| p == &pending_path) {
                                fs.list.selection.selected = Some(idx);
                                fs.view.table_state.select(Some(idx));
                                *fs.view.table_state.offset_mut() = pending_scroll;
                            }
                        }
                    }
                }
            }
            let _ = tx.send(AppEvent::Tick).await;

            if git_view {
                let git_path = path.clone();
                let git_remote = remote.clone();
                let app_for_git = app_clone.clone();
                let tx_for_git = tx.clone();
                let should_fetch = {
                    let app_guard = app_for_git.lock();
                    app_guard.panes
                        .get(pane_idx)
                        .and_then(|pane| pane.current_state())
                        .map(|fs| {
                            fs.git.git_cache_until
                                .map(|until| Instant::now() >= until)
                                .unwrap_or(true)
                        })
                        .unwrap_or(false)
                };
                if !should_fetch {
                    return;
                }
                tokio::spawn(async move {
                    let git_fetch_path = git_path.clone();
                    let git_data = tokio::task::spawn_blocking(move || {
                        if let Some(session) = &git_remote {
                            crate::modules::remote::fetch_git_data(session, &git_fetch_path)
                        } else {
                            crate::modules::files::fetch_git_data(&git_fetch_path)
                        }
                    })
                    .await
                    .ok()
                    .flatten();

                    let path_still_active = {
                        let app_guard = app_for_git.lock();
                        app_guard.panes
                            .get(pane_idx)
                            .and_then(|pane| pane.current_state())
                            .map(|fs| fs.nav.current_path == git_path)
                            .unwrap_or(false)
                    };
                    if !path_still_active {
                        return;
                    }

                    let active_tab_idx = {
                        let app_guard = app_for_git.lock();
                        app_guard.panes
                            .get(pane_idx)
                            .map(|p| p.active_tab_index)
                            .unwrap_or(0)
                    };

                    let (history, pending, branch, ahead, behind, summary, remotes, stashes) =
                        git_data.unwrap_or_else(|| {
                            (
                                Vec::new(),
                                Vec::new(),
                                String::new(),
                                0,
                                0,
                                String::new(),
                                Vec::new(),
                                Vec::new(),
                            )
                        });

                    let branch_opt = if branch.is_empty() { None } else { Some(branch) };
                    let summary_opt = if summary.is_empty() {
                        None
                    } else {
                        Some(summary)
                    };

                    let _ = tx_for_git
                        .send(AppEvent::GitHistoryUpdated(
                            pane_idx,
                            active_tab_idx,
                            history,
                            pending,
                            branch_opt,
                            ahead,
                            behind,
                            summary_opt,
                            remotes,
                            stashes,
                        ))
                        .await;
                });
            }
        });
    }
}
