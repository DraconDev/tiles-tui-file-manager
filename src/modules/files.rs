use crate::app::{CommitInfo, FileMetadata, GitPendingChange};
use dracon_files::{
    FileCategory as LibFileCategory, FileCopyContract, FileInspectContract, FileSearchContract,
    FileSuitabilityContract, FsCatalog,
};
use dracon_git::{CliGitSnapshotProvider, GitPreviewContract, GitSnapshotContract};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Git data: (history, pending, branch, ahead, behind, summary, remotes, stashes)
type GitData = (
    Vec<CommitInfo>,
    Vec<GitPendingChange>,
    String,
    usize,
    usize,
    String,
    Vec<String>,
    Vec<String>,
);

fn map_metadata(meta: dracon_files::EntryMetadata) -> FileMetadata {
    FileMetadata {
        size: meta.size,
        modified: meta.modified,
        created: meta.created,
        permissions: meta.permissions,
        is_dir: meta.is_dir,
    }
}

pub fn read_dir_with_metadata(path: &Path) -> (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) {
    let mut files = Vec::new();
    let mut metadata = HashMap::new();

    let Ok(entries) = std::fs::read_dir(path) else {
        return (files, metadata);
    };

    for entry in entries.flatten() {
        let p = entry.path();
        let symlink_meta = std::fs::symlink_metadata(&p).ok();
        let target_meta = std::fs::metadata(&p).ok();
        let meta = target_meta.as_ref().or(symlink_meta.as_ref());

        files.push(p.clone());

        if let Some(m) = meta {
            let is_dir = target_meta
                .as_ref()
                .map(|tm| tm.is_dir())
                .or_else(|| symlink_meta.as_ref().map(|sm| sm.file_type().is_dir()))
                .unwrap_or(false);
            metadata.insert(
                p,
                FileMetadata {
                    size: m.len(),
                    modified: m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    created: m.created().unwrap_or(SystemTime::UNIX_EPOCH),
                    permissions: permissions_bits(m),
                    is_dir,
                },
            );
        }
    }

    (files, metadata)
}

/// Calculate the total size of a local directory in bytes by walking the tree.
pub fn folder_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = std::fs::metadata(&p) {
                if meta.is_dir() {
                    total += folder_size(&p);
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

pub fn read_dir_recursive_meta(paths: &[PathBuf]) -> (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) {
    let mut files = Vec::new();
    let mut metadata = HashMap::new();
    for path in paths {
        let p = path.clone();
        let symlink_meta = std::fs::symlink_metadata(&p).ok();
        let target_meta = std::fs::metadata(&p).ok();
        let meta = target_meta.as_ref().or(symlink_meta.as_ref());
        files.push(p.clone());
        if let Some(m) = meta {
            let is_dir = target_meta
                .as_ref()
                .map(|tm| tm.is_dir())
                .or_else(|| symlink_meta.as_ref().map(|sm| sm.file_type().is_dir()))
                .unwrap_or(false);
            metadata.insert(
                p,
                FileMetadata {
                    size: m.len(),
                    modified: m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    created: m.created().unwrap_or(SystemTime::UNIX_EPOCH),
                    permissions: permissions_bits(m),
                    is_dir,
                },
            );
        }
    }
    (files, metadata)
}

fn permissions_bits(meta: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        if meta.permissions().readonly() {
            0o444
        } else {
            0o666
        }
    }
}

pub fn get_file_category(path: &Path) -> crate::app::FileCategory {
    let catalog = FsCatalog;
    match catalog.get_file_category(path) {
        LibFileCategory::Archive => crate::app::FileCategory::Archive,
        LibFileCategory::Image => crate::app::FileCategory::Image,
        LibFileCategory::Script => crate::app::FileCategory::Script,
        LibFileCategory::Text => crate::app::FileCategory::Text,
        LibFileCategory::Document => crate::app::FileCategory::Document,
        LibFileCategory::Audio => crate::app::FileCategory::Audio,
        LibFileCategory::Video => crate::app::FileCategory::Video,
        LibFileCategory::Other => crate::app::FileCategory::Other,
    }
}

pub fn fetch_git_data(path: &Path) -> Option<GitData> {
    let provider = CliGitSnapshotProvider;
    let snapshot = provider.fetch_snapshot(path).ok().flatten()?;

    let history = snapshot
        .history
        .into_iter()
        .map(|c| CommitInfo {
            hash: c.hash,
            author: c.author,
            date: c.date,
            message: c.message,
            decorations: c.decorations,
            files_changed: c.files_changed,
            insertions: c.insertions,
            deletions: c.deletions,
            graph: String::new(),
        })
        .collect();

    let pending = snapshot
        .pending
        .into_iter()
        .map(|p| GitPendingChange {
            status: p.status,
            path: p.path,
            insertions: p.insertions,
            deletions: p.deletions,
        })
        .collect();

    // Fetch ASCII graph data separately and merge with history
    let graph_map = fetch_git_graph(path);
    let mut history: Vec<CommitInfo> = history.into_iter().map(|mut c| {
        if let Some(g) = graph_map.get(&c.hash) {
            c.graph = g.clone();
        }
        c
    }).collect();

    Some((
        history,
        pending,
        snapshot.branch,
        snapshot.ahead,
        snapshot.behind,
        snapshot.summary,
        snapshot.remotes,
        snapshot.stashes,
    ))
}

/// Fetch ASCII graph characters for each commit hash.
/// Runs `git log --graph --pretty=format:%H` and parses the graph prefix before each hash.
fn fetch_git_graph(repo_path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["--no-pager", "log", "--graph", "--pretty=format:%H", "-n", "100"])
        .output();
    
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return map,
    };
    
    for line in stdout.lines() {
        // Lines look like:
        // * abc1234def5678... (full hash after graph chars)
        // *   abc1234def5678...
        // |\  
        // | * abc1234def5678...
        // 
        // We need to find where the hash starts and extract graph prefix
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        
        // Try to find a 40-char hex hash in the line
        if let Some(hash_pos) = line.find(|c: char| c.is_ascii_hexdigit()) {
            let potential_hash = &line[hash_pos..];
            if potential_hash.len() >= 40 {
                let hash = &potential_hash[..40];
                if hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Extract graph characters (everything before the hash)
                    let graph = line[..hash_pos].to_string();
                    map.insert(hash.to_string(), graph);
                }
            }
        }
    }
    
    map
}

