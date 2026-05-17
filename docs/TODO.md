# Tiles Improvement TODO

Generated from full code review — 2026-05-17

---

## P0 — Architecture (blocks future velocity)

- [ ] **Split `ui/mod.rs` (5,066 lines) into submodules**
  - [ ] `ui/modals.rs` — draw_*_modal functions (drag_drop, hotkeys, open_with, rename, new_file, new_folder, bulk_rename, save_as, delete, properties, command_palette, import_servers, add_remote, style_color, reset_settings)
  - [ ] `ui/settings.rs` — draw_settings_modal + draw_*_settings (shortcuts, column, tab, general, style, remote)
  - [ ] `ui/monitor.rs` — draw_monitor_page, draw_monitor_overview, draw_monitor_applications, draw_processes_view
  - [ ] `ui/git_view.rs` — draw_git_page, draw_commit_view, parse_commit_refs, style_for_ref_label, refs_line
  - [ ] `ui/file_view.rs` — draw_file_view, draw_stat_bar, draw_main_stage
  - [ ] `ui/header.rs` — draw_global_header
  - [ ] `ui/footer.rs` — draw_footer
  - [ ] `ui/context_menu.rs` — draw_context_menu
  - [ ] `ui/debug.rs` — draw_debug_page
  - [ ] Keep `ui/mod.rs` as thin dispatcher: `pub fn draw()` calling into submodules

- [ ] **Extract `run_tty()` event loop into handler modules**
  - [ ] Create `src/handlers/mod.rs` with dispatch logic
  - [ ] `src/handlers/files.rs` — RefreshFiles, CreateFile, CreateFolder, Delete, Trash, Copy, Move, Rename, BulkRename
  - [ ] `src/handlers/editor.rs` — Save, SaveAs, editor sync, file-watcher reload
  - [ ] `src/handlers/remote.rs` — ConnectRemote, DisconnectRemote, remote file ops
  - [ ] `src/handlers/git.rs` — GitRefresh, GitCommit, GitCheckout
  - [ ] `src/handlers/monitor.rs` — MonitorUpdate, KillProcess, SignalSelect
  - [ ] `src/handlers/clipboard.rs` — ClipboardCopy, ClipboardPaste
  - [ ] `src/handlers/settings.rs` — SaveSettings, LoadSettings, ResetSettings
  - [ ] `src/handlers/navigation.rs` — Navigate, TabSwitch, ToggleZoom, Sidebar clicks
  - [ ] Main loop becomes: `match event { ... handlers::on_xxx(app, tx).await ... }`

- [ ] **Decompose `App` struct (~120 fields) into sub-structs**
  - [ ] `SidebarState` — show_side_panel, sidebar_width_percent, sidebar_bounds, sidebar_scroll_offset, sidebar_folders/favorites/recent/storage/remotes, tree_expanded_folders, tree_cache
  - [ ] `MonitorState` — move SystemState here, monitor_subview, process_table_state
  - [ ] `GitState` — git_branch, git_ahead, git_behind, git_summary, git_pending, git_history, git_remotes, git_stashes (from FileState)
  - [ ] `DragState` — drag_start_pos, drag_source, is_dragging, hovered_drop_target
  - [ ] `EditorGlobalState` — editor_state, scroll_positions, replace_buffer
  - [ ] `UndoState` — undo_stack, redo_stack
  - [ ] `SettingsState` — settings_index, settings_section, settings_target, settings_scroll
  - [ ] Implement `Default` for all sub-structs, use `..Default::default()` in `App::new()`

- [ ] **Decompose `FileState` (~170 pub fields) into sub-structs**
  - [ ] `FileListState` — files, tree_files, tree_file_depths, filtered_indices, selected_index, scroll_offset
  - [ ] `GitViewState` — git_history, git_history_state, git_pending_state, git_branch, git_ahead, git_behind, git_pending, git_summary, git_remotes, git_stashes
  - [ ] `NavigationState` — history, history_index, search_filter, search_generation, show_hidden, sort_column, sort_ascending
  - [ ] `ViewLayoutState` — column_bounds, breadcrumb_bounds, breadcrumb_header_bounds, view_height, table_state

---

## P1 — Quality (prevents bugs, improves CI)

- [ ] **Fix terma clippy errors (blocks CI with `-D warnings`)**
  - [ ] `terma/src/visuals/slicer.rs` — redundant field names (`x: x` → `x`)
  - [ ] `terma/src/compositor/engine.rs` — too many arguments in `draw_rect` (introduce struct param)
  - [ ] `terma/src/compositor/engine.rs` — collapsible `else { if .. }` → `else if`

- [ ] **Add tests for untested critical modules**
  - [ ] `app.rs` — test App::new() default state, tab management, pane switching
  - [ ] `config.rs` — test save/load round-trip, SSH config parsing, settings.toml parsing
  - [ ] `state/mod.rs` — test AppMode transitions, FileState navigation history
  - [ ] `events/editor.rs` — test keyboard shortcuts, undo/redo, search/replace
  - [ ] `modules/system.rs` — test process parsing, disk stats formatting

- [ ] **Guard the one production `unwrap()`**
  - [ ] `events/monitor.rs:63` — `app.process_table_state.selected().unwrap()` → use `if let Some(idx) = ...` or `unwrap_or(0)`

- [ ] **Replace `type_complexity` suppression with named structs**
  - [ ] `ScanResult` 4-tuple in `main.rs` → define `struct ScanResult { entries, metadata, tree_files, tree_metadata }`
  - [ ] Remove `#[allow(clippy::type_complexity)]` from `try_send_event`

---

## P2 — Hygiene (security, correctness, polish)

- [ ] **Run `cargo audit` and fix vulnerabilities**
  - [ ] Add `cargo audit` step to CI
  - [ ] Update `image` crate from 0.24 → 0.25

- [ ] **Move debug log to XDG data directory**
  - [ ] Change `"debug.log"` → `dirs::data_local_dir().join("tiles/debug.log")`
  - [ ] Create directory on first write

- [ ] **Pin dependency minor versions for reproducibility**
  - [ ] `tokio = { version = "1.0", ... }` → `"1.41"` (or current)
  - [ ] `regex = "1"` → `"1.11"`
  - [ ] `base64 = "0.22.1"` — fine (pinned patch)
  - [ ] Run `cargo update` + lock

- [ ] **Decompose `event_helpers.rs` (1,307 lines)**
  - [ ] `src/helpers/path.rs` — resolve_relative_path, expand_tilde, path normalization
  - [ ] `src/helpers/files.rs` — file operation helpers (delete, copy, move, trash)
  - [ ] `src/helpers/navigation.rs` — folder navigation, history, selection restore

- [ ] **Add `#[must_use]` to pure functions**
  - [ ] `fuzzy_contains`, `resolve_relative_path`, `try_send_event`

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

- [ ] **Consider `tokio` feature slim-down**
  - [ ] `features = ["full"]` is convenient but pulls ~50 features
  - [ ] Audit which features are actually used (likely: rt-multi-thread, macros, sync, fs, process, io-util)
  - [ ] Reduces compile time and binary size

- [ ] **Add `cargo clippy --release` to CI for release-specific warnings**
  - [ ] Already in CI ✅ — verify it runs on all branches

- [ ] **Consider adding `cargo doc --no-deps` to CI**
  - [ ] Catches broken doc links early
