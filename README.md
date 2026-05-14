# Tiles — TUI File Manager

[![crates.io](https://img.shields.io/crates/v/tiles-tui-file-manager.svg)](https://crates.io/crates/tiles-tui-file-manager)
[![GitHub release](https://img.shields.io/github/v/release/DraconDev/tiles?label=latest)](https://github.com/DraconDev/tiles/releases)

![Tiles](assets/tiles-screenshot.png)

A dual-pane TUI file manager built in Rust. Vim-style navigation, built-in text editor with syntax highlighting, git integration, SSH remote browsing, system monitoring, and smart terminal tab spawning — all in one terminal interface.

## Features

- **Dual-Pane File Manager** — Navigate two directories side-by-side with drag & drop, batch operations, hidden file toggle, column sorting
- **Built-in Text Editor** — Syntax highlighting via syntect, unlimited undo/redo, multi-selection, live search, right-click context menu, line movement (Alt+Up/Down), duplicate (Ctrl+D), kill line (Ctrl+K/U)
- **Smart Terminal Spawning** — Commands open in a new tab in your current terminal window (Konsole via D-Bus, Kitty, Wezterm) instead of a new window
- **Run Files** — Ctrl+Enter runs scripts (shebang), Cargo projects, and extension-mapped executables
- **Git Integration** — Commit history viewer, staged/unstaged diffs, branch info, ahead/behind tracking
- **Remote SSH** — Browse remote filesystems via SSH, SFTP-style file operations, auto-import from `~/.ssh/config`
- **System Monitor** — CPU/memory/disk/network stats with sparklines, process list with kill support
- **Sidebar** — Folder tree, Favorites, Recent folders, Storage devices, SSH remotes. Toggle sections in Settings.
- **Path Input** — Click the breadcrumb bar to edit the path directly
- **Keyboard-first** — Vim-style navigation, command palette (`:`), context menus

### Key Bindings Philosophy

| Key | Files View | Editor View |
|-----|-----------|-------------|
| `Space` | Open file in editor (text) or do nothing (binary/image) | — |
| `Enter` | Enter directory / open file with `xdg-open` | — |
| `Ctrl+Enter` | — | Run current file |
| `Ctrl+D` | System monitor | Duplicate current line |
| `Ctrl+E` | — | Switch to editor |

## Keyboard Shortcuts

> Shortcuts are context-sensitive. The same key does different things depending on which view is active.

### Navigation
| Key | Action |
|-----|--------|
| `h/j/k/l` or arrows | Navigate |
| `Enter` | Open file with xdg-open / enter directory |
| `Space` | Open text file in editor (binary/image files: no-op) |
| `Backspace` | Go to parent directory |
| `Tab` | Switch panes / focus sidebar |
| `Ctrl+H` | Toggle hidden files |
| `:` | Command palette |

### Sidebar
| Key | Action |
|-----|--------|
| `Tab` | Focus sidebar / return to file pane |
| `Up/Down` | Navigate sidebar items |
| `Enter` | Navigate to folder (expand if collapsed) |
| `Space` | Toggle expand/collapse folder |
| `Shift+C` | Collapse all folders |
| `Esc` | Exit sidebar focus |

### Editor
| Key | Action |
|-----|--------|
| `Ctrl+E` | Editor view |
| `Ctrl+Enter` | Run current file |
| `Alt+Up` / `Alt+Down` | Move current line up/down |
| `Ctrl+D` | Duplicate current line |
| `Ctrl+K` | Kill to end of line |
| `Ctrl+U` | Kill to start of line |
| `Ctrl+A` | Select all |
| `Ctrl+Home` | Jump to document start |
| `Ctrl+End` | Jump to document end |
| `Right-click` | Open context menu |

### Other Views
| Key | Action |
|-----|--------|
| `Ctrl+G` | Git history view |
| `Ctrl+L` | Edit current path |
| `q` | Quit |

## Install

### From crates.io (recommended)

```bash
cargo install tiles-tui-file-manager
```

The binary is installed as `tiles`:

```bash
tiles
```

### From source

```bash
git clone https://github.com/DraconDev/tiles
cd tiles
cargo build --release
./target/release/tiles
```

Or use the build script (auto-detects install path):

```bash
./scripts/build.sh
```

### Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/DraconDev/tiles/releases).

## Terminal Compatibility

Tiles detects your terminal and spawns commands as new tabs in the current window when possible:

| Terminal | Tab Spawning Method |
|----------|-------------------|
| Konsole | D-Bus `org.kde.KDBusService.CommandLine` (falls back to `--new-tab` CLI flag) |
| Kitty | `kitty @ launch --type=tab` |
| Wezterm | `wezterm cli spawn --new-window=false` |
| Others | Per-terminal CLI flags (`--new-tab`, `--tab`, etc.) |

> **Konsole note:** `qdbus` crashes on some NixOS/Konsole versions (SIGSEGV exit 139). Tiles uses `dbus-send` instead, which works reliably. The blocked `runCommand` D-Bus API is bypassed via `KDBusService.CommandLine`.

## Optional Dependencies

For drag & drop support (dragging files from Tiles to other apps):
- [dragon](https://github.com/mwh/dragon)
- [ripdrag](https://github.com/nik012003/ripdrag)

Tiles auto-detects these tools and adds a "Drag" option to the context menu.

## Configuration

Settings are stored in `~/.config/tiles/settings.toml`. Toggle sidebar sections, theme, and other preferences from within the app via the Settings modal.

## License

Dracon License v1.1 — see [LICENSE](LICENSE).
