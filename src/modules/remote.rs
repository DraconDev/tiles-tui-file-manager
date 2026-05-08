use crate::app::{CommitInfo, FileMetadata, GitPendingChange};
use crate::state::{RemoteBookmark, RemoteSession};
use dracon_system::{
    RemoteBookmark as RemoteBookmarkContract, RemoteConnectContract, RemoteConnectRequest,
    RemoteConnection, RemoteEntryMetadata, RemoteExecContract, RemoteFsContract,
    SshRemoteConnector, SshRemoteExecProvider, SshRemoteFsProvider,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

pub fn connect_remote(bookmark: &RemoteBookmark) -> anyhow::Result<RemoteSession> {
    let connector = SshRemoteConnector;
    let request = RemoteConnectRequest {
        bookmark: RemoteBookmarkContract {
            name: bookmark.name.clone(),
            host: bookmark.host.clone(),
            user: bookmark.user.clone(),
            port: bookmark.port,
            key_path: bookmark.key_path.clone(),
        },
        timeout_ms: 8_000,
    };
    let connected = connector.connect(&request)?;
    Ok(RemoteSession {
        host: connected.host,
        user: connected.user,
        name: connected.name,
        alias: bookmark.alias.clone(),
        port: connected.port,
        key_path: connected.key_path,
    })
}

pub fn read_dir_with_metadata(
    remote: &RemoteSession,
    path: &Path,
) -> std::io::Result<(Vec<PathBuf>, HashMap<PathBuf, FileMetadata>)> {
    let provider = SshRemoteFsProvider;
    let connection = to_connection(remote);
    let (files, metadata) = provider.read_dir_with_metadata(&connection, path)?;
    Ok((
        files,
        metadata
            .into_iter()
            .map(|(k, v)| (k, map_metadata(v)))
            .collect(),
    ))
}

pub fn read_to_string(remote: &RemoteSession, path: &Path) -> std::io::Result<String> {
    let provider = SshRemoteFsProvider;
    provider.read_to_string(&to_connection(remote), path)
}

pub fn write_string(remote: &RemoteSession, path: &Path, content: &str) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.write_string(&to_connection(remote), path, content)
}

pub fn create_file(remote: &RemoteSession, path: &Path) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.create_file(&to_connection(remote), path)
}

pub fn create_dir_all(remote: &RemoteSession, path: &Path) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.create_dir_all(&to_connection(remote), path)
}

pub fn rename(remote: &RemoteSession, old: &Path, new: &Path) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.rename(&to_connection(remote), old, new)
}

pub fn remove_path(remote: &RemoteSession, path: &Path) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.remove_path(&to_connection(remote), path)
}

pub fn copy_recursive(remote: &RemoteSession, src: &Path, dst: &Path) -> std::io::Result<()> {
    let provider = SshRemoteFsProvider;
    provider.copy_recursive(&to_connection(remote), src, dst)
}

pub fn is_dir(remote: &RemoteSession, path: &Path) -> std::io::Result<bool> {
    let provider = SshRemoteFsProvider;
    provider.is_dir(&to_connection(remote), path)
}

pub fn chmod(remote: &RemoteSession, path: &Path, mode: u32) -> std::io::Result<()> {
    let path_str = path.to_string_lossy();
    let escaped = path_str.replace('\'', "'\"'\"'");
    let _ = exec_program(remote, "chmod", &[&format!("{:o}", mode), &escaped])?;
    Ok(())
}

/// Check if a remote file is binary by reading the first 8KB and looking for null bytes.
/// Returns (is_binary, size_mb) where size_mb is 0 if unknown.
pub fn is_binary_file(remote: &RemoteSession, path: &Path) -> std::io::Result<(bool, u64)> {
    let path_str = path.to_string_lossy();
    let escaped = path_str.replace('\'', "'\"'\"'");
    
    // Get file size
    let size_out = exec_program(remote, "sh", &["-c", &format!("stat -c%s '{}' 2>/dev/null || echo 0", escaped)])?;
    let size: u64 = size_out.trim().parse().unwrap_or(0);
    let size_mb = size / (1024 * 1024);
    
    // Read first 8KB and check for null bytes
    let chunk = exec_program(remote, "sh", &["-c", &format!("head -c 8192 '{}' 2>/dev/null", escaped)])?;
    let has_null = chunk.bytes().any(|b| b == 0);
    
    Ok((has_null, size_mb))
}

