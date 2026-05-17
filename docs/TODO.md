# Tiles Improvement TODO

Generated from full code review — 2026-05-17
Updated with refactor progress — 2026-05-17

---

## P0 — Architecture (blocks future velocity)

- [x] **Split `ui/mod.rs` (5,066 lines) into submodules** — DONE (commits 6e612266 → 0313dcc0)
  - [x] `ui/modals.rs` — draw_*_modal functions
  - [x] `ui/settings.rs` — draw_settings_modal + draw_*_settings
  - [x] `ui/monitor.rs` — draw_monitor_page, draw_monitor_overview, draw_monitor_applications, draw_processes_view
  - [x] `ui/git_view.rs` — draw_commit_view, parse_commit_refs, style_for_ref_label, refs_line
  - [x] `ui/file_view.rs` — draw_file_view (494 lines, extracted from pane.rs)
  - [x] `ui/git_page.rs` — draw_git_page + 3 helpers (346 lines, extracted from pane.rs)
  - [x] `ui/header.rs` — draw_global_header
  - [x] `ui/footer.rs` — draw_footer + draw_stat_bar
  - [x] `ui/context_menu.rs` — draw_context_menu
  - [x] `ui/debug.rs` — draw_debug_page
  - [x] `ui/small_modals.rs` — small modal dialogs
  - [x] `ui/misc.rs` — misc UI functions
  - [x] `ui/pane.rs` — thin dispatcher (43 lines, was 836)
  - [x] Keep `ui/mod.rs` as thin dispatcher: `pub fn draw()` calling into submodules (386 lines, was 5,060)
  - [x] `events/settings_handlers.rs` — style color, reset, preview MB (209 lines, from modals.rs)
  - [x] `events/editor_modals.rs` — replace, search, goto handlers (240 lines, from modals.rs)
  - [x] `events/modal_mouse.rs` — mouse event handling (522 lines, from modals.rs)

- [ ] **Extract `run_tty()` event loop into handler modules** — PARTIAL
  - [x] `src/setup.rs` (222 lines) — setup_app, handle_event, prime_visible_tabs, prime_local_file_state
  - [x] `src/tree_walk.rs` (61 lines) — walk_tree
  - [x] `src/events/mouse_helpers.rs` (28 lines) — fs_mouse_index, get_open_with_suggestions
  - [ ] Create `EventLoopCtx` struct to hold shared mutable state (app, event_tx, panes_needing_refresh, last_self_save, debouncer)
  - [ ] `src/handlers/files.rs` — RefreshFiles, CreateFile, CreateFolder, Delete, Trash, Copy, Move, Rename, BulkRename
  - [ ] `src/handlers/editor.rs` — Save, SaveAs, editor sync, file-watcher reload
  - [ ] `src/handlers/remote.rs` — ConnectRemote, DisconnectRemote, remote file ops
  - [ ] `src/handlers/git.rs` — GitRefresh, GitCommit, GitCheckout
  - [ ] `src/handlers/monitor.rs` — MonitorUpdate, KillProcess, SignalSelect
  - [ ] `src/handlers/clipboard.rs` — ClipboardCopy, ClipboardPaste
  - [ ] `src/handlers/settings.rs` — SaveSettings, LoadSettings, ResetSettings
  - [ ] `src/handlers/navigation.rs` — Navigate, TabSwitch, ToggleZoom, Sidebar clicks
  - [ ] Main loop becomes: `match event { ... handlers::on_xxx(app, tx).await ... }`
  - **BLOCKER:** Deep coupling to shared mutable state. 29 match arms all reference `app.lock()`, `last_self_save`, `debouncer`, `panes_needing_refresh`. Requires EventLoopCtx pattern first.

- [x] **Decompose `App` struct (~120 fields) into sub-structs** — DONE (commit efa3a9e9)
  - [x] `SidebarState` — show_side_panel, sidebar_width_percent, sidebar_bounds, sidebar_scroll_offset, sidebar_folders/favorites/recent/storage/remotes, tree_expanded_folders, tree_cache
  - [x] `MonitorState` — move SystemState here, monitor_subview, process_table_state
  - [x] `GitState` (part of FileGitState) — git_branch, git_ahead, git_behind, git_summary, git_pending, git_history, git_remotes, git_stashes
  - [x] `DragState` — drag_start_pos, drag_source, is_dragging, hovered_drop_target
  - [x] `EditorGlobalState` — editor_state, scroll_positions, replace_buffer
  - [x] `UndoState` — undo_stack, redo_stack
  - [x] `SettingsState` — settings_index, settings_section, settings_target, settings_scroll
  - [x] +6 more sub-structs (CoreState, LayoutState, MouseState, SelectionState, NavState, EditorGlobal)
  - [x] Implement `Default` for all sub-structs, use `..Default::default()` in `App::new()`

- [x] **Decompose `FileState` (~170 pub fields) into sub-structs** — DONE (commits 952dec60 + d9c8dcd3)
  - [x] `FileListState` — files, tree_file_depths, selection, columns, local_count, metadata, path_colors
  - [x] `FileGitState` — git_history, git_history_state, git_pending_state, git_branch, git_ahead, git_behind, git_pending, git_summary, git_remotes, git_stashes, git_cache_until
  - [x] `FileNavState` — current_path, remote_session, show_hidden, search_filter, search_generation, history, history_index, sort_column, sort_ascending, search_debounce_until
  - [x] `FileViewState` — preview, view_height, table_state, column_bounds, breadcrumb_bounds, breadcrumb_header_bounds, pending_select_path, file_row_bounds
  - [x] 645 field references migrated across 20+ files (Python script)

