## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Progress

### P2 — Polish (done this iteration)
- [x] **Cross-pane drop on empty space** — `DropTarget::CurrentDir(pane_idx)` variant. When dragging over another pane but not on a folder, drops into that pane's current directory. Footer shows target dir name.
- [x] **Remove dead `default_purple()` alias** — removed, callers use `default()`
- [x] **Terma clippy** — already clean, no action needed

### P1 — Quality (done this iteration)
- [x] **Add theme tests** — 6 new tests (84 total, was 78):
  - default_is_legacy_red, all_presets_have_black_selection_fg, all_presets_have_nonzero_accent
  - presets_are_distinct, set_and_get_roundtrip, accent_primary_accessor_matches_style

### Remaining items
- [ ] **P0: EventLoopCtx** — extract main.rs event loop (1,476 lines, 70 match arms). BLOCKER for all handler extraction.
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep blocker
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — dracon-terminal-engine, needs reproduction. insert_newline() looks correct in code review.
- [ ] **P1: Add tests** — event_helpers.rs (0), file_manager.rs (0), modules/files.rs (0) still need coverage
- [ ] **P2: Marquee from Name column** — vertical drag heuristic
- [ ] **P2: Criterion benchmarks**
