## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Remove dead `default_purple()` alias
- [x] Terma clippy — already clean
- [x] Theme tests — 6 new
- [x] File module tests — 7 new
- [x] Editor cursor bug — investigated, 6 tests pass. Needs live repro.
- [x] Marquee from Name column — vertical-drag heuristic
- [x] EventLoopCtx wired into main.rs — 1,476 → 1,419 lines
- [x] EventLoopCtx::handle_tick() + sync_watches() extracted
- [x] File_manager helper tests — 5 new (96 total)

## Remaining
- [ ] **P0: EventLoopCtx** — extract RefreshFiles, FilesChangedOnDisk, SaveFile, PreviewRequested handlers
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P1: Add tests** — more event_helpers coverage
- [ ] **P2: Criterion benchmarks**
