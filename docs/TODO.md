# Tiles Improvement TODO

Full audit — 2026-05-18

---

## Codebase Stats

| Metric | Value |
|--------|-------|
| Total lines | 20,180 |
| Source files | 48 |
| Public functions | 217 |
| Tests | 78 |
| Clippy | Clean ✅ |
| Doc warnings | 0 |
| TODO/FIXME/HACK | 0 |
| Production unwraps | 0 |
| unsafe blocks | 1 (stdin poll) |
| Clippy suppressions | 6 (justified) |

### Largest files

| File | Lines | Status |
|------|-------|--------|
| `src/events/file_manager.rs` | 1,915 | ⚠️ largest, blocked from split |
| `src/main.rs` | 1,476 | ⚠️ event loop god-file |
| `src/event_helpers.rs` | 1,298 | ⚠️ blocked from split |
| `src/events/modals.rs` | 993 | manageable |
| `src/ui/panes/sidebar.rs` | 969 | OK |
| `src/events/mod.rs` | 874 | OK |
| `src/events/editor.rs` | 866 | OK |

---

## ✅ Completed

### Architecture
- [x] Split `ui/mod.rs` (5,060→386 lines, -92%)
- [x] Decompose `App` (120 fields → 13 sub-structs)
- [x] Decompose `FileState` (35 fields → 4 sub-structs)
- [x] Event handler extraction (8 new modules)

### Theme System
- [x] ThemeStyle 6→22 fields, 14 presets, 34 accessors
- [x] 261 hardcoded `Color::` replaced, 12 intentional remaining
- [x] Default theme = Legacy Red (`preset_legacy_red()`)
- [x] All 14 presets pass WCAG contrast (selection_fg=black)
- [x] Theme persistence: `config.rs` skips persisting when current==default
- [x] Migration: `setup.rs` discards stale `preset_cool()`/`preset_warm()` from `state.json`

### Mouse UX (Dolphin-inspired)
- [x] Marquee drag selection (transparent border-only rect)
- [x] Undo close tab — Ctrl+Shift+T (max 10)
- [x] File drag-and-drop with Name column click zone
- [x] Cross-pane drag-and-drop (pane_rects stored during render)
- [x] Deferred click pattern (pending_click_idx) — preserves multi-selection during marquee
- [x] Sidebar folder click = navigate only (no auto-expand, Dolphin-style)
- [x] Click empty space deselects all
- [x] Bounds-checked stale indices on mouseUp (crash fix)

### Quality
- [x] Save state on quit (all 3 quit paths)
- [x] Guard production unwrap
- [x] TreeScanResult struct (replaced type_complexity)
- [x] `#[must_use]` on pure functions
- [x] cargo audit, pin deps, slim tokio
- [x] Doc comments on all public API
- [x] XDG debug log, CI (clippy, doc, audit)

### Runtime Bug Fixes
- [x] Konsole tab open, pipewire noise, settings off-by-one
- [x] Editor reload race, Ctrl+H hidden, sidebar dotfiles
- [x] Git mouse coords, full commit hash, relative time
- [x] Self-save fallback, editor preview clear
- [x] Stale file list on navigation (clear files/metadata immediately)
- [x] Theme persistence cycle (4 root causes fixed)

---

## 🔴 P0 — Architecture (blocks future velocity)

- [ ] **Extract `run_tty()` event loop** — `src/main.rs` is 1,476 lines with 70 `AppEvent::` match arms
  - [ ] Create `EventLoopCtx` struct (app, event_tx, panes_needing_refresh, last_self_save, debouncer)
  - [ ] `src/handlers/files.rs` — RefreshFiles, CreateFile, CreateFolder, Delete, Trash, Copy, Move, Rename
  - [ ] `src/handlers/editor.rs` — Save, SaveAs, editor sync, file-watcher reload
  - [ ] `src/handlers/remote.rs` — ConnectRemote, DisconnectRemote, remote file ops
  - [ ] `src/handlers/git.rs` — GitRefresh, GitCommit, GitCheckout
  - [ ] `src/handlers/monitor.rs` — MonitorUpdate, KillProcess, SignalSelect
  - [ ] `src/handlers/clipboard.rs` — ClipboardCopy, ClipboardPaste
  - [ ] `src/handlers/settings.rs` — SaveSettings, LoadSettings, ResetSettings
  - [ ] `src/handlers/navigation.rs` — Navigate, TabSwitch, ToggleZoom
  - **BLOCKER:** Deep coupling to shared mutable state. 29 match arms all reference `app.lock()`, `last_self_save`, `debouncer`, `panes_needing_refresh`. Requires EventLoopCtx first.