/// Download a remote file to a local temp path using ssh streaming.
/// Returns the local path where the file was saved.
pub fn download_remote_file(remote: &RemoteSession, path: &Path) -> std::io::Result<PathBuf> {
    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "downloaded".to_string());
    
    let tmp_dir = std::env::temp_dir().join("tiles_remote");
    std::fs::create_dir_all(&tmp_dir)?;
    
    // Sanitize filename for local filesystem
    let safe_name: String = file_name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    
    let local_path = tmp_dir.join(&safe_name);
    
    // Build ssh command
    let mut cmd = std::process::Command::new("ssh");
    cmd.arg("-o").arg("StrictHostKeyChecking=no")
       .arg("-o").arg("BatchMode=yes");
    
    if remote.port != 22 {
        cmd.arg("-p").arg(remote.port.to_string());
    }
    
    if let Some(key) = &remote.key_path {
        cmd.arg("-i").arg(key);
    }
    
    let host_spec = format!("{}@{}", remote.user, remote.host);
    let remote_path_escaped = path.to_string_lossy().replace('\'', "'\"'\"'");
    
    cmd.arg(&host_spec)
       .arg(format!("cat '{}'", remote_path_escaped));
    
    // Stream stdout to local file
    let mut child = cmd.stdout(std::process::Stdio::piped())
        .spawn()?;
    
    if let Some(mut stdout) = child.stdout.take() {
        use std::io::{Read, Write};
        let mut file = std::fs::File::create(&local_path)?;
        let mut buffer = [0u8; 8192];
        loop {
            let n = stdout.read(&mut buffer)?;
            if n == 0 { break; }
            file.write_all(&buffer[..n])?;
        }
    }
    
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("ssh download failed for {}", path.display()),
        ));
    }
    
    Ok(local_path)
}

/// Upload a local file to a remote path using scp (preferred) or base64 fallback.
/// Returns Ok(()) on success.
pub fn upload_file(remote: &RemoteSession, local_path: &Path, remote_path: &Path) -> std::io::Result<()> {
    // Try scp first (fastest, handles binary, any size)
    if let Err(_) = upload_via_scp(remote, local_path, remote_path) {
        // Fallback: base64 encoding via ssh exec (slower, ~1-2MB max practical)
        upload_via_base64(remote, local_path, remote_path)?;
    }
    Ok(())
}

/// Calculate the total size of a remote directory in bytes.
/// Uses `du -sb` for fast calculation.
pub fn folder_size(remote: &RemoteSession, path: &Path) -> std::io::Result<u64> {
    let path_str = path.to_string_lossy().replace('\'', "'\"'\"'");
    let output = exec_program(remote, "sh", &["-c", &format!(
        "du -sb '{}' 2>/dev/null | cut -f1",
        path_str
    )])?;
    let size: u64 = output.trim().parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse folder size: {}", e),
        )
    })?;
    Ok(size)
}

fn upload_via_scp(
    remote: &RemoteSession,
    local_path: &Path,
    remote_path: &Path,
) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new("scp");
    cmd.arg("-o").arg("StrictHostKeyChecking=no")
       .arg("-o").arg("BatchMode=yes");
    
    if remote.port != 22 {
        cmd.arg("-P").arg(remote.port.to_string());
    }
    
    if let Some(key) = &remote.key_path {
        cmd.arg("-i").arg(key);
    }
    
    cmd.arg(local_path);
    
    let host_spec = format!("{}@{}:{}", remote.user, remote.host, remote_path.to_string_lossy());
    cmd.arg(&host_spec);
    
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("scp upload failed: {}", stderr),
        ));
    }
    Ok(())
}

