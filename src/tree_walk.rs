#![allow(clippy::too_many_arguments)]

//! Tree walking for directory expansion.
//! Extracted from main.rs (Phase 4).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::state::FileColumn;

/// Recursively walk expanded directories, collecting (path, depth) pairs
/// sorted by the given column and order within each directory level.
pub fn walk_tree(
    path: &Path,
    depth: u16,
    max_depth: u16,
    expanded: &HashSet<PathBuf>,
    hidden: bool,
    tree_files: &mut Vec<(PathBuf, u16)>,
    sort_column: FileColumn,
    sort_ascending: bool,
) {
    if depth >= max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else { return };
    let mut sorted: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    sorted.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        if a_is_dir != b_is_dir {
            return if a_is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let ordering = match sort_column {
            FileColumn::Name => {
                let na = a.file_name().to_string_lossy().to_lowercase();
                let nb = b.file_name().to_string_lossy().to_lowercase();
                na.cmp(&nb)
            }
            FileColumn::Size => {
                let sa = a.path().metadata().map(|m| m.len()).unwrap_or(0);
                let sb = b.path().metadata().map(|m| m.len()).unwrap_or(0);
                sa.cmp(&sb)
            }
            FileColumn::Modified | FileColumn::Created => {
                let da = a.path().metadata().ok().and_then(|m| m.modified().ok());
                let db = b.path().metadata().ok().and_then(|m| m.modified().ok());
                da.cmp(&db)
            }
            _ => a.file_name().cmp(&b.file_name()),
        };
        if sort_ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
    for entry in sorted {
        let p = entry.path();
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if !hidden && name.starts_with('.') {
            continue;
        }
        tree_files.push((p.clone(), depth));
        if p.is_dir() && expanded.contains(&p) {
            walk_tree(&p, depth + 1, max_depth, expanded, hidden, tree_files, sort_column, sort_ascending);
        }
    }
}
