# Tiles Code Review - Bug Fixes & Improvements

## Objective

Conduct a comprehensive post-fix review of the Tiles codebase to identify remaining bugs, performance issues, memory leaks, and architectural improvements.

## Context

The immediate bug (servers not showing in sidebar) was caused by two issues:
1. `FileState.loading` missing `#[serde(default)]` — old `state.json` files failed to deserialize
2. `load_servers()` placed inside `if let Some(state)` — when state loading failed, servers were never populated

Both issues have been fixed. This review identifies **remaining** issues.

---

## Severity Legend

- **CRITICAL**: Data loss, crash, or security vulnerability
- **HIGH**: Significant performance degradation or resource exhaustion
- **MEDIUM**: Functional bug or maintainability issue
- **LOW**: Code quality or minor inefficiency

---

## Findings

### CRITICAL

#### 1. Non-Atomic State Save (`config.rs:193`)

**Location**: `src/config.rs:193`

**Issue**: `save_state()` uses `fs::write(state_path, json)?` which is not atomic. If the application crashes or is killed during the write, `state.json` will be partially written/corrupted. On next startup, `load_state()` will return `None`, triggering the same class of bug we just fixed.

**Impact**: Data loss of all user state (pane layouts, favorites, settings).

**Verification**: Kill -9 the process during `save_state` and observe corrupted `state.json`.

---

### HIGH

#### 2. Unbounded `checksum_cache` Growth (`app.rs:109`)

**Location**: `src/app.rs:109`, `src/main.rs:1100`

**Issue**: `checksum_cache: HashMap<PathBuf, (String, String)>` stores MD5/SHA256 checksums for files. Entries are only removed when a file is explicitly deleted via the "Properties" modal (`events/modals.rs:507`). There is no TTL, size limit, or periodic cleanup.

**Impact**: Long-running sessions or users who compute checksums on many files will see unbounded memory growth. Checksum strings are ~64 bytes each + PathBuf overhead.

**Verification**: Compute checksums on thousands of files and observe memory growth.

---

#### 3. Every Remote Operation Creates a New SSH Connection

**Location**: `src/modules/remote.rs:695-704`, `src/modules/remote.rs:46-100`

**Issue**: The `remote_session_pool` in `app.rs:135` caches `RemoteSession` structs (metadata: host, user, name, etc.), NOT actual SSH connection handles. Every call to `read_dir_with_metadata`, `read_to_string`, `exec_program`, etc. calls `to_connection()` and the `SshRemoteFsProvider`/`SshRemoteExecProvider` creates a fresh TCP connection + SSH handshake + authentication.

**Impact**: Severe latency on every remote operation (1-2s per operation). High connection churn on remote servers. The "connection pooling" is misleading — it only avoids re-parsing server config.

**Code flow**:
```
read_dir_with_metadata() -> to_connection() -> SshRemoteFsProvider.read_dir_with_metadata() -> NEW TCP + SSH handshake
```

**Verification**: Add logging around `TcpStream::connect` in `upload_via_sftp` (which uses ssh2 directly) and observe it fires on every remote file listing.

---

#### 4. `upload_via_sftp` Creates Full SSH Session Per File

**Location**: `src/modules/remote.rs:202-287`

**Issue**: This function establishes a complete TCP connection, SSH handshake, key authentication, and SFTP session for **every single file upload**. For batch uploads, this is extremely inefficient.

**Impact**: Uploading N files = N SSH connections. With 8s timeout and 1-2s handshake, upload throughput is severely limited.

**Note**: The `upload_file_with_progress` function at line 289 likely calls this.

---

### MEDIUM

#### 5. `last_path` in ServerConfig is Write-Only

**Location**: `src/servers.rs:44`, `src/main.rs:454`, `src/event_helpers.rs:525`

**Issue**: `ServerConfig.last_path` stores the last visited path per server. It is:
- Read when creating a new file/folder via sidebar remote context menu (`event_helpers.rs:525`)
- **Never updated** after navigation — `ConnectToRemote` always sets `current_path = PathBuf::from("/")` (`main.rs:454`)
- Never persisted back to `servers.toml`

