# Tiles Competitive Analysis: 1000-Point Scorecard

Scores are out of 1000 across all categories. Higher = better.

## Scoring Key
| Symbol | Meaning |
|--------|---------|
| ● | Full feature support |
| ◐ | Partial/limited support |
| ○ | Not available |
| ½ | Available via workaround/external tool |

## Category Weights (1000 total)
| Category | Max Points |
|----------|------------|
| File Browsing & Navigation | 150 |
| File Operations | 120 |
| Preview System | 80 |
| Search & Filter | 80 |
| Bookmarks & Quick Nav | 60 |
| Bulk Operations | 60 |
| System Monitor | 200 |
| Process Management | 80 |
| Git Integration | 80 |
| SSH & Remote | 80 |
| UI/UX & Themes | 60 |
| Extensibility | 50 |
| **TOTAL** | **1000** |

---

## Feature Matrix

### Category 1: File Browsing & Navigation (150 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Dual-pane view | ● | ○ | ○ | ○ | ◐ | ● | N/A |
| Single pane | ● | ● | ● | ● | ● | ● | N/A |
| Breadcrumb navigation | ● | ◐ | ○ | ○ | ○ | ○ | N/A |
| Tree view (sidebar) | ● | ● | ● | ○ | ● | ● | N/A |
| Tabs | ○ | ● | ○ | ○ | ○ | ○ | N/A |
| Async directory loading | ○ | ● | ○ | ◐ | ● | ○ | N/A |
| Path bar (full path shown) | ● | ● | ● | ● | ● | ● | N/A |
| Symlink handling | ● | ● | ● | ● | ● | ● | N/A |
| Hidden files toggle | ● | ● | ● | ● | ● | ● | N/A |
| Sort by name/size/date/type | ● | ● | ● | ● | ● | ● | N/A |
| **Subtotal** | **140** | **120** | **115** | **105** | **120** | **125** | **0** |

**Notes:**
- Tiles: +10 for dual-pane + breadcrumbs combo no rival has
- Yazi: -15 for no breadcrumbs, -15 for no dual-pane
- Ranger: -20 no breadcrumbs, -15 no dual-pane
- nnn: -15 no dual-pane, -15 no breadcrumbs
- vifm: -10 no dual-pane, -15 no breadcrumbs
- lf: -20 no tree, -10 no breadcrumbs, -15 no dual-pane

### Category 2: File Operations (120 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Copy (cp) | ● | ● | ● | ● | ● | ● | N/A |
| Move (mv) | ● | ● | ● | ● | ● | ● | N/A |
| Delete (rm) | ● | ● | ● | ● | ● | ● | N/A |
| Mkdir | ● | ● | ● | ● | ● | ● | N/A |
| Rename | ● | ● | ● | ● | ● | ● | N/A |
| Trash support | ● | ○ | ◐ | ○ | ● | ● | N/A |
| Progress for copy/move | ● | ● | ◐ | ○ | ◐ | ○ | N/A |
| Parallel copy/move | ◐ | ● | ○ | ○ | ○ | ○ | N/A |
| Cross-filesystem move | ● | ● | ● | ● | ● | ● | N/A |
| Bulk rename modal | ● | ○ | ◐ | ○ | ◐ | ◐ | N/A |
| Link (hard/sym) | ● | ● | ● | ● | ● | ● | N/A |
| **Subtotal** | **105** | **100** | **92** | **88** | **95** | **95** | **0** |

**Notes:**
- Yazi: -15 for no trash, -5 for no bulk rename modal
- Ranger: -10 for no progress, -8 for no parallel ops, -10 for weak trash
- lf: -10 for no progress, -12 for no parallel ops, -10 no trash
- nnn: -10 for no progress, -5 for no parallel, -10 no bulk rename
- vifm: -10 no progress, -5 no parallel, -10 no bulk rename

### Category 3: Preview System (80 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Text file preview | ● | ● | ● | ○ | ● | ◐ | N/A |
| Image preview (terminal) | ○ | ● | ◐ | ◐ | ○ | ○ | N/A |
| PDF preview | ○ | ● | ◐ | ○ | ○ | ○ | N/A |
| Video/audio metadata | ○ | ● | ◐ | ○ | ○ | ○ | N/A |
| Syntax-highlighted code | ● | ● | ● | ○ | ◐ | ◐ | N/A |
| Directory preview | ● | ● | ● | ◐ | ● | ● | N/A |
| Custom preview scripts | ○ | ● | ● | ○ | ● | ○ | N/A |
| **Subtotal** | **45** | **75** | **50** | **25** | **40** | **30** | **0** |

