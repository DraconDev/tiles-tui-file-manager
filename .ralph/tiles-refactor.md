## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. ✅ Activate FileState sub-structs (nav, list, view, git) — DONE ✅
3. ✅ Split `ui/mod.rs` (5,060 → 386 lines, 92% reduction) — DONE ✅
4. 🔲 Extract `run_tty()` event handlers — IN PROGRESS

### All 3 core goals COMPLETE. Phase 4 is ongoing but diminishing returns.

---

### Phase 1 ✅ (commit efa3a9e9)
### Phase 2 ✅ (commits 952dec60 + d9c8dcd3)
### Phase 3 ✅ (17 modules, 4,672 lines extracted from ui/mod.rs)

### Phase 4 — Event handler extraction 🔲 IN PROGRESS
**Completed:**
- `8362806b` — setup.rs (222 lines)
- `58dc9cac` — tree_walk.rs (61 lines)
- `e730a1dd` — split pane.rs into git_page.rs + file_view.rs + pane.rs
- **main.rs: 1,740 → 1,459 lines** (-281 lines)

**Remaining:** run_tty() event loop (~1,340 lines) with 29 AppEvent match arms.
Deep coupling to shared state makes further extraction low-value without
structural changes (EventLoopCtx struct pattern).

---

## Completed Commits (19 total)
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `952dec60` refactor(file_subtypes): define FileState sub-structs
- `6e612266` → `0313dcc0` refactor(ui): extract 14 modules from ui/mod.rs
- `8362806b` refactor(main): extract setup helpers to src/setup.rs
- `58dc9cac` refactor(main): extract walk_tree to src/tree_walk.rs
- `d9c8dcd3` refactor(state): activate FileState sub-structs (nav, list, view, git)
- `e730a1dd` refactor(ui): split pane.rs into git_page.rs + file_view.rs + pane.rs

## Summary of Changes
- **App**: 120 fields → 13 sub-structs
- **FileState**: 35 fields → 4 sub-structs (nav, list, view, git)
- **ui/mod.rs**: 5,060 → 386 lines (92% reduction)
- **main.rs**: 1,740 → 1,459 lines (16% reduction)
- **pane.rs**: 836 → 42 lines (95% reduction, split into git_page.rs + file_view.rs)
- **Total files changed**: 27+ files across all phases
- **All 54 tests pass, clippy clean at every step**