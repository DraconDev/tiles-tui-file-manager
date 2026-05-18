## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

## Remaining
- [ ] **P0: EventLoopCtx** — struct defined in src/handlers/event_loop_ctx.rs, need to wire into main.rs and extract handlers
- [ ] **P0: Decompose event_helpers.rs** (1,298 lines) — circular dep blocker
- [ ] **P0: Decompose file_manager.rs** (1,915 lines) — same blocker
- [ ] **P1: Editor cursor bug** — needs live reproduction
- [ ] **P1: Add tests** — file_manager.rs (0), more event_helpers coverage
- [ ] **P2: Criterion benchmarks**

## Constraints
- `cargo build && cargo test && cargo clippy -- -D warnings` must pass after every change
- 91 tests must pass
- Small commits