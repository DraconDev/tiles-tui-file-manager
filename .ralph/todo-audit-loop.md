## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Progress

### Done this session
- [x] **Cross-pane drop on empty space** — `DropTarget::CurrentDir(pane_idx)`
- [x] **Remove dead `default_purple()` alias**
- [x] **Terma clippy** — already clean
- [x] **Theme tests** — 6 new (84 total)
- [x] **File module tests** — 7 new (91 total)
- [x] **Editor cursor bug** — investigated, 6 tests in dracon-terminal-engine, all pass. Needs live repro.
- [x] **Marquee from Name column** — vertical-drag heuristic (dy > dx*2)
- [x] **EventLoopCtx struct foundation** — defined and wired into main.rs
- [x] **EventLoopCtx::handle_tick()** — Tick handler extracted
- [x] **EventLoopCtx::sync_watches()** — sync_watches closure extracted
- [x] **File_manager helper tests** — 5 new (96 total)
- [x] **main.rs reduction** — 1,476 → 1,419 lines

### Remaining
- [ ] **P0: EventLoopCtx** — extract RefreshFiles, FilesChangedOnDisk, SaveFile handlers
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P1: Add tests** — more event_helpers coverage
- [ ] **P2: Criterion benchmarks**
