## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Marquee from Name column — vertical-drag heuristic
- [x] Theme tests — 6 new
- [x] File module tests — 7 new
- [x] File_manager helper tests — 5 new
- [x] Event_helpers tests — 4 new (100 total)
- [x] EventLoopCtx wired into main.rs — 1,476 → 1,021 lines (31% reduction)
- [x] 15 handler methods extracted: tick, refresh_files, files_changed_on_disk,
      save_file, create_file, create_folder, rename, delete, trash_file, symlink,
      git_history_updated, task_progress, task_finished, global_search_updated,
      system_monitor, git_history, editor, status_msg, add_to_favorites

## Remaining
- [ ] **P0: EventLoopCtx** — remaining: PreviewRequested (~200 lines), ConnectToRemote, RemoteConnected, Copy (async), SpawnTerminal, SpawnDetached
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: Criterion benchmarks**
