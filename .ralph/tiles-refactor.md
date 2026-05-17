## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. 🔲 Activate FileState sub-structs (4 defined, 70+ field migrations needed) — PARTIAL
3. ✅ Split `ui/mod.rs` (5,060 → 383 lines, 92% reduction) — DONE ✅
4. 🔲 Extract `run_tty()` event handlers into `src/handlers/` — IN PROGRESS

### Rules
- Run `cargo build && cargo test` after every change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests

---

### Phase 1 — App struct decomposition ✅
- `efa3a9e9` — App struct → 13 sub-structs

### Phase 2 — FileState decomposition 🔲 PARTIAL
- `952dec60` — FileState sub-structs defined (FileNavState, FileListState, FileViewState, FileGitState)
- **NOT YET ACTIVATED**: fields still flat on FileState, sub-structs have `#[allow(dead_code)]`

### Phase 3 — ui/mod.rs split ✅ COMPLETE
**14 modules extracted (4,672 lines):**
- `header.rs` (327), `footer.rs` (380), `debug.rs` (233), `context_menu.rs` (197),
- `monitor.rs` (730), `modals.rs` (450), `small_modals.rs` (385), `misc.rs` (266),
- `settings.rs` (667), `git_view.rs` (278), `pane.rs` (801)
- **ui/mod.rs: 5,060 → 383 lines** (pure module hub)

### Phase 4 — Event handler extraction 🔲 IN PROGRESS
**Step 1 done:**
- `8362806b` — Extract setup_app, handle_event, prime_visible_tabs, prime_local_file_state to src/setup.rs (222 lines)
- **main.rs: 1,740 → 1,502 lines** (-238)

**Remaining in main.rs:**
- `run_tty()` function: ~1,400 lines (event loop with 29 AppEvent match arms)
- 29 event handler match arms spanning lines ~290-1126 (~836 lines)
- Handler groups:
  - System events: Tick, Raw, Ui, SystemUpdated, TaskProgress, TaskFinished, StatusMsg
  - Remote events: ConnectToRemote, RemoteConnected
  - File operations: RefreshFiles, FilesChangedOnDisk, PreviewRequested, SaveFile, CreateFile, CreateFolder, Rename, Delete, TrashFile, Copy, Symlink
  - Process events: SpawnTerminal, SpawnDetached, KillProcess
  - Git events: GitHistoryUpdated, GitHistory
  - UI events: GlobalSearchUpdated, SystemMonitor, Editor, AddToFavorites

**Challenge:** Event handlers access shared mutable state (`app.lock()`), local variables (`last_self_save`, `debouncer`, `event_tx`), and spawn async tasks. Cannot simply extract match arms as functions without refactoring the state ownership model.

---

## Completed Commits (15 total)
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `952dec60` refactor(file_subtypes): define FileState sub-structs
- `6e612266` → `0313dcc0` refactor(ui): extract 14 modules from ui/mod.rs
- `8362806b` refactor(main): extract setup helpers to src/setup.rs