- [ ] **Decompose `event_helpers.rs` (1,298 lines)**
  - [ ] `src/helpers/path.rs` — resolve_relative_path, expand_tilde, path normalization
  - [ ] `src/helpers/files.rs` — file operation helpers (delete, copy, move, trash)
  - [ ] `src/helpers/navigation.rs` — folder navigation, history, selection restore
  - **BLOCKER:** Circular dep with `events/mod.rs` (navigate_back/forward/push_history called from dispatcher)

- [ ] **Decompose `events/file_manager.rs` (1,915 lines)**
  - [ ] Extract mouse handling (~600 lines) to `events/file_mouse.rs`
  - [ ] Extract key handling (~500 lines) to `events/file_keys.rs`
  - [ ] Keep dispatcher + shared helpers in `file_manager.rs`
  - **BLOCKER:** Same circular dep pattern as event_helpers

---

## 🟡 P1 — Bugs & Quality

- [ ] **Editor cursor bug** (dracon-terminal-engine)
  - After pressing Enter, cursor column offset by +1 per empty row before insertion point
  - Requires reproduction and fix in `/home/dracon/Dev/dracon-terminal-engine`

- [x] ~~Fix terma clippy errors~~ — already clean, no action needed

- [ ] **Add tests for untested critical modules** — PARTIAL
  - [x] `app.rs` — 6 tests, `state/mod.rs` — 6 tests, `config.rs` — 11 tests
  - [x] `events/editor.rs` — 3 tests, `modules/system.rs` — 7 tests
  - [ ] `event_helpers.rs` — 0 tests (1298 lines, core navigation)
  - [ ] `events/file_manager.rs` — 0 tests (1915 lines, all mouse/keyboard)
  - [ ] `ui/theme.rs` — 0 tests (645 lines, 14 presets)
  - [ ] `modules/files.rs` — 0 tests (file operations)

- [x] **Remove dead `default_purple()` alias** — removed, all callers already use `default()`

---

## 🟢 P2 — Polish & Features

- [ ] **Cross-pane drag: drop on empty space** — currently only drops on folder rows. Should support dropping into the other pane's current directory (drop on empty space = move to that dir).

- [ ] **Marquee: drag from Name column** — currently marquee only starts from non-Name column clicks. Some users may expect Name-column vertical drag to start marquee (like Dolphin). Consider: if drag is primarily vertical (>3 rows) and horizontal distance < Name column width, prefer marquee over file drag.

- [ ] **Criterion benchmarks for hot paths**
  - [ ] `draw()` — full render cycle
  - [ ] `walk_tree()` — directory traversal
  - [ ] `fuzzy_contains()` — search matching

- [ ] **Hover +/- selection buttons** — skeptical, revisit only if marquee isn't sufficient

---

## 📊 Refactor Stats (65+ commits)

| Metric | Before | After |
|--------|--------|-------|
| ui/mod.rs | 5,060 lines | 386 lines (-92%) |
| App struct | 120 flat fields | 13 sub-structs |
| FileState struct | 35 flat fields | 4 sub-structs |
| modals.rs | 1,929 lines | 991 lines (-49%) |
| pane.rs | 836 lines | 43 lines (-95%) |
| Total modules created | 0 | 21+ |
| Tests | 54 | 78 ✅ |
| Clippy | Clean | Clean ✅ |
| Theme presets | 6 | 14 ✅ |
| ThemeStyle fields | 6 | 22 ✅ |
| Hardcoded colors | 261 | 0 (12 intentional) ✅ |
