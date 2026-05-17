## Task: Work through remaining TODO items for Tiles TUI File Manager

### Progress this iteration

| Item | Commit | Priority |
|------|--------|----------|
| Guard unwrap in monitor.rs | f95873c3 | P1 ✅ |
| Replace tuple with TreeScanResult struct | f95873c3 | P1 ✅ |
| Add #[must_use] to pure functions | e2f7721c | P2 ✅ |
| Move debug log to XDG directory | 90bb96b4 | P2 ✅ |
| Add unit tests for app.rs (6 tests) | 9feaff30 | P1 ✅ |
| Add unit tests for state/mod.rs (6 tests) | 9feaff30 | P1 ✅ |
| Run cargo audit | c81ef0a8 | P2 ✅ |
| Pin dependency versions | c81ef0a8 | P2 ✅ |
| Upgrade image 0.24 → 0.25 | c81ef0a8 | P2 ✅ |

### Remaining items

| Item | Priority | Effort |
|------|----------|--------|
| Tests for events/editor.rs | P1 | Medium |
| Tests for modules/system.rs | P1 | Medium |
| Fix terma clippy errors | P1 | Medium (different crate) |
| Decompose event_helpers.rs | P2 | Blocked (circular deps) |
| Add doc comments | P3 | Large |
| Add criterion benchmarks | P3 | Large |
| Tokio feature slim-down | P3 | Small |
| EventLoopCtx for main.rs | P0 | Large |

### Stats
- Tests: 54 → 62
- All green: build, test, clippy