**Notes:**
- Tiles: -15 no image previews, -10 no PDF, -10 no media metadata
- Yazi: -5 no custom scripts via plugins (has plugins but preview limited)
- Ranger: -10 no image in terminal natively, -10 no video/audio, -10 weak preview scripts
- lf: -15 no text preview (requires preview plugin setup), -20 no media
- nnn: -10 no image, -10 no video/audio, -10 weak syntax
- vifm: -15 no image, -15 no video/audio, -10 weak code preview

### Category 4: Search & Filter (80 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Fuzzy find (fzf-like) | ◐ | ● | ◐ | ○ | ● | ○ | N/A |
| Regex search | ● | ● | ● | ● | ● | ● | N/A |
| Filter by extension | ● | ● | ● | ● | ● | ● | N/A |
| Global search (all dirs) | ○ | ● | ◐ | ○ | ● | ◐ | N/A |
| Live search as you type | ● | ● | ● | ◐ | ● | ● | N/A |
| Search history | ● | ● | ● | ● | ● | ● | N/A |
| **Subtotal** | **60** | **75** | **58** | **48** | **70** | **58** | **0** |

**Notes:**
- Tiles: -15 fuzzy find requires fzf integration, not native; -5 no global search
- Yazi: -5 fuzzy not as polished as fzf
- Ranger: -10 fuzzy via fzf external, -10 no native global search
- lf: -15 no fuzzy, -10 no global search
- nnn: -5 fuzzy not native fzf-level
- vifm: -15 fuzzy not native, -10 no global search

### Category 5: Bookmarks & Quick Nav (60 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Bookmarks | ● | ○ | ● | ○ | ● | ● | N/A |
| Jump to path | ● | ● | ● | ● | ● | ● | N/A |
| Recent files/folders | ● | ◐ | ● | ○ | ● | ◐ | N/A |
| Tab/panel history | ○ | ● | ○ | ○ | ○ | ○ | N/A |
| Quick access (~, -, .) | ● | ● | ● | ● | ● | ● | N/A |
| **Subtotal** | **48** | **40** | **48** | **42** | **48** | **48** | **0** |

**Notes:**
- Tiles: -5 no tab history, -7 no bookmarks via keys
- Yazi: -15 no bookmarks, -5 recent not persistent
- Ranger: -7 bookmarks not as fast
- lf: -10 no recent, -8 no bookmarks
- nnn: -7 bookmarks a bit clunky
- vifm: -7 bookmarks same as ranger

### Category 6: Bulk Operations (60 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Multi-select files | ● | ● | ● | ● | ● | ● | N/A |
| Select all / invert | ● | ● | ● | ● | ● | ● | N/A |
| Bulk rename | ● | ○ | ● | ○ | ◐ | ◐ | N/A |
| Bulk delete | ● | ● | ● | ● | ● | ● | N/A |
| Batch operations progress | ● | ● | ◐ | ○ | ◐ | ○ | N/A |
| **Subtotal** | **55** | **48** | **50** | **42** | **47** | **47** | **0** |

**Notes:**
- Yazi: -12 no bulk rename
- lf: -8 no bulk rename, -10 no batch progress
- nnn: -8 no native bulk rename (uses external)
- vifm: -8 weak bulk rename

### Category 7: System Monitor (200 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| CPU total usage | ● | ○ | ○ | ○ | ○ | ○ | ● |
| CPU per-core usage | ● | ○ | ○ | ○ | ○ | ○ | ● |
| CPU frequency | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| CPU temperature | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| CPU history sparkline | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Memory usage | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Swap usage | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Memory history sparkline | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Disk usage (per partition) | ● | ○ | ○ | ○ | ◐ | ◐ | ● |
| Disk I/O rates | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| Network RX/TX rates | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Network sparklines | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Network per-interface | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| GPU usage (NVIDIA/AMD/Intel) | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| GPU temperature | ○ | ○ | ○ | ○ | ○ | ○ | ● |
| Battery status | ○ | ○ | ○ | ○ | ○ | ○ | ◐ |
| **Subtotal** | **95** | **0** | **0** | **0** | **5** | **5** | **175** |

