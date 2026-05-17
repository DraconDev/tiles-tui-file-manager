## Task: Architecture Refactor — Tiles TUI File Manager

### Goals
1. ✅ Decompose `App` struct (~120 fields → 13 sub-structs) — DONE
2. 🔲 Activate FileState sub-structs (4 defined, 70+ field migrations needed) — PARTIAL
3. ✅ Split `ui/mod.rs` (5,060 → 383 lines, 92% reduction) — DONE ✅
4. 🔲 Extract `run_tty()` event handlers into `src/handlers/`

### Rules
- Run `cargo build && cargo test` after every change
- Run `cargo clippy` after every change (CI enforces `-D warnings`)
- Preserve all existing behavior and tests

---

## REFLECTION (Iteration 6)

### What has been accomplished?
- **Phase 1 ✅**: App decomposed into 13 sub-structs (commit efa3a9e9)
- **Phase 2 Partial**: FileState sub-structs defined but not activated (commit 952dec60)
- **Phase 3 ✅**: ui/mod.rs split from 5,060 → 383 lines (14 modules, 4,672 lines extracted)
- 14 clean commits, all 54 tests pass, clippy clean at every step

### What's working well?
- **Extraction pattern** (`#![allow(unused_imports)]` + `use crate::ui::theme as theme;` + regex) is reliable
- **Self-contained groups** extract cleanly (modals, monitor, settings, small_modals)
- **"Extract everything to EOF"** approach works for complex functions with nested closures
- **Clippy `-D warnings`** catches issues early; no regressions slipped through
- **Small commits** make rollback easy when extractions fail

### What's not working?
- **Brace counting in Python** is unreliable for complex functions (closures, macros). Workaround: extract from line X to EOF.
- **Import management is tedious** — each extraction needs 10-20 imports added, multiple build/fix cycles
- **Cross-module dependencies** (draw_main_stage ↔ draw_file_view) force grouping functions together
- **Re-export warnings** — `pub use` of items not called within mod.rs triggers unused_import warnings; requires `#[allow(unused_imports)]`

### Should the approach be adjusted?
- Phase 2 (FileState activation) is HIGH RISK — 70+ field migrations across the entire codebase. A script-based approach is essential.
- Phase 4 (event handler extraction) — `run_tty()` is in main.rs (1,740 lines), not app.rs. The event handling code (lines 231-1126) is ~900 lines of match arms. This is a massive extraction.
- pane.rs (836 lines) could be further split (draw_file_view is 458 lines alone), but this is optimization, not critical.

### Next priorities
1. **Phase 4: Extract run_tty() event handlers** — highest impact remaining work
   - main.rs: 1,740 lines → target <500 lines
   - Event handlers span lines ~231-1126 (~900 lines of match arms)
   - Need to identify logical handler groups: key events, mouse events, terminal events
2. **Phase 2: Activate FileState sub-structs** — medium priority, high risk
   - 70+ field reference updates across src/
   - Script-based approach required
3. **Optional: Split pane.rs further** — low priority
   - draw_file_view (458 lines) could be its own module

---

### Phase 1 — App struct decomposition ✅
- `efa3a9e9` — App struct → 13 sub-structs

### Phase 2 — FileState decomposition 🔲 PARTIAL
- `952dec60` — FileState sub-structs defined (FileNavState, FileListState, FileViewState, FileGitState)
- **NOT YET ACTIVATED**: fields still flat on FileState, sub-structs have `#[allow(dead_code)]`

### Phase 3 — ui/mod.rs split ✅ COMPLETE
**14 modules extracted (4,672 lines):**
- `header.rs` (327), `footer.rs` (380), `debug.rs` (233), `context_menu.rs` (197),
- `monitor.rs` (730), `modals.rs` (450), `small_modals.rs` (385), `misc.rs` (266),
- `settings.rs` (667), `git_view.rs` (278), `pane.rs` (801)
- **ui/mod.rs: 5,060 → 383 lines** (pure module hub)

### Phase 4 — Event handler extraction 🔲 NOT STARTED
- `run_tty()` in main.rs: 1,740 lines total
- Event handling code: ~900 lines (lines 231-1126)
- Helper functions: setup_app (104), handle_event (9), prime_visible_tabs (8), prime_local_file_state (91)

---

## Completed Commits (14 total)
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
- `8b285e0d` refactor(ui): extract draw_footer to src/ui/footer.rs
- `829b68c2` refactor(ui): extract draw_commit_view to src/ui/git_view.rs
- `0313dcc0` refactor(ui): extract all remaining draw functions to src/ui/pane.rs