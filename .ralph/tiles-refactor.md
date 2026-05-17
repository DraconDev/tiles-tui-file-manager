## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. 🔲 Activate FileState sub-structs (4 defined, ~675 field references to migrate) — PARTIAL
3. ✅ Split `ui/mod.rs` (5,060 → 383 lines, 92% reduction) — DONE ✅
4. 🔲 Extract `run_tty()` event handlers — IN PROGRESS

### Rules
- Run `cargo build && cargo test` after every change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests

---

### Phase 1 — App struct decomposition ✅
### Phase 3 — ui/mod.rs split ✅ COMPLETE (14 modules, 4,672 lines extracted)

### Phase 2 — FileState decomposition 🔲 PARTIAL
- `952dec60` — FileState sub-structs defined (FileNavState, FileListState, FileViewState, FileGitState)
- **NOT YET ACTIVATED**: fields still flat on FileState, sub-structs have `#[allow(dead_code)]`
- **Migration required**: ~675 field references across src/ need updating
  - Most common patterns: `fs.current_path` → `fs.nav.current_path`, `fs.files` → `fs.list.files`, etc.
  - FileState struct has `#[serde(skip)]` on 20+ fields — serialization is a concern
  - Variable names for FileState: `fs` (most common), `self` (in impl), `tab`, `file_state`
  - Safest approach: write a Python script that handles each variable pattern separately

### Phase 4 — Event handler extraction 🔲 IN PROGRESS
**Completed:**
- `8362806b` — setup.rs (setup_app, handle_event, prime_visible_tabs, prime_local_file_state) — 222 lines
- `58dc9cac` — tree_walk.rs (walk_tree function) — 61 lines
- **main.rs: 1,740 → 1,460 lines** (-280 lines)

**Remaining in main.rs (1,460 lines):**
- `run_tty()` event loop: ~1,340 lines
  - Setup: 205 lines
  - Event match block: 786 lines (29 AppEvent match arms)
  - Post-match refresh: 330 lines
  - Final draw: 14 lines
- Event handler coupling analysis:
  - Score 0 (trivial): Ui, SpawnTerminal, SpawnDetached, KillProcess
  - Score 1 (easy): CreateFile, CreateFolder, Rename, Delete, TrashFile — only need app.lock() + event_tx
  - Score 2 (moderate): SystemUpdated, RemoteConnected, RefreshFiles, Symlink, GitHistoryUpdated, etc.
  - Score 3-6 (hard): Raw, AddToFavorites, FilesChangedOnDisk, SaveFile, Copy, Tick, ConnectToRemote, PreviewRequested
- **Challenge**: Handlers access shared mutable state (app.lock(), last_self_save, debouncer, panes_needing_refresh). Simple function extraction requires passing many parameters.
- **Possible approach**: Create an `EventLoopCtx` struct holding shared state, with handler methods.

---

## Completed Commits (17 total)
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `952dec60` refactor(file_subtypes): define FileState sub-structs
- `6e612266` → `0313dcc0` refactor(ui): extract 14 modules from ui/mod.rs
- `8362806b` refactor(main): extract setup helpers to src/setup.rs
- `58dc9cac` refactor(main): extract walk_tree to src/tree_walk.rs