---

## P1 — Quality (prevents bugs, improves CI)

- [ ] **Fix terma clippy errors (blocks CI with `-D warnings`)**
  - [ ] `terma/src/visuals/slicer.rs` — redundant field names (`x: x` → `x`)
  - [ ] `terma/src/compositor/engine.rs` — too many arguments in `draw_rect` (introduce struct param)
  - [ ] `terma/src/compositor/engine.rs` — collapsible `else { if .. }` → `else if`

- [x] **Add tests for untested critical modules** — PARTIAL (commit 9feaff30)
  - [x] `app.rs` — 6 new tests (defaults, pane, file state, split, sidebar, shield)
  - [x] `state/mod.rs` — 6 new tests (FileState, Pane, history, AppMode)
  - [x] `config.rs` — already has 11 tests ✅
  - [ ] `events/editor.rs` — test keyboard shortcuts, undo/redo, search/replace (complex, needs editor state)
  - [x] `modules/system.rs` — 7 new tests (process_tree_depth, parse_ppid_from_stat)

- [x] **Guard the one production `unwrap()`** — DONE (commit f95873c3)
  - [x] `events/monitor.rs:63` — replaced with `if let Some(sel)` pattern

- [x] **Replace `type_complexity` suppression with named structs** — DONE (commit f95873c3)
  - [x] `ScanResult` 4-tuple in `main.rs` → defined `struct TreeScanResult { tree_files, tree_metadata, git_files, git_metadata }`
  - [x] Remove `#[allow(clippy::type_complexity)]` from `try_send_event`

---

## P2 — Hygiene (security, correctness, polish)

- [x] **Run `cargo audit` and fix vulnerabilities** — DONE (commit c81ef0a8)
  - [x] Add `cargo audit` step to CI
  - [x] Update `image` crate from 0.24 → 0.25
  - Note: 3 transitive warnings (bincode unmaintained, paste unmaintained, lru unsound) — all via ratatui/syntect, not fixable here

- [x] **Move debug log to XDG data directory** — DONE (commit 90bb96b4)
  - [x] Change `"debug.log"` → `dirs::data_local_dir().join("tiles/debug.log")`
  - [x] Create directory on first write

- [x] **Pin dependency minor versions for reproducibility** — DONE (commit c81ef0a8)
  - [x] `tokio = { version = "1.0", ... }` → `"1.41"`
  - [x] `regex = "1"` → `"1.11"`
  - [x] `base64 = "0.22.1"` — fine (pinned patch)
  - [x] `image = "0.24"` → `"0.25"`
  - [x] `parking_lot = "0.12"` → `"0.12.3"`

- [ ] **Decompose `event_helpers.rs` (1,292 lines)** — ATTEMPTED, BLOCKED
  - [ ] `src/helpers/path.rs` — resolve_relative_path, expand_tilde, path normalization
  - [ ] `src/helpers/files.rs` — file operation helpers (delete, copy, move, trash)
  - [ ] `src/helpers/navigation.rs` — folder navigation, history, selection restore
  - **BLOCKER:** navigate_back, navigate_forward, push_history called from `events/mod.rs` dispatcher → circular dependency with `event_helpers.rs`. Would require restructuring event module hierarchy.

- [x] **Add `#[must_use]` to pure functions** — DONE (commit e2f7721c)
  - [x] `fuzzy_contains`, `try_send_event`

---

## P3 — Polish (documentation, performance)

- [ ] **Add doc comments to all public API**
  - [ ] `pub fn draw()` — explain render pipeline
  - [ ] `pub fn handle_event()` — explain event dispatch
  - [ ] `pub fn try_send_event()` — explain channel semantics
  - [ ] All `pub` methods on `App`

- [ ] **Add criterion benchmarks for hot paths**
  - [ ] `draw()` — full render cycle
  - [ ] `list_path_for_filter()` — file listing + filtering
  - [ ] `fuzzy_contains()` — search matching
  - [ ] Add `[[bench]]` to Cargo.toml

- [x] **Consider `tokio` feature slim-down** — DONE (commit 099bd446)
  - [x] `features = ["full"]` → `["rt-multi-thread", "macros", "sync", "time"]`
  - [x] Reduces compile time and binary size

- [ ] **Add `cargo clippy --release` to CI for release-specific warnings**
  - [ ] Already in CI ✅ — verify it runs on all branches

- [ ] **Consider adding `cargo doc --no-deps` to CI**
  - [ ] Catches broken doc links early

---

## Refactor Stats (23 commits)

| Metric | Before | After |
|--------|--------|-------|
| ui/mod.rs | 5,060 lines | 386 lines (-92%) |
| App struct | 120 flat fields | 13 sub-structs |
| FileState struct | 35 flat fields | 4 sub-structs |
| modals.rs | 1,929 lines | 991 lines (-49%) |
| pane.rs | 836 lines | 43 lines (-95%) |
| Total modules created | 0 | 21+ |
| Tests | 54 | 54 ✅ |
| Clippy | Clean | Clean ✅ |