fn upload_via_base64(
    remote: &RemoteSession,
    local_path: &Path,
    remote_path: &Path,
) -> std::io::Result<()> {
    use base64::Engine;
    
    let bytes = std::fs::read(local_path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    let remote_path_escaped = remote_path.to_string_lossy().replace('\'', "'\"'\"'");
    
    // Write base64 in chunks to avoid command line length limits
    const CHUNK_SIZE: usize = 4096;
    
    // First, ensure parent directory exists
    let parent = remote_path.parent()
        .map(|p| p.to_string_lossy().replace('\'', "'\"'\"'"))
        .unwrap_or_else(|| "/tmp".to_string());
    let _ = exec_program(remote, "sh", &["-c", &format!("mkdir -p '{}'", parent)])?;
    
    // Clear target file
    let _ = exec_program(remote, "sh", &["-c", &format!("> '{}'", remote_path_escaped)])?;
    
    // Append base64 chunks
    for chunk in b64.as_bytes().chunks(CHUNK_SIZE) {
        let chunk_str = std::str::from_utf8(chunk).unwrap_or("");
        let cmd = format!(
            "printf '%s' '{}' | base64 -d >> '{}'",
            chunk_str, remote_path_escaped
        );
        let _ = exec_program(remote, "sh", &["-c", &cmd])?;
    }
    
    Ok(())
}

/// Compute MD5 and SHA256 checksums for a remote file.
/// Returns (md5_hex, sha256_hex) or error.
pub fn compute_checksums(remote: &RemoteSession, path: &Path) -> std::io::Result<(String, String)> {
    let path_str = path.to_string_lossy().replace('\'', "'\"'\"'");
    
    let md5_out = exec_program(remote, "sh", &["-c", &format!(
        "md5sum '{}' 2>/dev/null || md5 -q '{}' 2>/dev/null || echo ''",
        path_str, path_str
    )])?;
    let md5 = md5_out.split_whitespace().next().unwrap_or("").to_string();
    
    let sha_out = exec_program(remote, "sh", &["-c", &format!(
        "sha256sum '{}' 2>/dev/null || shasum -a 256 '{}' 2>/dev/null || echo ''",
        path_str, path_str
    )])?;
    let sha256 = sha_out.split_whitespace().next().unwrap_or("").to_string();
    
    Ok((md5, sha256))
}

/// Compute unified diff between two remote files.
/// Returns the diff output as a string.
pub fn diff_files(remote: &RemoteSession, path_a: &Path, path_b: &Path) -> std::io::Result<String> {
    let a_escaped = path_a.to_string_lossy().replace('\'', "'\"'\"'");
    let b_escaped = path_b.to_string_lossy().replace('\'', "'\"'\"'");
    
    let out = exec_program(remote, "sh", &["-c", &format!(
        "diff -u '{}' '{}' 2>/dev/null || diff '{}' '{}' 2>/dev/null || echo '<diff not available>'",
        a_escaped, b_escaped, a_escaped, b_escaped
    )])?;
    
    Ok(out)
}

pub fn global_search(
    remote: &RemoteSession,
    root: &Path,
    query: &str,
) -> (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) {
    let root_str = root.to_string_lossy();
    let pattern = format!("*{}*", query.replace('\'', "'\"'\"'"));
    let cmd = format!("find '{}' -type f -iname '{}' 2>/dev/null | head -n 200", root_str, pattern);
    let out = exec_program(remote, "sh", &["-c", &cmd]).unwrap_or_default();
    let files = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    (files, HashMap::new())
}

pub fn fetch_git_data(remote: &RemoteSession, path: &Path) -> Option<GitData> {
    let path_str = path.to_string_lossy();
    let cd_cmd = format!("cd '{}' && ", path_str.replace('\'', "'\"'\"'"));

    let branch = exec_program(remote, "sh", &["-c", &format!("{}git rev-parse --abbrev-ref HEAD", cd_cmd)])
        .ok()?
        .trim()
        .to_string();

    let (ahead, behind) = if let Ok(raw) = exec_program(
        remote,
        "sh",
        &[ "-c", &format!("{}git rev-list --left-right --count HEAD...@{{u}}", cd_cmd) ],
    ) {
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() == 2 {
            (parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0))
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let mut history = Vec::new();
    if let Ok(raw) = exec_program(
        remote,
        "sh",
        &[ "-c", &format!("{}git --no-pager log -n 100 --pretty=format:%H%x1f%an%x1f%ar%x1f%s%x1f%d --shortstat", cd_cmd) ],
    ) {
        let mut current: Option<CommitInfo> = None;
        for line in raw.lines() {
            if let Some(parsed) = parse_git_log_record(line) {
                if let Some(c) = current.take() {
                    history.push(c);
                }
                current = Some(parsed);
            } else if let Some(c) = current.as_mut() {
                if line.contains("changed") {
                    let (files_changed, insertions, deletions) = parse_git_shortstat(line);
                    c.files_changed = files_changed;
                    c.insertions = insertions;
                    c.deletions = deletions;
                }
            }
        }
        if let Some(c) = current {
            history.push(c);
        }
    }

    let mut pending = Vec::new();
    let mut stats_map: HashMap<String, (usize, usize)> = HashMap::new();
    if let Ok(raw) = exec_program(remote, "sh", &[ "-c", &format!("{}git diff --numstat", cd_cmd) ]) {
        for line in raw.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                stats_map.insert(
                    parts[2].to_string(),
                    (parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)),
                );
            }
        }
    }
    if let Ok(raw) = exec_program(remote, "sh", &[ "-c", &format!("{}git diff --staged --numstat", cd_cmd) ]) {
        for line in raw.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let entry = stats_map.entry(parts[2].to_string()).or_insert((0, 0));
                entry.0 += parts[0].parse::<usize>().unwrap_or(0);
                entry.1 += parts[1].parse::<usize>().unwrap_or(0);
            }
        }
    }
    if let Ok(raw) = exec_program(remote, "sh", &[ "-c", &format!("{}git status --porcelain", cd_cmd) ]) {
        for line in raw.lines() {
            if line.len() > 3 {
                let status = line[0..2].trim().to_string();
                let file = line[3..].to_string();
                let (ins, del) = stats_map.get(&file).cloned().unwrap_or((0, 0));
                pending.push(GitPendingChange {
                    status,
                    path: file,
                    insertions: ins,
                    deletions: del,
                });
            }
        }
    }

    let summary = exec_program(remote, "sh", &[ "-c", &format!("{}git diff HEAD --shortstat", cd_cmd) ])
        .unwrap_or_default()
        .trim()
        .to_string();
    let remotes = exec_program(remote, "sh", &[ "-c", &format!("{}git remote -v", cd_cmd) ])
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let stashes = exec_program(remote, "sh", &[ "-c", &format!("{}git stash list", cd_cmd) ])
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Some((
        history, pending, branch, ahead, behind, summary, remotes, stashes,
    ))
}

