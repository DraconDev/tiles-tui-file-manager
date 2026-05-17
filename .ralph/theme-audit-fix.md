# Theme Audit Fix — All items

## P2 — Structural color routing ✅ DONE
(Previously completed)

## P1 — Item 2: Replace `Color::White` fg → `theme::fg()` ✅ DONE

### Replacements completed
- **monitor.rs** (15 occurrences): CPU %, GHz, RAM/SWP stats, disk names, disk I/O MB/s, network RX/TX rates, total net, interface names, spacer, system info line, nav hover
- **small_modals.rs** (5 occurrences): PID, process name, signal list items (normal + danger), confirm dialog items
- **modals.rs** (2 occurrences): command palette selected item, confirmation "NO" button default style
- **misc.rs** (1 occurrence): color swatch label (code 0)
- **footer.rs** (2 occurrences): sidebar item name, summary text with sidebar_focus
- **git_view.rs** (2 occurrences): commit author, commit subject
- **file_view.rs** (1 occurrence): multi-selected file name
- **panes/sidebar.rs** (1 occurrence): mounted disk name default style
- **sparkline.rs** (1 occurrence): default color field — already fixed (was `theme::fg()`)

### Unused import cleanup
- Removed `use crate::ui::theme::THEME;` from mod.rs (unused — `mod theme` provides `theme::`)
- Removed `THEME` from debug.rs import line
- Removed `THEME` from context_menu.rs import line

### Verification
- `cargo clippy -- -D warnings` — passes
- `cargo test --bin tiles` — 72 tests pass
- Only remaining `Color::White` in `src/ui/` is in git_page.rs (bg for unknown git status badge — structural, not fg)