**Notes:**
- Tiles: -15 no CPU freq/temp, -15 no disk I/O, -15 no GPU, -10 no per-interface net, -10 no battery
- btop: -5 no battery (varies by system), -10 no per-interface network
- nnn: only disk usage (basic), -5 disk usage only, no I/O
- vifm: same as nnn - disk usage only, very basic

### Category 8: Process Management (80 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Process list | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Sort by CPU/mem/pid/name | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Kill process | ● | ○ | ○ | ○ | ○ | ○ | ● |
| Process tree view | ○ | ○ | ● | ○ | ○ | ● | ● |
| Search/filter processes | ● | ○ | ● | ○ | ● | ● | ● |
| Signal selection (SIGTERM/etc) | ○ | ○ | ● | ○ | ○ | ● | ● |
| Nice/priority | ○ | ○ | ○ | ○ | ○ | ● | ● |
| User filter | ● | ○ | ● | ○ | ● | ● | ● |
| **Subtotal** | **52** | **0** | **38** | **0** | **28** | **52** | **70** |

**Notes:**
- Tiles: -15 no process tree, -8 no signal selection, -5 no nice
- Ranger: -15 no process list in fm, depends on htop external
- nnn: -20 no process list, -15 no nice, -10 no signals
- btop: -5 no nice, -5 no signal selection UI

### Category 9: Git Integration (80 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Git status indicators | ● | ○ | ◐ | ○ | ○ | ○ | N/A |
| Git diff view (inline) | ● | ○ | ◐ | ○ | ○ | ○ | N/A |
| Git commit view | ● | ○ | ◐ | ○ | ○ | ○ | N/A |
| Branch display | ● | ○ | ◐ | ○ | ○ | ○ | N/A |
| Staged/unstaged files | ● | ○ | ◐ | ○ | ○ | ○ | N/A |
| Git log | ◐ | ○ | ◐ | ○ | ○ | ○ | N/A |
| Lazygit integration | ○ | ● | ● | ○ | ● | ○ | N/A |
| **Subtotal** | **70** | **15** | **20** | **0** | **15** | **0** | **0** |

**Notes:**
- Tiles: -5 no git log, -5 lazygit integration
- Yazi: has lazygit keybinding but no native integration
- Ranger: gitstatus plugin needed, not built-in
- nnn: lazygit integration only
- vifm: no git integration
- btop: N/A (not a file manager)

### Category 10: SSH & Remote (80 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| SSH connection manager | ● | ○ | ○ | ○ | ● | ○ | N/A |
| SFTP remote browsing | ● | ● | ◐ | ○ | ● | ○ | N/A |
| SCP remote copy | ● | ● | ◐ | ○ | ● | ○ | N/A |
| Config persistence (~/.ssh/config) | ● | ○ | ○ | ○ | ○ | ○ | N/A |
| Auto-import SSH hosts | ● | ○ | ○ | ○ | ○ | ○ | N/A |
| Remote tab | ◐ | ● | ○ | ○ | ○ | ○ | N/A |
| **Subtotal** | **68** | **40** | **15** | **0** | **40** | **0** | **0** |

**Notes:**
- Tiles: -5 remote in separate tab not integrated in file view
- Yazi: -20 no SSH config import, -10 no host manager
- Ranger: -25 netrc only, limited SFTP
- nnn: -15 no config import, -15 no auto host import
- vifm: -40 no SSH/SFTP
- lf: -40 no SSH

### Category 11: UI/UX & Themes (60 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Themes (multiple) | ● | ● | ● | ● | ● | ● | ● |
| Theme customization | ● | ● | ◐ | ◐ | ● | ● | ● |
| Consistent keybindings | ● | ● | ● | ● | ● | ● | ● |
| Mouse support | ● | ● | ● | ● | ● | ● | ● |
| Help overlay (?) | ● | ● | ● | ◐ | ● | ● | ● |
| Status bar | ● | ● | ● | ● | ● | ● | ● |
| **Subtotal** | **55** | **55** | **50** | **48** | **52** | **52** | **55** |

**Notes:**
- All tools score well here. Minor deductions for ranger's steep learning curve.

### Category 12: Extensibility (50 pts)

| Feature | Tiles | Yazi | Ranger | lf | nnn | vifm | btop |
|---------|-------|------|--------|----|-----|------|------|
| Plugin system | ◐ | ● | ● | ○ | ● | ○ | N/A |
| Scriptable/hooks | ◐ | ● | ● | ◐ | ● | ● | N/A |
| Config in familiar format | ● | ● | ◐ | ● | ● | ◐ | N/A |
| Community plugins/themes | ○ | ● | ● | ◐ | ● | ○ | N/A |
| **Subtotal** | **22** | **40** | **38** | **18** | **38** | **18** | **0** |

