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
    // Stat each entry once (re-statting per comparison is O(n log n) syscalls
    // and can observe a changing filesystem mid-sort).
    struct Row {
        p: PathBuf,
        is_dir: bool,
        name_lower: String,
        size: u64,
        modified: Option<std::time::SystemTime>,
        created: Option<std::time::SystemTime>,
    }
    let mut sorted: Vec<Row> = entries.filter_map(|e| e.ok()).map(|e| {
        let p = e.path();
        let meta = e.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        Row {
            name_lower: e.file_name().to_string_lossy().to_lowercase(),
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: meta.as_ref().and_then(|m| m.modified().ok()),
            created: meta.as_ref().and_then(|m| m.created().ok()),
            is_dir,
            p,
        }
    }).collect();
    sorted.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            return if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let ordering = match sort_column {
            FileColumn::Name => a.name_lower.cmp(&b.name_lower),
            FileColumn::Size => a.size.cmp(&b.size),
            FileColumn::Modified => a.modified.cmp(&b.modified),
            FileColumn::Created => a.created.cmp(&b.created),
            _ => a.p.cmp(&b.p),
        };
        if sort_ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
    for row in sorted {
        let p = row.p;
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if !hidden && name.starts_with('.') {
            continue;
        }
        tree_files.push((p.clone(), depth));
        if row.is_dir && expanded.contains(&p) {
            walk_tree(&p, depth + 1, max_depth, expanded, hidden, tree_files, sort_column, sort_ascending);
        }
    }
}
