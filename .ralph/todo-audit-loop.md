## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Marquee from Name column — vertical-drag heuristic
- [x] Theme tests — 6 new
- [x] File module tests — 7 new
- [x] File_manager helper tests — 5 new
- [x] Event_helpers tests — 4 new (100 total)
- [x] EventLoopCtx wired into main.rs — 1,476 → 758 lines (49% reduction!)
- [x] 24 handler methods extracted into EventLoopCtx (950 lines):
      tick, refresh_files, files_changed_on_disk, save_file,
      create_file, create_folder, rename, delete, trash_file, symlink,
      git_history_updated, task_progress, task_finished,
      global_search_updated, system_monitor, git_history, editor,
      status_msg, add_to_favorites, remote_connected, spawn_terminal,
      spawn_detached, system_updated, connect_to_remote, copy,
      preview_requested

## Remaining
- [ ] **P0: EventLoopCtx** — remaining in main.rs: Raw event handler, the "Handle Refreshes" file list refresh loop, terminal resize, quit/save paths
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: Criterion benchmarks**
