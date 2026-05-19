# Changelog

All notable changes to this project will be documented in this file.

## [14.112.0] - Architecture Overhaul, Performance, Dead Code Cleanup

### Changed
- **Architecture** — Decomposed monolithic god-files into focused modules:
  - `EventLoopCtx` — 24 handler methods extracted from main.rs (1,476 → 421 lines, -71%)
  - `file_mouse.rs` — 647-line mouse handler extracted from file_manager.rs (1,990 → 1,143 lines, -43%)
  - `file_actions.rs` — 381-line keyboard action handlers
  - `nav_helpers.rs` — 330-line navigation history module
  - `clipboard.rs` — 116-line clipboard utilities with OSC 52 fallback
  - `refresh.rs` — 344-line async file refresh loop
  - UI split into 14 modules (5,060 → 386 lines, -92%)
- **App** — 120 flat fields → 13 sub-structs; FileState → 4 sub-structs
- **Theme** — 22-field ThemeStyle, 14 presets, 34 accessors, zero hardcoded colors
- **Performance** — Fixed unconditional redraw on every 250ms Tick (4 redraws/sec → on-demand)
- **Performance** — Short-circuit `path_colors` HashMap lookups when empty
- **Performance** — Zero-allocation `__DIVIDER__` check (`as_os_str` vs `to_string_lossy`)
- **Dead code** — Removed `App.tile_queue`, duplicate `FileListState.path_colors`, dead `panes_needing_refresh` parameter chain, unused `layout.rs`, false-positive `#[allow(dead_code)]` on 8 used items
- **Deduplication** — `is_valid_search_char`, `is_virtual_divider`, `open_file_or_navigate` consolidated into `file_actions.rs`

### Added
- **Marquee drag selection** — transparent border-only rect, Ctrl+drag toggles, Escape cancels
- **Marquee from Name column** — vertical-drag heuristic (dy > dx*2 && dy >= 2)
- **Cross-pane drop on empty space** — `DropTarget::CurrentDir(pane_idx)`
- **Undo close tab (Ctrl+Shift+T)** — max 10 closed tabs restored
- **Deferred click pattern** — `pending_click_idx` preserves multi-selection during marquee drags
- **Criterion benchmarks** — 4 benchmark groups, 8 measurements
- **Tests** — 78 → 129 tests (+65%): theme (6), file actions (6), clipboard (2), nav helpers (18), file_mouse (6), file_manager (4), event_helpers (7), files.rs (6), double-click edge cases (4)

### Fixed
- **CPU spike** — `handle_tick()` forced full redraw every 250ms even when idle
- **Theme persistence cycle** — 4 root causes: stale state.json discarded, skip persisting when current == default
- **Self-save guard** — Don't remove inotify entries on first match; let 5s retain() cleanup handle expiration
- **Stale file list on navigation** — All paths clear files/metadata immediately
- **Bounds-check crash** — `pending_click_idx` and marquee `file_row_bounds` bounds-checked before indexing
- **Konsole tabs** — pipewire noise, settings off-by-one, editor reload race, Ctrl+H hidden, sidebar dotfiles

## [14.32.0] - Keybinding Rework, Scroll Restore, UI Refinements

### Changed
- **Run shortcut** — `Ctrl+Enter` replaced with `Ctrl+R` in both Files and Editor views. `Alt+Enter` no longer triggers run (it shows Properties).
- **Replace shortcut** — `F2` in Editor view now opens Find & Replace (was tied to Rename). F2 in Files view remains Rename.
- **Modified/open indicators** — Row background highlight (`selection_bg()`) replaces `●` dot for modified editor tabs/footer and open sidebar files. No colored dots anywhere in the UI.
- **License** — Changed from `"Dracon-1.1"` to `"AGPL-3.0-only"` in Cargo.toml to match actual LICENSE file.
- **flake.nix** — Updated version, license (`agpl3Only`), and description to match Cargo.toml.