**Impact**: Users always connect to `/` even if they previously navigated to `/var/log`. The `last_path` field is essentially dead data.

---

#### 6. `remote_health` HashMap Grows Without Pruning

**Location**: `src/app.rs:107`, `src/main.rs:1726,1746`

**Issue**: `remote_health` tracks per-server health status. Entries are inserted on every remote file refresh but never removed. If users connect to many different servers over time, this map grows indefinitely.

**Impact**: Minimal memory (bool + Instant per entry), but indicates a pattern of unbounded growth.

---

#### 7. `folder_memory` Unbounded Growth

**Location**: `src/app.rs:63`, `src/event_helpers.rs:980`

**Issue**: `folder_memory` stores (index, scroll) for visited folders to restore position on return. No size limit or eviction policy.

**Impact**: Users who navigate extensively will see memory growth. Each entry is a PathBuf + two usizes.

---

#### 8. `AppEvent::GitHistoryUpdated` is Enormous

**Location**: `src/state/mod.rs:43-54`

**Issue**: This enum variant contains:
- `Vec<CommitInfo>` (could be 100+ commits)
- `Vec<GitPendingChange>`
- `Vec<String>` (remotes)
- `Vec<String>` (stashes)
- Multiple `String` and `usize` fields

The `AppEvent` enum discriminant size is determined by the largest variant. This makes every `AppEvent` allocation large (hundreds of bytes on the stack/heap), causing:
- Increased channel memory usage
- Cache pressure when matching events
- Potential for stack overflow in deeply nested calls

**Impact**: Every `AppEvent` sent through the channel allocates a large object even for simple events like `Tick`.

---

#### 9. `try_recv()` Event Loop Can Spike Frame Time

**Location**: `src/main.rs:386`

**Issue**: The main loop uses `while let Ok(event) = event_rx.try_recv()` which drains ALL pending events before drawing. During event bursts (rapid key input, file watcher events, git updates), this can process hundreds of events in a single frame, causing UI lag.

**Impact**: No frame time budget enforcement. Event processing can starve rendering.

---

#### 10. `fuzzy_contains` Allocates on Every Call

**Location**: `src/config.rs:34-50`

**Issue**: This function allocates two new `String`s (`.to_lowercase()`) for every call. If used in tight loops (search filtering), this is expensive.

**Impact**: Search performance degradation on large file lists.

---

### LOW

#### 11. `path_colors` in `FileState` is Dead Code

**Location**: `src/state/mod.rs:358-360`

**Issue**: The field is marked `#[allow(dead_code)]` and `#[serde(skip)]`. It is never read or written in the codebase.

**Impact**: Wasted memory per tab (HashMap overhead).

---

#### 12. `remote_bookmarks` in `PersistentState` is Always Empty

**Location**: `src/config.rs:64`, `src/main.rs:139`

**Issue**: `remote_bookmarks` is serialized as an empty vector since migration to `servers.toml`. It adds noise to `state.json` and the struct.

---

#### 13. Debug Log Rotation is Fragile

**Location**: `src/app.rs:545-568`

**Issue**: The rotation logic:
```rust
fs::rename(path, "debug.log.1");
fs::remove_file("debug.log.2");
fs::rename("debug.log.1", "debug.log.2");
```

This can lose `debug.log.1` if it exists when rotation triggers. No error handling on rename/remove failures.

---

#### 14. Shell Escaping is Incomplete

**Location**: `src/modules/remote.rs:800-802`

**Issue**: `escape_shell_single_quoted` only handles single quotes. It does not handle:
- Backslashes before single quotes
- ANSI-C quoting (`$'...'`)
- Newlines or other control characters

**Impact**: Remote paths with certain special characters may cause command injection or unexpected behavior.

---

## Implementation Plan

### Phase 1: Critical & High Priority

- [ ] **Fix 1: Atomic state saves** — Write to temp file + atomic rename in `save_state()` (`config.rs:193`)
  - Rationale: Prevents corruption and data loss on crash
  - Approach: `fs::write(tmp_path, json)?; fs::rename(tmp_path, state_path)?`

