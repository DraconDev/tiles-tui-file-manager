## Goal
Fix two active P4 bugs in the Tiles TUI file manager.

## Progress

### Bug 1: Default theme still purple on start — FIXED ✅
Root causes (3 found, all fixed):
1. **ACTIVE_STYLE/ACTIVE_THEME statics** initialized with `default_purple()` → changed to `ThemeStyle::default()`
2. **state.json persistence cycle** — old binary kept re-persisting purple theme. Fix: `config.rs` skips `theme_style` when `current == default`
3. **Stale state.json** — cleared purple theme_style. New binary won't re-add it due to fix #2.

### Bug 2: Marquee broken when row already selected — FIXED ✅
Root causes (3 found, all fixed):
1. **handle_click fires immediately on mouseDown** → deferred click pattern (`pending_click_idx`)
2. **File drag threshold < marquee threshold** → raised file drag to 3px
3. **pending_click double-firing on Ctrl/Shift** → only set for plain clicks

Additional: `DragState.pending_click_idx` field, marquee_start for ALL row clicks, cleanup in all mouseUp paths.

## Verification checklist
- [x] cargo build clean
- [x] cargo test 78/78 pass
- [x] cargo clippy -- -D warnings clean
- [x] state.json has no theme_style
- [x] No double-fire on Ctrl/Shift clicks
- [x] Empty space click still deselects
- [x] Double-click still works
- [x] Marquee mouseUp clears pending_click_idx

## Commits
- d7e34a8b fix: default theme warm amber, marquee works with existing selection
- 5ea688c9 fix: theme persistence cycle + deferred click for marquee
- f8d7047b fix: double-fire handle_click on Ctrl/Shift, missing pending_click_idx cleanup
- 27b9d275 docs: update TODO — P4 bugs marked fixed

## Status: COMPLETE
All code changes done. User must restart app with new binary to verify.
