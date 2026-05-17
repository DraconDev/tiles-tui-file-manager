## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. ✅ Activate FileState sub-structs (nav, list, view, git) — DONE ✅
3. ✅ Split `ui/mod.rs` (5,060 → 383 lines, 92% reduction) — DONE ✅
4. 🔲 Extract `run_tty()` event handlers — IN PROGRESS

### Rules
- Run `cargo build && cargo test` after every change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests

---

### Phase 1 — App struct decomposition ✅ (commit efa3a9e9)
### Phase 2 — FileState decomposition ✅ (commits 952dec60 + d9c8dcd3)
### Phase 3 — ui/mod.rs split ✅ (14 modules, 4,672 lines extracted)

### Phase 4 — Event handler extraction 🔲 IN PROGRESS
**Completed:**
- `8362806b` — setup.rs (222 lines)
- `58dc9cac` — tree_walk.rs (61 lines)
- **main.rs: 1,740 → 1,460 lines** (-280 lines)

**Remaining:** run_tty() event loop (~1,340 lines) with 29 AppEvent match arms.
Handlers are deeply coupled to shared mutable state — further extraction requires
an EventLoopCtx struct or similar pattern.

---

## Completed Commits (18 total)
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `952dec60` refactor(file_subtypes): define FileState sub-structs
- `6e612266` → `0313dcc0` refactor(ui): extract 14 modules from ui/mod.rs
- `8362806b` refactor(main): extract setup helpers to src/setup.rs
- `58dc9cac` refactor(main): extract walk_tree to src/tree_walk.rs
- `d9c8dcd3` refactor(state): activate FileState sub-structs (nav, list, view, git)

## Summary of Changes
- **App**: 120 fields → 13 sub-structs
- **FileState**: 35 fields → 4 sub-structs (nav, list, view, git)
- **ui/mod.rs**: 5,060 → 383 lines (92% reduction)
- **main.rs**: 1,740 → 1,460 lines (16% reduction)
- **Total files changed**: 25+ files across all phases
- **All 54 tests pass, clippy clean at every step**