pub fn build_remote_terminal_command(
    remote: &RemoteSession,
    path: &Path,
    command: Option<&str>,
) -> String {
    let dest = format!("{}@{}", remote.user, remote.host);
    let path_q = shell_quote_path(path);
    let body = if let Some(cmd) = command {
        format!("cd {path_q} && {cmd}")
    } else {
        format!("cd {path_q} && exec $SHELL -l")
    };
    let body_q = escape_shell_single_quoted(&body);
    let key_part = remote
        .key_path
        .as_ref()
        .map(|p| format!(" -i {}", shell_quote_path(p)))
        .unwrap_or_default();
    format!("ssh -p {}{} {} {}", remote.port, key_part, dest, body_q)
}

pub fn show_file_diff(
    remote: &RemoteSession,
    repo_path: &Path,
    file_path: &str,
) -> std::io::Result<String> {
    let repo_str = repo_path.to_string_lossy().replace('\'', "'\"'\"'");
    let file_safe = file_path.replace('\'', "'\"'\"'");
    let out = exec_program(remote, "sh", &[
        "-c",
        &format!("cd '{}' && git diff '{}'", repo_str, file_safe),
    ])?;
    if out.trim().is_empty() {
        Ok("(No changes or file only in index)".to_string())
    } else {
        Ok(out)
    }
}

fn to_connection(remote: &RemoteSession) -> RemoteConnection {
    RemoteConnection {
        name: remote.name.clone(),
        host: remote.host.clone(),
        user: remote.user.clone(),
        port: remote.port,
        key_path: remote.key_path.clone(),
        auth_method: "session".to_string(),
    }
}

fn map_metadata(meta: RemoteEntryMetadata) -> FileMetadata {
    FileMetadata {
        size: meta.size,
        modified: meta.modified,
        created: meta.created,
        permissions: meta.permissions,
        is_dir: meta.is_dir,
    }
}

fn exec_program(remote: &RemoteSession, program: &str, args: &[&str]) -> std::io::Result<String> {
    let exec = SshRemoteExecProvider;
    exec.exec_program(&to_connection(remote), program, args)
}

