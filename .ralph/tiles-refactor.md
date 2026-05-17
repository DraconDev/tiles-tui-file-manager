## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. ✅ Define FileState sub-structs (4 sub-structs defined, not yet activated) — PARTIAL
3. 🔲 Split `ui/mod.rs` (3538 lines → 8+ submodules) — IN PROGRESS
4. 🔲 Extract `run_tty()` event handlers into `src/handlers/`

### Rules
- Run `cargo build && cargo test` after every change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests

### Phase 1 — App struct decomposition ✅
- `efa3a9e9` — App struct → 13 sub-structs

### Phase 2 — FileState decomposition 🔲 PARTIAL
- `952dec60` — FileState sub-structs defined (FileNavState, FileListState, FileViewState, FileGitState)

### Phase 3 — ui/mod.rs split 🔲 IN PROGRESS
**Extracted so far (5 modules):**
- ✅ `header.rs`: draw_global_header (327 lines) — commit 6e612266
- ✅ `footer.rs`: draw_stat_bar (54 lines) — commit 353e9545
- ✅ `debug.rs`: 3 debug functions (233 lines) — commit 125c5ea5
- ✅ `context_menu.rs`: draw_context_menu (197 lines) — commit ffdd9233
- ✅ `monitor.rs`: 4 monitor functions (730 lines) — commit 28e63a35

**ui/mod.rs: 5,060 → 3,538 lines** (1,522 lines extracted)

**Remaining extraction targets:**
1. git_view.rs — draw_commit_view + helpers + draw_git_page (~2521 lines) — FAILED (too many cross-deps)
2. file_view.rs — draw_main_stage (27 lines) + draw_file_view (459 lines) — FAILED (too many cross-deps)
3. footer.rs — draw_footer (327 lines) — FAILED (complex imports, nested use clauses)
4. settings.rs — 10 functions (~976 lines)
5. modals.rs — 15 functions (~957 lines)
6. helpers (parse_commit_refs, style_for_ref_label, refs_line) — 50 lines

**Key technique (monitor.rs pattern):**
```rust
#![allow(unused_imports)]
use crate::ui::theme as theme;
// Then regex replace: crate::ui::theme::fn() -> theme::fn()
```

### Phase 4 — event handlers extraction
- Not started

---

## Completed Commits
- `efa3a9e9` refactor(app): decompose App struct into 13 logical sub-structs
- `952dec60` refactor(file_subtypes): define FileState sub-structs
- `6e612266` refactor(ui): extract draw_global_header to src/ui/header.rs
- `353e9545` refactor(ui): extract draw_stat_bar to src/ui/footer.rs
- `125c5ea5` refactor(ui): extract debug functions to src/ui/debug.rs
- `ffdd9233` refactor(ui): extract draw_context_menu to src/ui/context_menu.rs
- `28e63a35` refactor(ui): extract monitor functions to src/ui/monitor.rs