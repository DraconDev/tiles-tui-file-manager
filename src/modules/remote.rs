use crate::app::{CommitInfo, FileMetadata, GitPendingChange};
use crate::state::{RemoteBookmark, RemoteSession};
use dracon_system_lib::{
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

pub fn global_search(
    remote: &RemoteSession,
    root: &Path,
    query: &str,
) -> (Vec<PathBuf>, HashMap<PathBuf, FileMetadata>) {
    let root_str = escape_shell_single_quoted(&root.to_string_lossy());
    let pattern = format!(
        "*{}*",
        query.replace('\'', "'\"'\"'")
    );
    let cmd = format!("find {} -type f -iname {} 2>/dev/null | head -n 200", root_str, escape_shell_single_quoted(&pattern));
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
                // Rename lines look like `1 0 old.ts => src/app/config.ts`:
                // key by the post-rename name so status lookups hit.
                let key = parts[2..]
                    .join(" ")
                    .rsplit(" => ")
                    .next()
                    .unwrap_or(parts[2])
                    .to_string();
                stats_map.insert(
                    key,
                    (parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)),
                );
            }
        }
    }
    if let Ok(raw) = exec_program(remote, "sh", &[ "-c", &format!("{}git diff --staged --numstat", cd_cmd) ]) {
        for line in raw.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let key = parts[2..]
                    .join(" ")
                    .rsplit(" => ")
                    .next()
                    .unwrap_or(parts[2])
                    .to_string();
                let entry = stats_map.entry(key).or_insert((0, 0));
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
                // Rename entries list "old -> new"; numstat is keyed by the new name.
                let lookup = file
                    .rsplit(" -> ")
                    .next()
                    .unwrap_or(&file);
                let (ins, del) = stats_map.get(lookup).cloned().unwrap_or((0, 0));
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
    // The destination is interpolated into a `sh -c` command string: never let
    // a crafted bookmark inject extra ssh options (`-o ProxyCommand=...`) or
    // shell metacharacters. Validate + quote the whole token.
    if !is_safe_ssh_identity(&remote.user, &remote.host) {
        return "echo 'Invalid SSH destination'".to_string();
    }
    let dest = escape_shell_single_quoted(&format!("{}@{}", remote.user, remote.host));
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

/// A bookmark user/host is safe to interpolate into a shell command when it
/// contains only typical identity characters and never a shell metacharacter
/// or a leading `-` (ssh would parse it as an option). `@` and `:` are
/// rejected in both fields: ssh splits the destination at the *last* `@`,
/// so a crafted value like `user@attacker` inside the host (or `a@b` in the
/// user) would silently redirect the connection to a different host than
/// the bookmark claims.
fn is_safe_ssh_identity(user: &str, host: &str) -> bool {
    let safe = |s: &str| {
        !s.is_empty()
            && !s.starts_with('-')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._".contains(c))
    };
    safe(user) && safe(host)
}

fn shell_quote_path(path: &Path) -> String {
    escape_shell_single_quoted(&path.to_string_lossy())
}

fn escape_shell_single_quoted(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_remote() -> RemoteSession {
        RemoteSession {
            host: "example.com".to_string(),
            user: "dracon".to_string(),
            name: "test".to_string(),
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
        assert!(cmd.contains("'dracon@example.com'"));
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
        assert!(cmd.contains("'dracon@example.com'"));
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
    fn build_remote_terminal_command_rejects_unsafe_destination() {
        let mut remote = sample_remote();
        remote.host = "-oProxyCommand=touch /tmp/pwned".to_string();
        let cmd = build_remote_terminal_command(&remote, Path::new("/"), None);
        assert!(!cmd.starts_with("ssh"), "got: {}", cmd);
        assert!(cmd.contains("Invalid SSH destination"));
    }

    #[test]
    fn build_remote_terminal_command_rejects_metacharacters() {
        let mut remote = sample_remote();
        remote.user = "user; rm -rf /".to_string();
        let cmd = build_remote_terminal_command(&remote, Path::new("/"), None);
        assert!(!cmd.starts_with("ssh"), "got: {}", cmd);
    }

    #[test]
    fn build_remote_terminal_command_rejects_at_and_colon() {
        // ssh splits the destination at the LAST @: a bookmark claiming
        // user "victim@attacker" or host "host:2222" would connect to a
        // different host than displayed. Both fields must reject @ and :
        // (and the now-pointless IPv6-literal brackets, which existed only
        // to delimit colon-bearing addresses — they fail closed instead of
        // relying on ssh's bracket parsing).
        for (user, host) in [
            ("victim@attacker", "real-host"),
            ("victim", "host@attacker"),
            ("victim", "host:2222"),
            ("vic:tim", "host"),
            ("victim", "[fd00::1]"),
        ] {
            let mut remote = sample_remote();
            remote.user = user.to_string();
            remote.host = host.to_string();
            let cmd = build_remote_terminal_command(&remote, Path::new("/"), None);
            assert!(!cmd.starts_with("ssh"), "user={user} host={host} must be rejected, got: {cmd}");
        }
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