pub fn global_search(root: &Path, query: &str) -> (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) {
    let catalog = FsCatalog;
    match catalog.global_search(root, query) {
        Ok((files, metadata)) => (
            files,
            metadata
                .into_iter()
                .map(|(k, v)| (k, map_metadata(v)))
                .collect(),
        ),
        Err(_) => (Vec::new(), HashMap::new()),
    }
}

pub fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    let catalog = FsCatalog;
    catalog.copy_recursive(src, dst)
}

pub fn check_file_suitability(path: &Path, max_bytes: u64) -> (bool, bool, u64) {
    let catalog = FsCatalog;
    let s = catalog.check_file_suitability(path, max_bytes);
    (s.is_binary, s.is_too_large, s.size_mb)
}

/// Returns (work_dir, program, args) for running a file, or None if not runnable.
pub fn get_run_command(path: &Path) -> Option<(PathBuf, String, Vec<String>)> {
    if path.is_dir() {
        return None;
    }

    let ext = path.extension().and_then(|e| e.to_str());
    let work_dir = path.parent()?.to_path_buf();

    // Shebang detection for scripts with executable bit
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mode = meta.permissions().mode();
            let is_executable = (mode & 0o111) != 0;
            if is_executable {
                if let Ok(first_line) = std::fs::read_to_string(path) {
                    let shebang_line = first_line.lines().next()?;
                    if shebang_line.starts_with("#!") {
                        let interpreter = shebang_line
                            .trim_start_matches("#!")
                            .split_whitespace()
                            .next()?;
                        let file_str = path.to_string_lossy();
                        crate::app::log_debug(&format!(
                            "get_run_command: shebang detected for {} -> interpreter={}",
                            path.display(), interpreter
                        ));
                        return Some((work_dir, interpreter.to_string(), vec![file_str.to_string()]));
                    }
                }
            }
        }
    }

    // Rust: find Cargo.toml in ancestor directories
    if ext == Some("rs") {
        let mut dir = work_dir.clone();
        loop {
            if dir.join("Cargo.toml").exists() {
                return Some((dir, "cargo".to_string(), vec!["run".to_string()]));
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // Extension-based interpreter mapping
    let result = match ext {
        Some("sh") | Some("bash") => Some(("bash".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("zsh") => Some(("zsh".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("py") => Some(("python3".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("js") | Some("mjs") => Some(("node".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("rb") => Some(("ruby".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("pl") => Some(("perl".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("php") => Some(("php".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("lua") => Some(("lua".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("r") => Some(("Rscript".to_string(), vec![path.to_string_lossy().to_string()])),
        Some("go") => Some(("go".to_string(), vec!["run".to_string(), path.to_string_lossy().to_string()])),
        _ => {
            crate::app::log_debug(&format!(
                "get_run_command: no handler for extension={:?} path={}",
                ext,
                path.display()
            ));
            return None;
        }
    };
    
    if let Some((program, args)) = result {
        crate::app::log_debug(&format!(
            "get_run_command: extension={:?} -> program={} args={:?}",
            ext, program, args
        ));
        Some((work_dir, program, args))
    } else {
        None
    }
}

pub fn show_commit_patch(repo_path: &Path, hash: &str) -> std::io::Result<String> {
    // Bypass dracon-git's buggy implementation which incorrectly passes `--` before the hash,
    // causing git to treat the hash as a path filter instead of a commit hash.
    let out = std::process::Command::new("git")
        .args(["show", "--patch", "--stat", "--color=never", hash])
        .current_dir(repo_path)
        .env_remove("DIRENV_DIR")
        .env_remove("DIRENV_FILE")
        .env_remove("DIRENV_WATCHES")
        .env_remove("DIRENV_DIFF")
        .env("DIRENV_LOG_FORMAT", "")
        .env("GIT_HOOKS_PATH", "")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS", "")
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn show_file_diff(repo_path: &Path, file_path: &str) -> std::io::Result<String> {
    let provider = CliGitSnapshotProvider;
    provider.show_file_diff(repo_path, file_path)
}

/// Compute MD5 and SHA256 checksums for a local file.
/// Returns (md5_hex, sha256_hex) or error.
pub fn compute_checksums(path: &Path) -> std::io::Result<(String, String)> {
    let md5_out = std::process::Command::new("sh")
        .args(&["-c", &format!(
            "md5sum '{}' 2>/dev/null || md5 -q '{}' 2>/dev/null || echo ''",
            path.display(), path.display()
        )])
        .output()?;
    let md5 = String::from_utf8_lossy(&md5_out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    
    let sha_out = std::process::Command::new("sh")
        .args(&["-c", &format!(
            "sha256sum '{}' 2>/dev/null || shasum -a 256 '{}' 2>/dev/null || echo ''",
            path.display(), path.display()
        )])
        .output()?;
    let sha256 = String::from_utf8_lossy(&sha_out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    
    Ok((md5, sha256))
}

/// Compute unified diff between two local files.
/// Returns the diff output as a string.
pub fn diff_files(path_a: &Path, path_b: &Path) -> std::io::Result<String> {
    let output = std::process::Command::new("diff")
        .args(&["-u", &path_a.to_string_lossy(), &path_b.to_string_lossy()])
        .output()?;
    
    // diff returns exit code 1 when files differ, which is not an error for us
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if !stderr.is_empty() && stdout.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            stderr.to_string(),
        ));
    }
    
    if stdout.is_empty() {
        Ok("Files are identical.\n".to_string())
    } else {
        Ok(stdout.to_string())
    }
}

pub async fn create_archive(paths: &[PathBuf], dest: &Path, format: usize) -> std::io::Result<()> {
    use tokio::process::Command;
    
    match format {
        1 => {
            // ZIP format
            let mut cmd = Command::new("zip");
            cmd.arg("-r").arg(dest).args(paths);
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    String::from_utf8_lossy(&output.stderr),
                ));
            }
        }
        _ => {
            // Default to tar.gz (format 0)
            let mut cmd = Command::new("tar");
            cmd.arg("-czf").arg(dest).args(paths);
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    String::from_utf8_lossy(&output.stderr),
                ));
            }
        }
    }
    
    Ok(())
}

/// Stage a file in git
pub fn git_stage(repo_path: &Path, file_path: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["add", file_path])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

/// Unstage a file in git
pub fn git_unstage(repo_path: &Path, file_path: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["reset", "HEAD", file_path])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

/// Stage all changes
pub fn git_stage_all(repo_path: &Path) -> std::io::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["add", "."])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

/// Unstage all changes
pub fn git_unstage_all(repo_path: &Path) -> std::io::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["reset", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

/// Commit changes with a message
pub fn git_commit(repo_path: &Path, message: &str) -> std::io::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["commit", "-m", message])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn read_dir_includes_symlink_entries() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tiles-symlink-test-{unique}"));
        let target = root.join("real_ssh_dir");
        std::fs::create_dir_all(&target).expect("create target dir");
        let link = root.join(".ssh");
        symlink(&target, &link).expect("create symlink");

        let (files, metadata) = read_dir_with_metadata(&root);
        assert!(files.iter().any(|p| p == &link), "symlink should be listed");
        assert!(metadata.contains_key(&link), "symlink should have metadata");
        assert_eq!(
            metadata.get(&link).map(|m| m.is_dir),
            Some(true),
            "symlink to dir should behave as directory"
        );

        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir_all(&root);
    }
}
