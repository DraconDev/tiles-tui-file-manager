## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. ✅ Define FileState sub-structs (4 sub-structs defined, not yet activated) — PARTIAL
3. 🔲 Split `ui/mod.rs` (3096 lines → 8+ submodules) — IN PROGRESS
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
**Extracted so far (6 modules):**
- ✅ `header.rs`: draw_global_header (327 lines) — commit 6e612266
- ✅ `footer.rs`: draw_stat_bar (54 lines) — commit 353e9545
- ✅ `debug.rs`: 3 debug functions (233 lines) — commit 125c5ea5
- ✅ `context_menu.rs`: draw_context_menu (197 lines) — commit ffdd9233
- ✅ `monitor.rs`: 4 monitor functions (730 lines) — commit 28e63a35
- ✅ `modals.rs`: 9 modal functions (450 lines) — commit 07150b15

**ui/mod.rs: 5,060 → 3,096 lines** (1,964 lines extracted)

**REMAINING in mod.rs (~14 functions, ~2,500 lines):**
- git_view group (~2521 lines): draw_commit_view + parse_commit_refs + style_for_ref_label + refs_line + draw_git_page
  - draw_git_page calls: draw_commit_view, draw_stat_bar, draw_footer, draw_signal_select_modal, draw_pane_breadcrumbs
- file_view group (486 lines): draw_main_stage + draw_file_view
  - draw_main_stage calls: draw_file_view, draw_ide_editor
- footer group (327 lines): draw_footer
- settings group (~200 lines): draw_settings_modal + draw_shortcuts_settings + draw_column_settings + draw_tab_settings + draw_general_settings + draw_style_settings
- misc remaining: draw_style_color_modal, draw_reset_settings_modal, draw_highlight_modal, draw_drag_ghost, format_modified_time, draw_signal_select_modal, draw_drag_drop_modal, draw_hotkeys_modal, draw_open_with_modal

**Key technique for nested `use` clauses:**
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
- `07150b15` refactor(ui): extract modal dialogs to src/ui/modals.rs
- `5b5eb243` chore: update task state and preserve clean build