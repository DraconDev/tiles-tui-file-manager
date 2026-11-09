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
        // `DirEntry::metadata()` is `lstat`-like: it reports a symlink as
        // "not a directory" even when it points at one, which made symlinked
        // directories unexpandable in the tree while the file list (which
        // follows links) showed them as directories. Resolve through the
        // shared listing helper so both views agree on which rows expand.
        let resolved = crate::modules::files::resolve_entry_metadata(&p);
        let is_dir = resolved.as_ref().map(|(_, is_dir)| *is_dir).unwrap_or(false);
        let meta = resolved.map(|(m, _)| m);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_root(tag: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("tiles-treewalk-{tag}-{unique}"))
    }

    fn name_of(p: &Path) -> String {
        p.file_name().unwrap_or_default().to_string_lossy().to_string()
    }

    /// A symlinked directory must be walkable exactly like a real one: the file
    /// list stats through the link (`is_dir == true`), so a link the user marks
    /// expanded has to list its children instead of silently staying flat.
    #[cfg(unix)]
    #[test]
    fn walk_tree_expands_symlinked_directory() {
        use std::os::unix::fs::symlink;

        let root = unique_root("symlink-expand");
        let real = root.join("real");
        std::fs::create_dir_all(&real).expect("create real dir");
        std::fs::write(real.join("inner.txt"), "payload").expect("write inner file");
        let link = root.join("linked");
        symlink(&real, &link).expect("create symlink");

        let mut collapsed = Vec::new();
        walk_tree(&root, 0, 4, &HashSet::new(), false, &mut collapsed, FileColumn::Name, true);
        assert!(
            collapsed.iter().any(|(p, d)| *d == 0 && p == &link),
            "the symlink itself is a row"
        );
        assert!(
            !collapsed.iter().any(|(p, _)| name_of(p) == "inner.txt"),
            "a collapsed symlinked dir must not list children"
        );

        let mut expanded = HashSet::new();
        expanded.insert(link.clone());
        let mut expanded_rows = Vec::new();
        walk_tree(&root, 0, 4, &expanded, false, &mut expanded_rows, FileColumn::Name, true);
        assert!(
            expanded_rows
                .iter()
                .any(|(p, d)| *d == 1 && name_of(p) == "inner.txt"),
            "an expanded symlinked dir must list its children: {:?}",
            expanded_rows
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Regression: `DirEntry::metadata()` is lstat-like, so it reported a
    /// symlinked directory as a non-directory while `read_dir_with_metadata`
    /// (which resolves links) reported a directory. Both views must agree, and
    /// the walk's directory-first ordering proves the walk sees a directory.
    #[cfg(unix)]
    #[test]
    fn walk_tree_is_dir_agrees_with_file_list_for_symlinked_dir() {
        use std::os::unix::fs::symlink;

        let root = unique_root("symlink-agree");
        let real = root.join("real");
        std::fs::create_dir_all(&real).expect("create real dir");
        // Name sorts before the link, so only directory-first ordering can
        // place the symlink ahead of it.
        std::fs::write(root.join("aaa.txt"), "file").expect("write file");
        let link = root.join("zzz-linked");
        symlink(&real, &link).expect("create symlink");

        let (files, metadata) = crate::modules::files::read_dir_with_metadata(&root);
        assert!(files.iter().any(|p| p == &link), "file list includes the link");
        assert_eq!(
            metadata.get(&link).map(|m| m.is_dir),
            Some(true),
            "file list treats a symlinked dir as a directory"
        );

        let mut rows = Vec::new();
        walk_tree(&root, 0, 4, &HashSet::new(), false, &mut rows, FileColumn::Name, true);
        let names: Vec<String> = rows.iter().map(|(p, _)| name_of(p)).collect();
        // Dirs sort first, alphabetically among themselves: `real`, then the
        // symlink (`zzz-linked`), then the plain file.
        assert_eq!(
            names,
            vec!["real".to_string(), "zzz-linked".to_string(), "aaa.txt".to_string()],
            "walk must order the symlinked dir with the dirs, matching the file list"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Broken links keep their own metadata so the row still renders — and they
    /// must never be walked into.
    #[cfg(unix)]
    #[test]
    fn walk_tree_keeps_broken_symlink_as_file_row() {
        use std::os::unix::fs::symlink;

        let root = unique_root("symlink-broken");
        std::fs::create_dir_all(&root).expect("create root");
        let broken = root.join("gone");
        symlink("missing-target", &broken).expect("create broken symlink");

        let mut expanded = HashSet::new();
        expanded.insert(broken.clone());
        let mut rows = Vec::new();
        walk_tree(&root, 0, 4, &expanded, false, &mut rows, FileColumn::Name, true);
        assert_eq!(rows, vec![(broken.clone(), 0)], "broken link is one flat row");

        let (_, metadata) = crate::modules::files::read_dir_with_metadata(&root);
        assert_eq!(
            metadata.get(&broken).map(|m| m.is_dir),
            Some(false),
            "broken link is not a directory"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Following links introduces a loop hazard; the depth cap must still
    /// terminate and bound every row.
    #[cfg(unix)]
    #[test]
    fn walk_tree_bounds_depth_when_symlink_loops_back_to_parent() {
        use std::os::unix::fs::symlink;

        let root = unique_root("symlink-cycle");
        let child = root.join("child");
        std::fs::create_dir_all(&child).expect("create child dir");
        let back = child.join("back");
        symlink(&root, &back).expect("create looping symlink");

        let mut expanded = HashSet::new();
        expanded.insert(child.clone());
        expanded.insert(back.clone());
        let mut rows = Vec::new();
        walk_tree(&root, 0, 3, &expanded, false, &mut rows, FileColumn::Name, true);

        assert!(
            rows.iter().all(|(_, d)| *d < 3),
            "no row may exceed max_depth: {:?}",
            rows.iter().map(|(p, d)| (name_of(p), *d)).collect::<Vec<_>>()
        );
        assert!(
            rows.len() < 16,
            "looping symlink must not fan out unbounded: {} rows",
            rows.len()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn walk_tree_respects_max_depth_and_hidden() {
        let root = unique_root("depth-hidden");
        std::fs::create_dir_all(root.join("sub")).expect("create sub dir");
        std::fs::write(root.join("sub/inner.txt"), "x").expect("write inner");
        std::fs::write(root.join(".hidden.txt"), "x").expect("write hidden");

        let mut expanded = HashSet::new();
        expanded.insert(root.join("sub"));

        let mut visible = Vec::new();
        walk_tree(&root, 0, 4, &expanded, false, &mut visible, FileColumn::Name, true);
        assert!(
            !visible.iter().any(|(p, _)| name_of(p) == ".hidden.txt"),
            "hidden entries are skipped when hidden is disabled"
        );
        assert!(
            visible.iter().any(|(p, d)| *d == 1 && name_of(p) == "inner.txt"),
            "expanded real dir lists children"
        );

        let mut capped = Vec::new();
        walk_tree(&root, 0, 1, &expanded, false, &mut capped, FileColumn::Name, true);
        assert!(
            capped.iter().all(|(_, d)| *d < 1),
            "depth cap stops the walk before children"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
