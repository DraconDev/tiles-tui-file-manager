# Tiles Improvement TODO

## 📊 Current Stats

| Metric | Value |
|--------|-------|
| Total source lines | 21,004 |
| Source files | 56 |
| Tests | 129 ✅ |
| Clippy | Clean ✅ |
| TODO/FIXME/HACK | 0 |
| Production unwraps | 0 |
| unsafe blocks | 1 (stdin poll) |
| Suppression directives | 20 (18 `#![allow(unused_imports)]` for pub-use re-exports, 2 `#[allow(dead_code)]` on protocol enums) |

## ✅ Completed

### Architecture
- [x] Split `ui/mod.rs` (5,060→386 lines, -92%)
- [x] Decompose `App` (120 fields → 13 sub-structs)
- [x] Decompose `FileState` (35 fields → 4 sub-structs)
- [x] Extract `EventLoopCtx` — 24 handler methods (src/handlers/event_loop_ctx.rs, 938 lines)
- [x] Extract refresh loop (src/handlers/refresh.rs, 344 lines)
- [x] Extract mouse handler (src/events/file_mouse.rs, 647 lines)
- [x] Extract file actions (src/events/file_actions.rs, 381 lines)
- [x] Extract nav helpers (src/nav_helpers.rs, 330 lines)
- [x] Extract clipboard (src/clipboard.rs, 116 lines)
- [x] Remove dead code: tile_queue, path_colors dup, layout.rs, dead EventLoopCtx methods
- [x] Remove false-positive `#[allow(dead_code)]` from 8 used items
- [x] Deduplicate `is_valid_search_char`, `is_virtual_divider`, `open_file_or_navigate`
- [x] Remove dead `panes_needing_refresh` parameter chain (handle_event → setup → events)
- [x] Clean up dead test imports across 5 test modules

### Theme System
- [x] 22-field ThemeStyle, 14 presets, 34 accessors
- [x] Zero hardcoded colors in `src/ui/` (was 261)
- [x] Legacy Red default theme, WCAG-compliant contrast

### Mouse UX (Dolphin-inspired)
- [x] Marquee drag selection (transparent border-only rect)
- [x] Ctrl+drag toggles, Escape cancels
- [x] Marquee from Name column (vertical-drag heuristic)
- [x] Cross-pane drop on empty space
- [x] Deferred click pattern (pending_click_idx)
- [x] Dolphin-style sidebar (click=navigate, arrow/Space=expand)

### Performance
- [x] Fix unconditional redraw on every Tick (4 redraws/sec → on-demand)
- [x] Short-circuit path_colors HashMap lookups when empty
- [x] Zero-allocation `__DIVIDER__` check (as_os_str vs to_string_lossy)
- [x] Cache semantic_coloring setting before row loop

### Quality
- [x] 78 → 129 tests (+65%)
- [x] Circular dep broken (event_helpers no longer imports from events/)
- [x] Criterion benchmarks (4 groups, 8 measurements)
- [x] Save state on quit, guard production unwrap
- [x] cargo audit, CI (clippy, doc, audit)
- [x] Undo close tab (Ctrl+Shift+T, max 10)

### Runtime Bug Fixes
- [x] Konsole tab, pipewire noise, settings off-by-one
- [x] Editor reload race, Ctrl+H hidden, sidebar dotfiles
- [x] Git mouse coords, full commit hash, relative time
- [x] Self-save fallback, editor preview clear
- [x] Stale file list on navigation
- [x] Theme persistence cycle (4 root causes)
- [x] Bounds-check crash fix (pending_click_idx, marquee)

## 🔴 P0 — Bugs

- [ ] **Editor cursor bug** (dracon-terminal-engine)
  - After pressing Enter, cursor column offset by +1 per empty row before insertion point
  - Requires reproduction and fix in `/home/dracon/Dev/dracon-terminal-engine`

## 🟡 P1 — Quality

- [ ] **Add tests for untested critical modules**
  - [ ] `event_helpers.rs` — 3 tests (core navigation)
  - [ ] `events/file_manager.rs` — 0 tests (1082 lines, keyboard handler)
  - [ ] `events/file_mouse.rs` — 0 tests (647 lines, mouse handler)
  - [ ] `modules/files.rs` — 0 tests (file operations)

## 🟢 P2 — Polish & Features

- [ ] **Criterion benchmarks for remaining hot paths**
  - [ ] `draw()` — full render cycle
  - [ ] `walk_tree()` — directory traversal
- [ ] **Hover +/- selection buttons** — revisit only if marquee isn't sufficient

## 📊 Decomposition Stats

| File | Before | After | Reduction |
|------|--------|-------|-----------|
| main.rs | 1,476 | 421 | **-71%** |
| event_helpers.rs | 1,343 | 850 | **-37%** |
| file_manager.rs | 1,990 | 1,082 | **-46%** |
| ui/mod.rs | 5,060 | 397 | **-92%** |
| modals.rs | 1,929 | 993 | **-49%** |
| pane.rs | 836 | 44 | **-95%** |