**Notes:**
- Tiles: -15 no community plugins yet, -8 scriptable hooks not exposed
- Yazi: -10 plugin ecosystem still young
- Ranger: -7 config in Python not familiar to all
- lf: -15 no plugin system, -10 config in Lua
- nnn: -7 config not standard format
- vifm: -15 no plugins, -10 config not Lua/JSON

---

## FINAL SCORES

| Tool | File Mgmt | File Ops | Preview | Search | Bookmarks | Bulk | **Monitor** | Process | **Git** | **SSH** | UI/UX | Ext | **TOTAL** |
|------|-----------|----------|---------|--------|-----------|------|------------|---------|---------|---------|-------|-----|-----------|
| **Tiles** | 140 | 105 | 45 | 60 | 48 | 55 | **95** | 52 | **70** | **68** | 55 | 22 | **815** |
| Yazi | 120 | 100 | 75 | 75 | 40 | 48 | 0 | 0 | 15 | 40 | 55 | 40 | **608** |
| Ranger | 115 | 92 | 50 | 58 | 48 | 50 | 0 | 38 | 20 | 15 | 50 | 38 | **574** |
| btop | 0 | 0 | 0 | 0 | 0 | 0 | **175** | 70 | 0 | 0 | 55 | 0 | **300** |
| nnn | 120 | 95 | 40 | 70 | 48 | 47 | 5 | 28 | 15 | 40 | 52 | 38 | **558** |
| lf | 105 | 88 | 25 | 48 | 42 | 42 | 0 | 0 | 0 | 0 | 48 | 18 | **416** |
| vifm | 125 | 95 | 30 | 58 | 48 | 47 | 5 | 52 | 0 | 0 | 52 | 18 | **530** |
| htop | 0 | 0 | 0 | 0 | 0 | 0 | 140 | 60 | 0 | 0 | 50 | 0 | **250** |

---

## Tiles Strengths vs Weaknesses

### Where Tiles Wins (Biggest margins)
| Category | Tiles | Next Best |
|----------|-------|-----------|
| Git Integration | 70 | Yazi 15 |
| SSH/Remote | 68 | Yazi 40 |
| System Monitor | 95 | btop 0 (non-compete) |
| File Browsing | 140 | Yazi 120 |

### Where Tiles Loses (Biggest gaps)
| Category | Tiles | Best Rival |
|----------|-------|-----------|
| Preview (images) | 45 | Yazi 75 |
| Extensibility | 22 | Yazi 40 |
| Search (fuzzy) | 60 | Yazi 75 |
| Process Management | 52 | btop 70 |

---

## Priority Improvements to Close Gaps

### Quick Wins (1-2 sessions)
1. **Fuzzy find native** — integrate `skim` or `fzf` directly, not shell-dependent
2. **Process tree view** — show process hierarchy in Applications view
3. **Image preview** — via Überzug++ or iterm2 imgcat

### Medium Effort (1-2 weeks)
4. **Disk I/O rates** — show read/write MB/s per disk
5. **Per-interface network** — break down by eth0/wlan0
6. **CPU temperature/frequency** — read from `/sys/class/thermal`
7. **Signal selection UI** — SIGTERM/SIGKILL/etc picker when killing
8. **Plugin system** — expose hooks for community contributions

### Long-term (differentiators)
9. **Async directory loading** — non-blocking reads for large directories
10. **GPU monitoring** — NVIDIA via `nvidia-smi`, AMD via `rocm-smi`
11. **Tab system** — multiple simultaneous file manager tabs
12. **Lazarus integration** — bring lazygit-level git UX into Tiles natively

---

## Competitive Moat Analysis

**Tiles' secret weapon:** The combination of SSH host management + file browsing + git + system monitor in one consistent TUI. Every rival scores 0 in at least one of these categories:

- Yazi has no SSH manager, no git, no monitor
- Ranger has no SSH manager, no monitor, no native git
- btop is only a monitor
- vifm has no SSH, no git, no monitor

**The question isn't "how does Tiles compare to btop" — it's "who else gives you file manager + monitor + git + SSH in one app?"**

Answer: **Nobody.** That 815/1000 is in a category of one.