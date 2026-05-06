# Changelog

All notable changes to this project will be documented in this file.

## [10.61.0] - Terminal & Context Menu Fixes

### Fixed
- **Terminal Tab Spawning** — Replaced `qdbus` with `busctl` for Konsole D-Bus calls
  - `qdbus` crashes with SIGSEGV (exit 139) on Konsole 26.04.0+, causing "Qt Multimedia SymbolResolver" UI errors
  - `busctl` (systemd) has no Qt dependency and works reliably
  - Ctrl+N now correctly opens a tab instead of a new window
- **EmptySpace Context Menu** — Right-click below the last file now shows context menu
  - Provides NewFile, NewFolder, Paste, ToggleHidden, CollapseAll, TerminalTab, TerminalWindow, SystemMonitor
  - Previously only worked when clicking on actual files/folders

### Added
- **Sidebar Keyboard in Editor View** — Space/Enter/C/Up/Down now work in sidebar when in Editor view
- **Open File Highlight** — Open files in sidebar now show full-row background highlight instead of dot

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
- **Script execution** (Ctrl+R, Ctrl+Enter, context menu) always opens in new tab
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
- **Run Files** — Press `Ctrl+Enter` to run the current file. Supports:
  - Scripts with shebang (`#!/bin/bash`, `#!/usr/bin/env python3`, etc.)
  - Rust projects (detects `Cargo.toml` by walking up the directory tree)
  - Extension-mapped executables: Python (`python3`), Node.js (`node`), Ruby (`ruby`), Perl (`perl`), PHP (`php`), Lua (`lua`), R (`Rscript`), Go (`go run`)
  - Run opens in a new terminal tab so the editor stays visible
- **Editor Context Menu** — Right-click in the editor area to access:
  - **Editable files**: Cut, Copy, Paste, Undo, Redo, Select All, Save, Run
  - **Read-only files** (Viewer mode, git diffs, binary files): Copy, Select All, Run
- **Unified Clipboard** — Copy/Cut stores in an internal buffer AND syncs to system clipboard. Paste reads from internal buffer first, falls back to system clipboard.
- **Editor Footer Bar** — Shows live cursor position (`Ln X, Col Y`), language, and modified indicator (`●`)
- **Modified Indicator on Tabs** — Amber dot appears on tab labels when a file has unsaved changes
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
