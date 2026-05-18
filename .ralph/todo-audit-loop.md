## Goal
Work through the prioritized TODO items for the Tiles TUI file manager.

### P0 — Architecture (highest priority, blocks future velocity)
1. **EventLoopCtx** — Create struct to hold shared mutable state from main.rs. This unblocks all handler extraction.
2. **Decompose event_helpers.rs** (1,298 lines) — blocked by circular deps, needs EventLoopCtx
3. **Decompose events/file_manager.rs** (1,915 lines) — same blocker

### P1 — Bugs & Quality
1. **Editor cursor bug** — dracon-terminal-engine, needs reproduction
2. **Terma clippy errors** — blocks CI
3. **Add tests** — event_helpers.rs, file_manager.rs, theme.rs, modules/files.rs all have 0 tests
4. **Remove dead `default_purple()` alias**

### P2 — Polish & Features
1. **Cross-pane drop on empty space** — drop into other pane's current directory
2. **Marquee from Name column** — vertical drag heuristic
3. **Criterion benchmarks**

**Constraints:**
- `cargo build && cargo test && cargo clippy -- -D warnings` must pass after every change
- Keep commits small and descriptive
- 78 tests must pass

**Key files:**
- `src/main.rs` (1,476 lines, event loop)
- `src/event_helpers.rs` (1,298 lines)
- `src/events/file_manager.rs` (1,915 lines)
- `src/ui/theme.rs` (645 lines)
- `/home/dracon/Dev/dracon-terminal-engine` (editor cursor bug)