### Added
- **Scroll position restore** — `scroll_positions: HashMap<PathBuf, (usize, usize, usize, usize)>` on App preserves editor scroll/cursor across preview swaps and Esc dismissals. Cache updated on every interaction path (keyboard, mouse, Esc).
- **`--version` / `-V` flag** — Prints version and exits.
- **Version in Settings** — General tab shows current version.
- **Drag cancellation** — `Moved` events (no button held) and `Esc` now properly cancel active drag.
- **Input shield** — Soft shield drops `KeyCode::Char` events 100ms after hard shield expires, preventing stale key insertion.

### Fixed
- **Scroll reset on preview swap** — Pane previews kept alive on Esc; only full-screen overlay state destroyed. Scroll restored from cache on `PreviewRequested`.
- **Self-save reload loop** — `FilesChangedOnDisk` checks `last_self_save` before adding previews to `needs_reload`. Time-window fallback (2s) for edge cases. Directory self-saves skipped.
- **Stale `app` reference in mouse handler** — Removed `app.scroll_positions.insert()` from `handle_text_editor_mouse` and `handle_generic_editor_shortcuts` (no `app` in scope).

## [12.1.0] - Drag Cancel Fixes

### Fixed
- **Drag not cancelled on mouse release** — `Moved` events (mouse moved without button held) now properly clear drag state, stopping ghost drag.
- **Drag not cancelled on Esc** — `Esc` key now cancels any active drag in both file pane and sidebar.

## [12.0.0] - Sorting, Click Fixes & Context Menu

### Added
- **Column header sorting** — Clicking Name/Size/Modified/Created column headers now sorts the file tree by that column. `walk_tree` uses the user's chosen `sort_column` and `sort_ascending` at each directory level, preserving tree structure.
- **Right-click empty space** — Right-clicking below the file list (or on a row with no file) opens a `ContextMenuTarget::EmptySpace` context menu with New File, New Folder, Paste, Toggle Hidden, Terminal Tab/Window, System Monitor.
- **Sort toggle unit tests** — `sort_toggle_toggles_ascending_on_same_column` and `column_bounds_match_click` verify sorting behavior.
- **`FileRowBounds` struct** — Tracks `arrow_end_x` per visible row in `FileState.file_row_bounds`, enabling precise arrow-region click detection.

### Changed
- **Arrow vs name click** — File pane now follows the same pattern as the sidebar:
  - Click **arrow** (▸/▾ before `arrow_end_x`): toggle expand/collapse only
  - Click **name**: select only (falls through to existing selection handler)
  - Double-click on **name**: navigate into folder
- **`walk_tree` sort params** — `sort_column` and `sort_ascending` are now passed into `walk_tree` and applied at each directory level. Previously `walk_tree` always sorted alphabetically, ignoring the user's sort preference.
- **Double-click state** — Removed the unconditional `mouse_last_click`/`mouse_click_pos` reset at the top of `handle_general_mouse` that was preventing double-click navigation into folders.

### Fixed
- **package.json icon** — `package.json` and `package-lock.json` now render with Unicode icon mode (`{ `) instead of Nerd Font, avoiding the blank-box (tofu) rendering issue in terminals without Nerd Font patched fonts (e.g., Konsole). All other files continue using Nerd Font icons.

### Changed
- **Double-click enter folder** — With the double-click state fix, double-clicking a folder name in the file pane now correctly navigates into it (previously broken by the state reset).

## [11.7.0] - package.json Icon Fallback

### Fixed
- **package.json rendering** — Reverted aggressive `detect_default_icon_mode()` that switched ALL icons to Unicode for Konsole. Replaced with a targeted override: only `package.json` and `package-lock.json` force Unicode mode, preserving Nerd Font icons for all other files.

## [11.2.0] - Sort Column in RefreshFiles Handler

### Added
- **`sort_column`/`sort_ascending` in refresh tuple** — The `RefreshFiles` event handler now captures the current sort preferences alongside the path, enabling `walk_tree` to apply the correct sort order at each directory level.

## [11.0.0] - Icon Mode Detection

