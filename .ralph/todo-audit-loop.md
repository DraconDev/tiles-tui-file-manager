## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Done
- [x] Cross-pane drop on empty space
- [x] Marquee from Name column — vertical-drag heuristic
- [x] **Theme tests** — 6 new
- [x] **File module tests** — 7 new
- [x] **File_manager helper tests** — 5 new
- [x] **Event_helpers tests** — 4 new
- [x] **EventLoopCtx extraction** — 1,476 → 424 lines (71% reduction)
- [x] **Clipboard extraction** — src/clipboard.rs (88 lines)
- [x] **File actions extraction** — src/events/file_actions.rs (344 lines)
- [x] **Criterion benchmarks** — benches/tiles_bench.rs
- [x] **Circular dep broken** — event_helpers no longer imports from events/
- [x] **File_actions tests** — 6 new (is_virtual_divider, path_join)
- [x] **Clipboard tests** — 2 new (error path, edge cases)
- [x] **108 tests total** (was 78)

## Remaining
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep ISOLATED, extraction unblocked
- [ ] **P0: Decompose file_manager.rs** (1,535 lines) — mouse handler (627 lines) + event dispatcher
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: More tests** — untested critical modules
