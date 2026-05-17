## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. 🔲 Decompose `FileState` (~42 fields → 4 sub-structs) — PARTIAL (sub-structs defined, not yet activated)
3. 🔲 Split `ui/mod.rs` (5,066 lines → 8+ submodules)
4. 🔲 Extract `run_tty()` event handlers into `src/handlers/`

### Rules
- Run `cargo build && cargo test` after every major change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests
- Keep commits small and descriptive (`refactor: extract FooState sub-struct`)
- Add `#[derive(Default)]` to all new sub-structs, use `..Default::default()` in constructors

### Phase 1 — App struct decomposition ✅
1. Read `src/app.rs` fully — identify field groupings ✅
2. Create `src/state/app_subtypes.rs` with: `AppCore`, `SidebarState`, `MonitorState`, `EditorGlobalState`, `UndoState`, `SettingsState`, `LayoutState`, `OutputState`, `DragState`, `NavState`, `RemoteState`, `MouseState`, `SelectionState2` ✅
3. Move fields into sub-structs ✅
4. Add `Default` derive to each ✅ (explicit impls for AppCore, MouseState, RemoteState)
5. Rewrite `App::new()` to use explicit field initialization ✅
6. Run build, test, clippy ✅
7. Commit ✅ (commit efa3a9e9)

### Phase 2 — FileState decomposition 🔲 PARTIAL
1. Read `src/state/mod.rs` fully — identify field groupings ✅
2. Create `src/state/file_subtypes.rs` with: `FileNavState`, `FileListState`, `FileViewState`, `FileGitState` ✅
3. Move fields — NOT YET (too large, would break 100+ file references)
4. Add `Default` ✅ (explicit impls for types with Instant fields)
5. Rewrite `FileState::new()` — NOT YET
6. Run build, test, clippy — NOT YET

**Status:** Sub-structs are defined and exported but `FileState` still uses flat fields.
The sub-structs are ready for incremental migration — see `migrate_fs_fields.py` script
for field reference migration. See `docs/TODO.md` Phase 2 items.

**Migration plan (don't rush):**
- For each `fs.*` field access, update to `fs.<sub>.<field>` using the sub-struct field mapping
- Update `config.rs` serialization (clearing search_filter, files, local_count)
- Update `FileState::new()` to use sub-structs
- After migration, FileState will be 4 fields: `nav`, `list`, `view`, `git`

### Phase 3 — ui/mod.rs split
1. Create `src/ui/` submodules: `modals.rs`, `settings.rs`, `monitor.rs`, `git_view.rs`, `file_view.rs`, `header.rs`, `footer.rs`, `context_menu.rs`, `debug.rs`
2. Move one draw function group at a time, wire in via `pub use`
3. Keep `ui/mod.rs` as thin dispatcher
4. Run build, test, clippy after each move
5. Commit per submodule

### Phase 4 — event handlers extraction
1. Create `src/handlers/mod.rs`
2. Extract one handler group at a time from `run_tty()`
3. Wire back via match arms
4. Run build, test, clippy
5. Commit per group

---

## Completed Commits
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `b6f8c2d1` refactor(file_subtypes): define FileState sub-structs (not yet activated)