### Added
- **Terminal-aware icon mode** — `detect_default_icon_mode()` checks terminal environment variables and defaults to `IconMode::Unicode` for Konsole/GNOME Terminal/Xterm, while modern terminals (Kitty, Alacritty, WezTerm, iTerm2, VSCode) use `IconMode::Nerd`.
- **`FileRowBounds` tracking** — Per-row bounds with `arrow_end_x` are populated during render, enabling arrow-region click detection for the file pane (matching sidebar behavior).

## [10.34.78] - Cleanup & Package Hygiene

### Changed
- **Cargo.toml license** — Fixed from `"MIT"` to `"Dracon-1.1"` to match actual LICENSE file *(now `"AGPL-3.0-only"` — see v14.32.0)*
- **Cargo.toml exclude** — Added `flake.nix`, `flake.lock`, `tiles.desktop`, `tiles_icon.svg` to exclude list
- **README.md** — Complete rewrite: terminal compatibility table, Space/Enter key behavior, install section, configuration docs
- **CONTRIBUTING.md** — Updated deps table to crates.io, added `terminal.rs` to structure, fixed `dracon-system` → `dracon-system-lib`
- **QA matrix** — Added terminal spawning test section (T1-T8)

### Removed
- **`vendor/utils.rs`** — Dead code (old `spawn_terminal_at` replaced by `modules/terminal.rs`)
- **`note.md`** — Empty file shipped in crate for no reason
- **Old release binaries** — Removed `tiles-v0.1.*-linux` and `tiles-v0.19.*-linux` from `releases/`

## [10.34.75] - Konsole Tab Fix + crates.io Publishing

### Added
- **Terminal Tab Spawning** — New `modules/terminal.rs` module replaces `dracon_terminal_engine::utils::spawn_terminal_at`:
  - **Konsole**: Uses `dbus-send` + `org.kde.KDBusService.CommandLine` to open tabs in existing window (bypasses blocked `runCommand` D-Bus API)
  - **Kitty**: Context-aware detection, spawns via `kitty @ launch --type=tab`
  - **Wezterm**: Spawns via `wezterm cli spawn --new-window=false`
  - **Generic fallback**: Per-terminal match arms with correct arg ordering (`--new-tab --workdir PATH -e CMD...` vs `--tab --working-directory=PATH -- CMD...`)
  - **`split_command()`** — Shell-like parser for command strings (handles single/double quotes)
- **crates.io publishing** — All dependencies now resolve from crates.io instead of git:
  - `dracon-files` v94.2.7
  - `dracon-git` v94.2.7
  - `dracon-system-lib` v94.2.7 (renamed from `dracon-system`)
  - `dracon-terminal-engine` v1.1.17
  - `tiles-tui-file-manager` v10.34.75 (crate name) / `tiles` (binary name)
- **Build script** (`scripts/build.sh`) — Auto-detects `which tiles` path, copies from `~/.cargo/bin/tiles` with atomic swap
- **Cargo.toml** — Added `exclude` list to reduce crate package from 61MB to 268KB
- **Token storage** — crates.io API token saved at `~/.dracon/crates-io-token`

### Changed
- `dracon-system` → `dracon-system-lib` (crate was renamed upstream)
- All `dracon_system::` imports updated to `dracon_system_lib::`
- All four `dracon-*` git dependencies replaced with crates.io version requirements
- README overhauled: terminal compatibility table, Space/Enter key behavior, install section, configuration docs
- CONTRIBUTING.md updated: deps table, project structure, terminal spawning pattern

### Fixed
- **Konsole tab spawning** — `qdbus` crashes with SIGSEGV (exit 139) on Konsole 26.04.0/NixOS. Replaced with `dbus-send` which works reliably
- **Generic fallback arg ordering** — Was `["--new-tab", "--workdir", "-e", PATH, CMD...]` (path after `-e` = wrong). Now per-terminal match arms with correct ordering
- **`array:string:` format** — `dbus-send` does NOT use double quotes around elements; `array:string:konsole,--new-tab,--workdir,/path` not `array:string:"konsole",...`

### Known Limitations
- `array:string:` in `dbus-send` uses commas as delimiters — args containing commas will break
- `runCommand` D-Bus API remains blocked by Konsole security policy

