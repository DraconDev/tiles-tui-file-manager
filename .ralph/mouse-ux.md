# Mouse UX — Marquee Drag + Undo Close Tab

## Status: ✅ COMPLETE

### FN-043 — Marquee drag selection ✅
- Commit: `19dfcb69`
- Click+drag in file list (row >= 3) draws rounded rect overlay
- `DragState` extended: `is_marquee`, `marquee_start`, `marquee_end`
- `MarqueeRect` helper + `marquee_rect()` / `clear_marquee()` methods
- On mouseUp: all files within rect selected (Ctrl+drag = toggle)
- Escape cancels active marquee
- `draw_marquee_rect()` in `ui/misc.rs` with accent_primary border
- 4 unit tests (normalize, none-when-inactive, clear, same-point)

### FN-044 — Undo close tab (Ctrl+Shift+T) ✅
- Commit: `19dfcb69`
- Ctrl+W saves tab info (path, pane_index) to `app.nav.closed_tabs`
- VecDeque, max 10 entries (oldest evicted)
- Ctrl+Shift+T reopens last closed tab in original pane
- Status message on restore
- 2 unit tests (push/pop, max cap)

### Verification
- 78 tests pass
- `cargo clippy -- -D warnings` clean
- `cargo build` clean