- [ ] **Fix 2: Bound `checksum_cache` size** — Add LRU eviction or periodic clear
  - Rationale: Prevents unbounded memory growth
  - Approach: Either wrap in `lru::LruCache` or clear on every Nth insert

- [ ] **Fix 3: Document connection pooling limitation** — Add code comments explaining `remote_session_pool` only caches metadata
  - Rationale: Prevents future developers from misunderstanding the architecture
  - Approach: Document in `app.rs` and `modules/remote.rs`

- [ ] **Fix 4: Add `last_path` persistence** — Save current path to `ServerConfig` and write back to `servers.toml` on disconnect/navigate
  - Rationale: Makes the field actually useful
  - Approach: Update `last_path` when navigating remote dirs, save on app exit

### Phase 2: Medium Priority

- [ ] **Fix 5: Prune `remote_health` and `folder_memory`** — Add size limits or TTL-based eviction
  - Rationale: Consistent resource management
  - Approach: `retain()` with age threshold or hard cap

- [ ] **Fix 6: Optimize `fuzzy_contains`** — Use case-insensitive comparison without allocation
  - Rationale: Reduces GC pressure in search
  - Approach: `text.chars().flat_map(|c| c.to_lowercase())` vs pattern chars

- [ ] **Fix 7: Add frame time budget to event loop** — Limit events processed per frame
  - Rationale: Prevents UI lag during event bursts
  - Approach: Process max 50-100 events per frame, leave rest for next frame

- [ ] **Fix 8: Box large `AppEvent` variants** — Wrap `GitHistoryUpdated` data in `Box`
  - Rationale: Reduces enum size and channel memory pressure
  - Approach: `GitHistoryUpdated(Box<GitHistoryData>)`

### Phase 3: Low Priority / Polish

- [ ] **Fix 9: Remove dead `path_colors` field** — Delete from `FileState`
  - Rationale: Cleaner code, reduced memory

- [ ] **Fix 10: Remove empty `remote_bookmarks` from state** — Exclude from serialization or remove field
  - Rationale: Cleaner state.json

- [ ] **Fix 11: Fix debug log rotation** — Use proper rotate pattern
  - Rationale: Preserve log history correctly

- [ ] **Fix 12: Improve shell escaping** — Use a proper shell escaping crate or whitelist approach
  - Rationale: Security hardening for remote paths

---

## Verification Criteria

- [ ] State save survives `kill -9` without corruption
- [ ] `checksum_cache` memory stabilizes after computing checksums on 1000+ files
- [ ] Comment at `remote_session_pool` declaration clearly states "metadata only, not SSH connection handles"
- [ ] Remote connections remember last path after app restart
- [ ] Event loop processes no more than 100 events per frame under synthetic burst load
- [ ] `cargo test` continues to pass (all 53 tests)

---

## Alternative Approaches

### For SSH Connection Reuse (Fix 3)

1. **SFTP Session Pooling**: Cache actual `ssh2::Session` + `ssh2::Sftp` objects in `remote_session_pool` instead of just metadata. This requires managing session lifetime and re-authentication on expiration.
   - Trade-off: Significant complexity increase, requires `ssh2` types to be `Send`/`Sync` safe.

2. **Connection-per-Operation (Current)**: Keep current architecture but add aggressive connection reuse within a single operation batch (e.g., directory listing + metadata fetching in one SSH session).
   - Trade-off: Less latency improvement but much simpler to implement.

### For `checksum_cache` (Fix 2)

1. **LRU Cache**: Use `lru` crate for automatic eviction.
   - Trade-off: Extra dependency, but battle-tested.

2. **Simple Cap**: Clear entire cache when it exceeds N entries.
   - Trade-off: Zero dependencies, but loses all cached checksums at once.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| State corruption on crash | High | High | Fix 1 (atomic writes) |
| Memory exhaustion (long sessions) | Medium | Medium | Fixes 2, 5, 7 (cache bounds) |
| Remote performance degradation | High | Medium | Document limitation; consider session pooling |
| Event loop lag | Medium | Medium | Fix 7 (frame budget) |
| Shell injection via paths | Low | High | Fix 12 (proper escaping) |
