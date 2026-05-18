## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Marquee from Name column — vertical-drag heuristic
- [x] Theme tests — 6 new
- [x] File module tests — 7 new
- [x] File_manager helper tests — 5 new
- [x] Event_helpers tests — 4 new (100 total)
- [x] EventLoopCtx wired into main.rs — 1,476 → 1,241 lines
- [x] Extracted: handle_tick, handle_refresh_files, handle_files_changed_on_disk, handle_save_file, sync_watches

## Remaining
- [ ] **P0: EventLoopCtx** — extract PreviewRequested (~200 lines), ConnectToRemote, CreateFile, CreateFolder, Rename, Delete, TrashFile, Copy, Symlink, KillProcess, GitHistory, etc.
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: Criterion benchmarks**
