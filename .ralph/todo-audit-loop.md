## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Progress

### Done this session
- [x] **Cross-pane drop on empty space** — `DropTarget::CurrentDir(pane_idx)`
- [x] **Remove dead `default_purple()` alias**
- [x] **Terma clippy** — already clean
- [x] **Theme tests** — 6 new (84 total)
- [x] **File module tests** — 7 new (91 total)
- [x] **Editor cursor bug** — investigated, wrote 6 tests in dracon-terminal-engine, all pass. Bug needs live reproduction.
- [x] **Marquee from Name column** — vertical-drag heuristic (dy > dx*2)
- [x] **EventLoopCtx struct foundation** — `src/handlers/event_loop_ctx.rs` defined

### Reflection
**What's working:** Test additions, feature work (drag/drop/marquee), and structural foundations compile cleanly.
**What's blocked:** Full EventLoopCtx extraction is a massive refactor (1,476 lines, 70 match arms). Each handler needs careful extraction. The circular dep pattern still blocks event_helpers.rs and file_manager.rs splits.
**Should adjust:** The P0 items are multi-hour efforts best done in dedicated sessions. For the loop, smaller items (tests, features) ship faster. EventLoopCtx is the highest-value P0 but needs a full session.
**Next priorities:** Continue EventLoopCtx extraction (start with Tick handler as simplest), or await user direction.

### Remaining
- [ ] **P0: EventLoopCtx** — struct defined, need to wire into main.rs and extract handlers
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P1: Add tests** — file_manager.rs (0), more event_helpers coverage
- [ ] **P2: Criterion benchmarks**
