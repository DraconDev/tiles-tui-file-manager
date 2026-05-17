## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. ✅ Define FileState sub-structs (4 sub-structs defined, not yet activated) — PARTIAL
3. 🔲 Split `ui/mod.rs` (1795 lines → 8+ submodules) — IN PROGRESS
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
**Extracted so far (10 modules):**
- ✅ `header.rs`: draw_global_header (327 lines) — commit 6e612266
- ✅ `footer.rs`: draw_stat_bar (54 lines) — commit 353e9545
- ✅ `debug.rs`: 3 debug functions (233 lines) — commit 125c5ea5
- ✅ `context_menu.rs`: draw_context_menu (197 lines) — commit ffdd9233
- ✅ `monitor.rs`: 4 monitor functions (730 lines) — commit 28e63a35
- ✅ `modals.rs`: 9 modal functions (450 lines) — commit 07150b15
- ✅ `small_modals.rs`: 4 small modals (385 lines) — commit 11875292
- ✅ `misc.rs`: 5 misc functions (266 lines) — commit 963aa964
- ✅ `settings.rs`: 6 settings functions (667 lines) — commit eec0a089

**ui/mod.rs: 5,060 → 1,795 lines** (3,265 lines extracted across 10 modules)

**REMAINING in mod.rs (~5 functions, ~1,250 lines):**
- git_view group (~1000 lines): draw_commit_view + 4 helpers + draw_git_page
  - draw_git_page calls: draw_commit_view, draw_stat_bar, draw_footer, draw_signal_select_modal, draw_pane_breadcrumbs
- file_view group (486 lines): draw_main_stage + draw_file_view
  - draw_main_stage calls: draw_file_view, draw_ide_editor
- footer group (327 lines): draw_footer

**Key technique for nested `use` clauses:**
```rust
#![allow(unused_imports)]
use crate::ui::theme as theme;
// Then regex replace: crate::ui::theme::fn() -> theme::fn()
```

**Cross-module calls:**
- settings.rs → debug::draw_remote_settings (use `crate::ui::debug::draw_remote_settings`)
- misc.rs → format_modified_time (re-exported, used by mod.rs)

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
- `07150b15` refactor(ui): extract modal dialogs to src/ui/modals.rs
- `11875292` refactor(ui): extract small modals to src/ui/small_modals.rs
- `963aa964` refactor(ui): extract misc UI functions to src/ui/misc.rs
- `eec0a089` refactor(ui): extract settings panel to src/ui/settings.rs
- `5b5eb243` chore: update task state and preserve clean build