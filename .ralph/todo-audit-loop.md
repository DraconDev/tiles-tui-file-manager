## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Marquee from Name column — vertical-drag heuristic
- [x] Theme tests — 6 new
- [x] File module tests — 7 new
- [x] File_manager helper tests — 5 new
- [x] Event_helpers tests — 4 new (100 total)
- [x] **EventLoopCtx extraction: 1,476 → 424 lines (71% reduction)**
- [x] **Clipboard extraction: src/clipboard.rs** (88 lines, 3 functions)
- [x] **File actions extraction: src/events/file_actions.rs** (344 lines, 6 action + 3 helper functions)
- [x] **Criterion benchmarks: benches/tiles_bench.rs** (4 benchmark groups, 8 measurements)

## Remaining
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — still blocked by circular deps
- [ ] **P0: Decompose file_manager.rs** (1,535 lines) — mouse handler + event dispatcher still large
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: More tests** — untested critical modules
