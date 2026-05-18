## Goal
Fix two active P4 bugs in the Tiles TUI file manager.

## Progress

### Bug 1: Default theme still purple on start — FIXED ✅
Root causes (3 found, all fixed):
1. **ACTIVE_STYLE/ACTIVE_THEME statics** initialized with `default_purple()` → changed to `ThemeStyle::default()`
2. **state.json persistence cycle** — old binary kept re-persisting purple theme. Fix: `config.rs` skips `theme_style` when `current == default`
3. **Stale state.json** — cleared purple theme_style. New binary won't re-add it due to fix #2.

Status: Need user to restart with new binary.

### Bug 2: Marquee broken when row already selected — FIXED ✅
Root causes (3 found, all fixed):
1. **handle_click fires immediately on mouseDown** — clears multi-selection before drag starts. Fix: **deferred click pattern** — plain clicks set `pending_click_idx`, resolved on mouseUp only if no drag/marquee occurred. Ctrl/Shift clicks still fire immediately.
2. **File drag threshold < marquee threshold** — file drag (1px) always won. Fix: file drag threshold raised to 3px (`dist_sq >= 9.0`), marquee at 2px (`dist_sq >= 4.0`).
3. **pending_click double-firing** — set for ALL clicks including Ctrl/Shift, causing handle_click to fire twice. Fix: only set for plain clicks.

Additional fixes:
- `pending_click_idx` cleared in marquee mouseUp early return path
- `DragState.pending_click_idx: Option<usize>` field added
- `marquee_start` set for ALL file row clicks (not just empty space)

Status: Need user to restart with new binary.

## Verification checklist
- [x] cargo build clean
- [x] cargo test 78/78 pass
- [x] cargo clippy -- -D warnings clean
- [x] state.json has no theme_style
- [x] No double-fire on Ctrl/Shift clicks
- [x] Empty space click still deselects
- [x] Double-click still works (mouse_last_click updated before return)
- [x] Marquee mouseUp clears pending_click_idx

## Remaining concern
User must restart the app with the new binary. If they're still running the old binary, state.json will keep getting re-persisted with purple.
