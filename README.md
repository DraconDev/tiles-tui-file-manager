# Tiles

![Tiles](assets/tiles-screenshot.png)

A high-performance terminal file manager built in Rust. Modular pane system, integrated text editor, git awareness, remote SSH file browsing, and system monitoring — all in one TUI.

## Features

- **File Manager** — Dual-pane navigation, drag & drop, batch operations, hidden file toggle, column sorting
- **Text Editor** — Syntax highlighting via `syntect`, unlimited undo/redo, multi-selection, live search, right-click context menu
- **Editor Shortcuts** — Alt+↑/↓ move lines, Ctrl+D duplicate, Ctrl+K/U kill lines, Ctrl+A select all, Ctrl+Home/End jump to edges
- **Run Files** — Ctrl+Enter runs scripts (shebang), Cargo projects, and extension-mapped executables
- **Git Integration** — Commit history viewer, staged/unstaged diffs, branch info, ahead/behind tracking
- **Remote SSH** — Browse remote filesystems via SSH, SFTP-style file operations
- **System Monitor** — CPU, memory, disk, network stats, process list
- **Sidebar** — Dolphin-style folder tree rooted at home, Favorites, Recent folders, Storage devices, SSH remotes. Toggle sections in Settings.
- **Path Input** — Click the breadcrumb bar to edit the path directly, copy on click
- **Context Menus** — Right-click on files, folders, or empty space to access file operations, terminal options, and more
- **Keyboard-first** — Vim-style navigation, command palette (`:`), context menus

## Keyboard Shortcuts

> **Note:** Shortcuts are context-sensitive. `Ctrl+D` duplicates lines in Editor view, but opens System Monitor in Files view.

### Navigation
| Key | Action |
|-----|--------|
| `h/j/k/l` or arrows | Navigate |
| `Enter` | Open file / enter directory |
| `Backspace` | Go to parent directory |
| `Tab` | Switch panes / focus sidebar |
| `Ctrl+H` | Toggle hidden files |
| `:` | Command palette |

### Terminal Tabs
| Key | Action |
|-----|--------|
| `Ctrl+N` | Open new terminal **tab** in current Konsole window |
| `Ctrl+T` | Open new **file tab** in Tiles (in-app) |
| `Ctrl+K` | Open new terminal **window** |

> **Note:** Ctrl+N requires `busctl` (systemd) and "Run all Konsole windows in a single process" enabled in Konsole settings.

### Sidebar
| Key | Action |
|-----|--------|
| `Tab` | Focus sidebar / return to file pane |
| `↑/↓` | Navigate sidebar items |
| `Enter` | Navigate to folder (expand if collapsed) |
| `Space` | Toggle expand/collapse folder |
| `Shift+C` | Collapse all folders |
| `Esc` | Exit sidebar focus |
| `Mouse click` | Click arrow to expand/collapse, click name to navigate |
| `Mouse wheel` | Scroll sidebar |

### Editor
| Key | Action |
|-----|--------|
| `Ctrl+E` | Editor view |
| `Ctrl+Enter` | Run current file |
| `Alt+↑` / `Alt+↓` | Move current line up/down |
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
| `Ctrl+D` | System monitor view |
| `Ctrl+L` | Edit current path |
| `q` | Quit |

## Remote Servers

Tiles supports browsing remote filesystems via SSH. Manage server bookmarks in **Settings → Remotes** (`Ctrl+G` then `5`).

### Adding Servers
- **Manually**: Press `A` in Remotes settings to add a new server bookmark
- **Import TOML**: Press `I` to import a `servers.toml` file
- **Import SSH Config**: Press `S` to import directly from your `~/.ssh/config` file

### Tilde Expansion
Paths containing `~` are automatically expanded:
- `~` or `~/path` → resolves to your home directory
- `~user/path` → resolves to `/home/user/path`

This applies to SSH key paths in both manual entry and SSH config import.

### Key Permission Validation
SSH private keys must have restrictive permissions (≤ 0600). Tiles warns you if a key is too permissive and shows the exact `chmod` command needed.

### Remotes Shortcuts (in Settings)
| Key | Action |
|-----|--------|
| `A` | Add new server |
| `E` | Edit selected server |
| `D` | Delete selected server |
| `I` | Import from TOML |
| `S` | Import from SSH config |
| `X` | Export to TOML |
| `T` | Edit servers.toml |
| `Enter` | Connect to selected server |

## Install

```bash
git clone https://github.com/DraconDev/tiles
cd tiles
./install.sh
```

Or build manually:

```bash
cargo build --release
./target/release/tiles
```

## Optional Dependencies

For drag & drop support (dragging files from Tiles to other apps):
- [dragon](https://github.com/mwh/dragon)
- [ripdrag](https://github.com/nik012003/ripdrag)

Tiles auto-detects these tools and adds a "Drag" option to the context menu.

## Download Pre-compiled Binaries

Download the latest binaries for Linux, macOS, and Windows from [GitHub Releases](https://github.com/DraconDev/tiles/releases).

## License

Dracon License v1.1 — see [LICENSE](LICENSE).
