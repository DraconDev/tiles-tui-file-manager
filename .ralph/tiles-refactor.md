## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. Decompose `App` struct (~120 fields → 7 sub-structs)
2. Decompose `FileState` (~170 fields → 4 sub-structs)
3. Split `ui/mod.rs` (5,066 lines → 8+ submodules)
4. Extract `run_tty()` event handlers into `src/handlers/`

### Rules
- Run `cargo build && cargo test` after every major change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests
- Keep commits small and descriptive (`refactor: extract FooState sub-struct`)
- Add `#[derive(Default)]` to all new sub-structs, use `..Default::default()` in constructors

### Phase 1 — App struct decomposition (start here, smallest scope, quick wins)
1. Read `src/app.rs` fully — identify field groupings
2. Create `src/state/app_subtypes.rs` with: `SidebarState`, `MonitorState`, `GitState`, `DragState`, `EditorGlobalState`, `UndoState`, `SettingsState`
3. Move fields into sub-structs
4. Add `Default` derive to each
5. Rewrite `App::new()` to use `..Default::default()`
6. Run build, test, clippy
7. Commit

### Phase 2 — FileState decomposition
1. Read `src/state/mod.rs` fully — identify field groupings
2. Add sub-structs: `FileListState`, `GitViewState`, `NavigationState`, `ViewLayoutState`
3. Move fields, add `Default`
4. Rewrite `FileState::new()` to use `..Default::default()`
5. Run build, test, clippy
6. Commit

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