fn parse_git_log_record(line: &str) -> Option<CommitInfo> {
    let parts: Vec<&str> = line.trim().split('\x1f').collect();
    if parts.len() < 5 {
        return None;
    }
    Some(CommitInfo {
        hash: parts[0].to_string(),
        author: parts[1].to_string(),
        date: parts[2].to_string(),
        message: parts[3].to_string(),
        decorations: parts[4].to_string(),
        files_changed: 0,
        insertions: 0,
        deletions: 0,
    })
}

fn parse_git_shortstat(line: &str) -> (usize, usize, usize) {
    let mut files_changed = 0usize;
    let mut insertions = 0usize;
    let mut deletions = 0usize;
    for segment in line.split(',').map(str::trim) {
        let value = segment
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        if segment.contains("file changed") || segment.contains("files changed") {
            files_changed = value;
        } else if segment.contains("insertion") {
            insertions = value;
        } else if segment.contains("deletion") {
            deletions = value;
        }
    }
    (files_changed, insertions, deletions)
}

fn shell_quote_path(path: &Path) -> String {
    escape_shell_single_quoted(&path.to_string_lossy())
}

fn escape_shell_single_quoted(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

pub fn create_archive(
    remote: &RemoteSession,
    paths: &[PathBuf],
    dest: &Path,
) -> std::io::Result<()> {
    let is_zip = dest.extension().and_then(|e| e.to_str()) == Some("zip");
    
    let path_strs: Vec<String> = paths.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    
    if is_zip {
        let args = ["-r"]
            .iter()
            .map(|s| s.to_string())
            .chain(std::iter::once(dest.to_string_lossy().to_string()))
            .chain(path_strs.into_iter())
            .collect::<Vec<_>>();
        exec_program(remote, "zip", &args.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
    } else {
        let args = ["-czf"]
            .iter()
            .map(|s| s.to_string())
            .chain(std::iter::once(dest.to_string_lossy().to_string()))
            .chain(path_strs.into_iter())
            .collect::<Vec<_>>();
        exec_program(remote, "tar", &args.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_remote() -> RemoteSession {
        RemoteSession {
            host: "example.com".to_string(),
            user: "dracon".to_string(),
            name: "test".to_string(),
            alias: None,
            port: 2222,
            key_path: Some(PathBuf::from("/home/dracon/.ssh/id_ed25519")),
        }
    }

    #[test]
    fn escape_single_quotes_for_shell() {
        let escaped = escape_shell_single_quoted("a'b");
        assert_eq!(escaped, "'a'\"'\"'b'");
    }

    #[test]
    fn build_remote_terminal_command_includes_key_and_port() {
        let remote = sample_remote();
        let cmd = build_remote_terminal_command(&remote, Path::new("/tmp/my dir"), Some("ls -la"));
        assert!(cmd.starts_with("ssh -p 2222 "));
        assert!(cmd.contains(" -i '/home/dracon/.ssh/id_ed25519' "));
        assert!(cmd.contains(" dracon@example.com "));
        assert!(cmd.contains("/tmp/my dir"));
        assert!(cmd.contains("ls -la"));
    }

    #[test]
    fn build_remote_terminal_command_defaults_to_shell_login() {
        let remote = RemoteSession {
            key_path: None,
            ..sample_remote()
        };
        let cmd = build_remote_terminal_command(&remote, Path::new("/"), None);
        assert!(cmd.contains("ssh -p 2222 "));
        assert!(cmd.contains(" dracon@example.com "));
        assert!(cmd.contains("exec $SHELL -l"));
    }

    #[test]
    fn parse_git_log_record_reads_all_fields() {
        let line = "abc123\x1fAda\x1f2 days ago\x1ffeat: add parser\x1f(HEAD -> main)";
        let parsed = parse_git_log_record(line).expect("record should parse");
        assert_eq!(parsed.hash, "abc123");
        assert_eq!(parsed.author, "Ada");
        assert_eq!(parsed.date, "2 days ago");
        assert_eq!(parsed.message, "feat: add parser");
        assert_eq!(parsed.decorations, "(HEAD -> main)");
    }

    #[test]
    fn parse_git_shortstat_extracts_counts() {
        let (files, ins, del) =
            parse_git_shortstat("3 files changed, 21 insertions(+), 8 deletions(-)");
        assert_eq!(files, 3);
        assert_eq!(ins, 21);
        assert_eq!(del, 8);
    }
}
