## Task: Work through remaining TODO items for Tiles TUI File Manager

### Current state
- P0 Architecture is ~85% done (3/4 big items complete, event loop extraction blocked)
- P1 Quality items are untouched — quick wins available
- P2 Hygiene items are untouched
- All 54 tests pass, clippy clean

### Priority order for this loop
1. **P1 Quick wins** (guard unwrap, ScanResult struct, #[must_use])
2. **P1 Tests** (add tests for untested critical modules)
3. **P2 Hygiene** (cargo audit, XDG debug log, pin deps)
4. **P0 EventLoopCtx** (if time permits — attempt the EventLoopCtx pattern for main.rs)

### Rules
- Run `cargo build && cargo test && cargo clippy -- -D warnings` after every change
- Keep commits small and descriptive
- All 54 tests must pass at every step