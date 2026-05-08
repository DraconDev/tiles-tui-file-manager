# Tiles Remote Server Architecture

## Overview

Tiles supports connecting to remote servers via SSH for file management, monitoring, and process control. This document describes the architecture and key implementation details.

## Core Components

### 1. Server Configuration (`src/servers.rs`)

**ServerConfig / RemoteBookmark**
- Stores server connection details: name, host, user, port, key_path, alias
- Persistent storage in `~/.config/tiles/servers.toml`
- Supports aliases (custom display names separate from connection name)

**Key Features:**
- Tilde expansion: `~/.ssh/key` → `/home/user/.ssh/key` using `dirs::home_dir()`
- SSH config import: Parses `~/.ssh/config` for Host, HostName, User, Port, IdentityFile
- Match directive support: `Match host <pattern>` treated as Host for import
- Duplicate detection: Prevents re-importing same server by name or host+user+port
- Key permission validation: Ensures SSH keys are ≤ 0600 (auto-fixes with `chmod 600`)

**Validation Rules:**
- Name: Required, no spaces, not "default"
- Host: Required, valid hostname or IP
- User: Required
- Port: 1-65535
- Key file: Must exist and have permissions ≤ 0600

### 2. Remote Module (`src/modules/remote.rs`)

**Connection Management:**
- `connect_ssh()`: Establishes SSH connection using ssh2 crate
- `RemoteSession` struct: Wraps ssh2::Session with connection details
- `is_binary_file()`: Probes first 8KB for null bytes to detect binaries

**File Operations:**
- `read_to_string()`: Reads remote file content
- `read_dir_with_metadata()`: Lists directory with file metadata
- `is_dir()`: Checks if path is directory
- `download_remote_file()`: Downloads binary to `/tmp/tiles_remote/` then xdg-open
- `upload_file_with_progress()`: Uploads local file via scp or base64 fallback
- `folder_size()`: Calculates directory size via `du -sb`
- `chmod()`: Changes file permissions via SSH exec
- `compute_checksums()`: Computes MD5/SHA256 via remote commands
- `diff_files()`: Computes unified diff between two files
- `create_archive()`: Creates tar.gz or zip archives

**Upload Methods:**
1. **SCP** (preferred): Uses system `scp` command with key/port options
2. **Base64 fallback**: Chunks file into base64 pieces, decodes on remote via `sh -c`

### 3. Connection Pool (`src/app.rs`)

**Pooling:**
- `remote_session_pool: HashMap<String, (RemoteSession, Instant)>`
- Stores sessions with last-used timestamp
- Cleanup: Removes stale entries older than 5 minutes
- First connection: Normal SSH handshake (~1-2s)
- Subsequent connections: Instant reuse from pool

**Health Tracking:**
- `remote_health: HashMap<String, (bool, Instant)>`
- Updated on every remote file refresh
- Footer indicator: Green (healthy), Yellow (>60s stale), Red (failed), Gray (unknown)

### 4. UI Integration (`src/ui/mod.rs`)

**Remote Indicators:**
- Tab labels: `󰒍 servername` instead of `/` when connected
- Footer: Shows `│ 󰒍 alias` with server alias
- Connection health: Colored dot (● green, ● yellow, ● red, ○ gray)

**Modals:**
- Add/Edit Remote: Form with Name, Alias, Host, User, Port, Key Path fields
- Import Servers: TOML file import with validation
- Import SSH Config: Parses `~/.ssh/config` with preview

### 5. Event System (`src/state/mod.rs`)

**Key Events:**
- `ConnectToRemote(pane_idx, bookmark_idx)`: Initiates connection
- `RemoteConnected(pane_idx, session, name)`: Connection successful
- `ReconnectRemote(bookmark_idx)`: Auto-reconnection (up to 3 retries)
- `RefreshFiles(pane_idx)`: Refreshes file listing (triggers reconnect on failure)
- `UploadToRemote(local_path, remote_path)`: File upload
- `ComputeChecksums(path)`: Calculates file hashes
- `CompareFiles(path1, path2)`: Shows diff viewer
- `CreateArchive(files, path, format)`: Creates compressed archive

### 6. Auto-Reconnection (`src/main.rs`)

**Retry Logic:**
- `retry_count` tracked per FileState
- Max 3 reconnection attempts
- Triggered when remote directory listing fails
- Shows status messages during reconnection
- Stops retrying after 3 failures

### 7. File State (`src/state/mod.rs`)

**Remote Tracking:**
- `remote_session: Option<RemoteSession>`: Active SSH session
- `bookmark_idx: Option<usize>`: Which bookmark this session belongs to
- `retry_count: u8`: Reconnection attempt counter
- `folder_sizes: HashMap<PathBuf, u64>`: Cached directory sizes

## Data Flow

### Connection Flow
```
User selects server → ConnectToRemote event → Check pool → 
[Cached] Clone session → Refresh files
[New] SSH handshake → Store in pool → Refresh files
```

### File Refresh Flow
```
RefreshFiles event → Check remote_session → 
[Remote] read_dir_with_metadata() → Update FileState.files
[Local] walk_tree() + std::fs::read_dir() → Update FileState.files
→ Trigger folder size calculation (rate-limited)
→ Update health status
```

### Upload Flow
```
User drags file to remote pane → DragDropMenu with Upload option → 
UploadToRemote event → upload_file_with_progress() → 
[SCP] System scp command
[Fallback] Base64 chunks via SSH exec → Progress callbacks → Status message
```

## Performance Optimizations

1. **Adaptive sleep**: 50ms when active, 100ms when idle (based on 500ms activity threshold)
2. **Conditional data collection**: System stats only collected when monitor view active (every 2s vs 5s)
3. **Folder size rate limiting**: Max once per 5 seconds per directory
4. **Process sort**: Only when Processes view active
5. **Git data**: Only fetched in Git/Commit views
6. **Search filter**: Pre-compute lowercase once instead of per-file

## Security Considerations

1. **SSH keys**: Validated for permissions ≤ 0600, auto-fixed when possible
2. **Tilde expansion**: Resolved at save time, not connection time
3. **Path escaping**: Single quotes escaped for SSH exec commands
4. **Binary downloads**: Limited to configurable max size (default 50MB)
5. **No password auth**: Only key-based authentication supported

## Testing

**Unit Tests (`src/servers.rs`):**
- `expand_tilde_*`: Tilde expansion behavior
- `parse_ssh_config_*`: SSH config parsing
- `parse_sample_toml`: TOML deserialization

**Integration:**
- All 53 tests pass
- Build: zero warnings

## Extension Points

1. **Native SFTP**: Replace scp/base64 with libssh2 SFTP for uploads
2. **Process tree view**: Requires upstream PPID in dracon-system
3. **Multi-server sync**: Compare files across different remote servers
4. **Remote editing**: Direct file editing via SFTP instead of download/upload

## Version History

- v10.96.0: Initial remote server support
- v10.104.0: SSH config import, duplicate detection
- v10.120.0: Connection health, key auto-fix
- v10.135.0: SFTP upload, folder sizes, file comparison, checksums
- v10.146.0+: Process monitor redesign, connection pooling, archive creation, server aliases