## [8.41.0] - Dolphin-Style Sidebar

### Added
- **Sidebar Folder Tree** — Dolphin-style tree rooted at home directory
  - Folders show `▸`/`▾` expansion markers
  - Click **arrow** to expand/collapse only
  - Click **name** to navigate + auto-expand
  - **`Shift+C`** collapses all folders (VSCode-style)
  - **`◄`** indicator shows current folder matching file pane
- **Sidebar Scrolling** — Mouse wheel + keyboard navigation scrolls long sidebars
- **Sidebar Section Toggles** — Settings → General → Sidebar Sections:
  - FOLDERS, FAVORITES, RECENT, STORAGE, REMOTES
  - Each independently show/hide
- **Empty Sidebar Message** — Shows "(All sections hidden. Enable in Settings.)" when all toggled off

### Changed
- **Sidebar title** now shows current directory path (not "FAVORITES")
- **Non-folder items** indented to align with folder icons (consistent sidebar + file pane)
- **Esc key** exits sidebar focus first (standard TUI behavior)
- **Script execution** (Ctrl+R, Ctrl+Enter *(now Ctrl+R only — see v14.32.0)*, context menu) always opens in new tab
- **Hidden files toggle** syncs between sidebar tree and file pane automatically

### Fixed
- Sidebar tree no longer follows file pane navigation (stays rooted at home)
- Settings UI mouse click handler supports all 14 rows
- Settings separator row is non-interactive
- Context menu "Run" opens in new tab (was: new window)
- Editor view sidebar uses same expansion state as Files view
- Removed dead code (`draw_tree_sidebar`, unused `SidebarScope` enum)

## [4.10.0] - Editor Enhancements

### Added
- **Run Files** — Press `Ctrl+Enter` to run the current file *(now Ctrl+R — see v14.32.0)*. Supports:
  - Scripts with shebang (`#!/bin/bash`, `#!/usr/bin/env python3`, etc.)
  - Rust projects (detects `Cargo.toml` by walking up the directory tree)
  - Extension-mapped executables: Python (`python3`), Node.js (`node`), Ruby (`ruby`), Perl (`perl`), PHP (`php`), Lua (`lua`), R (`Rscript`), Go (`go run`)
  - Run opens in a new terminal tab so the editor stays visible
- **Editor Context Menu** — Right-click in the editor area to access:
  - **Editable files**: Cut, Copy, Paste, Undo, Redo, Select All, Save, Run
  - **Read-only files** (Viewer mode, git diffs, binary files): Copy, Select All, Run
- **Unified Clipboard** — Copy/Cut stores in an internal buffer AND syncs to system clipboard. Paste reads from internal buffer first, falls back to system clipboard.
- **Editor Footer Bar** — Shows live cursor position (`Ln X, Col Y`), language, and modified indicator *(now row highlight — see v14.32.0)*
- **Modified Indicator on Tabs** — Amber dot appears on tab labels when a file has unsaved changes *(now row highlight — see v14.32.0)*
- **Auto-Open New Files** — `Ctrl+N` creates a new file and immediately opens it in the editor
- **Save-As Path Sync** — After Save-As, the editor title and tab labels update to reflect the new filename

### Editor Shortcuts
| Key | Action |
|-----|--------|
| `Alt+↑` / `Alt+↓` | Move current line up/down |
| `Ctrl+D` | Duplicate current line |
| `Ctrl+K` | Kill to end of line |
| `Ctrl+U` | Kill to start of line |
| `Ctrl+A` | Select all |
| `Ctrl+Home` | Jump to document start |
| `Ctrl+End` | Jump to document end |

### Changed
- Tab limit increased from 3 to 8 tabs per pane
- Context menu in read-only editor modes now only shows relevant actions (no Cut/Paste/Save)

### Fixed
- Save-As now properly updates the editor path in all cases
- Editor helper functions now correctly prefer the active pane editor over stale full-screen editor state

## Prior Versions

See the git history for earlier changelog entries.
