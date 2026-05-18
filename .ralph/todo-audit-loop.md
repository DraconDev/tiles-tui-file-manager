## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Progress

### P2 — Polish (done)
- [x] **Cross-pane drop on empty space** — `DropTarget::CurrentDir(pane_idx)`, footer label
- [x] **Remove dead `default_purple()` alias**
- [x] **Terma clippy** — already clean

### P1 — Quality (done)
- [x] **Theme tests** — 6 new (84 total) 
- [x] **File module tests** — 7 new (91 total)
- [x] **Editor cursor bug** — wrote 6 reproduction tests in dracon-terminal-engine. All pass. `insert_newline()` is correct. Bug likely in `ensure_cursor_visible` wrap mode or tiles' editor integration. Needs live reproduction with specific file content.

### Remaining
- [ ] **P0: EventLoopCtx** — main.rs (1,476 lines, 70 match arms). BLOCKER.
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P1: Add tests** — file_manager.rs (0 tests), more event_helpers coverage
- [ ] **P2: Marquee from Name column** — vertical drag heuristic
- [ ] **P2: Criterion benchmarks**
