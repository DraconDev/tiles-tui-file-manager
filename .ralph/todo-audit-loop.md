## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Journey Summary

### Structural Decomposition
| File | Before | After | Reduction |
|------|--------|-------|-----------|
| main.rs | 1,476 | 424 | **71%** |
| event_helpers.rs | 1,343 | ~800 | **40%** |
| file_manager.rs | 1,990 | 1,528 | **23%** |
| **Total** | **~4,800** | **~2,760** | **~43%** |

### What was extracted
- **EventLoopCtx** — 24 handler methods (src/handlers/event_loop_ctx.rs)
- **Refresh loop** — src/handlers/refresh.rs (344 lines)
- **Clipboard** — src/clipboard.rs (88 lines)
- **File actions** — src/events/file_actions.rs (344 lines)
- **Nav helpers** — src/nav_helpers.rs (330 lines)
- **Criterion benchmarks** — benches/tiles_bench.rs

### Quality
- Tests: **78 → 106** (+28 tests across 6 modules)
- Circular dep broken: event_helpers no longer imports from events/
- Criterion benchmarks for 4 hot paths
- All clippy clean

### Blocked
- file_manager.rs mouse handler (627 lines) tightly coupled
- Editor cursor bug needs live reproduction
