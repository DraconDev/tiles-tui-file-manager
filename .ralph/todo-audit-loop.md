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
  - 24 handler methods in event_loop_ctx.rs (950 lines)
  - Refresh loop in handlers/refresh.rs (344 lines)
  - TreeScanResult moved to handlers/refresh.rs
  - main.rs is now setup + input threads + event dispatch only

## Reflection
**What's working:** The `ctx.handle_*()` pattern cleanly separates concerns. Async handlers take `app.clone()`. The refresh extraction into its own file reduces main.rs to essentially boilerplate.

**What's left in main.rs (424 lines):** Setup (~50), watcher + input threads (~100), tick/stats loops (~30), terminal.draw + quit logic (~20), event dispatch match (~200). Most of this is infrastructure, not business logic.

**What's blocked:** event_helpers.rs and file_manager.rs decompositions are still blocked by circular deps.

**Should adjust:** main.rs is now at a natural minimum — the remaining code is glue/infrastructure that doesn't benefit from extraction. Shift focus to: adding tests for critical untested modules, and attempting the circular dep workaround.

**Next priorities:** 
1. Try event_helpers.rs decomposition (maybe via trait dispatch?)
2. Add tests for file_manager.rs mouse handling
3. Criterion benchmarks for hot paths

## Remaining
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep, try trait workaround
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P2: Criterion benchmarks**
