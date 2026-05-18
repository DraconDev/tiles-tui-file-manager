## Goal
Fix two active P4 bugs in the Tiles TUI file manager:

### Bug 1: Default theme still purple on start
`ThemeStyle::default()` was changed to `preset_warm()` (amber), `state.json` was cleared of `theme_style`, but the app still renders purple on launch. Something overrides the default after init.

**Investigation steps:**
1. Check `App::new()` — does it call `set_style_settings()` or `apply_style()` anywhere during construction?
2. Check `setup.rs` — does `load_state()` still read a cached `theme_style` from state.json?
3. Check `theme.rs` — does `style_settings()` / `apply_style()` return or apply the wrong preset?
4. Check `settings.rs` — is the settings index for "Warm (Default)" correct? Does selecting it actually call `preset_warm()`?
5. Check `modals.rs` — when Enter is pressed on Style section, does `style_preset_for_index()` map index 1 to `preset_warm()`?
6. Verify `state.json` is actually empty of `theme_style` (re-check the file)
7. Check if `cyberpunk()` function is used somewhere as a base that gets called instead of `default()`
8. Add a debug print or trace to confirm which theme is being loaded on startup

**Key files:** `src/ui/theme.rs`, `src/setup.rs`, `src/app.rs`, `src/config.rs`, `src/events/modals.rs`, `src/events/settings_handlers.rs`, `src/ui/settings.rs`, `~/.config/tiles/state.json`

### Bug 2: Marquee broken when row already selected
Click on empty space next to a selected row should start marquee drag. Currently nothing happens or the existing selection is lost.

**Investigation steps:**
1. Trace the full mouseDown flow in `src/events/file_manager.rs` — what happens when click is on a file row but NOT in the Name column?
2. Does `fs_mouse_index()` return Some(idx)? If yes, the "empty space click" branch is never reached — the click is treated as a row click.
3. Check: after my edit, does `in_name_col` guard on `handle_click()` mean the click does NOTHING? No handle_click AND no marquee_start = dead click.
4. The fix should be: on mouseDown on a file row outside the Name column, set `marquee_start` (already done in the `in_name_column` else branch) BUT the problem is this code is inside the `fs_mouse_index returns Some(idx)` block. We need to also set marquee tracking for clicks on file rows.
5. On mouseDrag: check if `is_marquee` gets set to true when `marquee_start` is Some and `drag_source` is None and drag distance > threshold.
6. On mouseUp: check if marquee selection commits correctly and existing selection is preserved/toggled.

**Key files:** `src/events/file_manager.rs` (mouseDown at ~line 1100-1230, mouseDrag at ~line 1240-1310, mouseUp at ~line 1310-1360)

**Constraints:**
- `cargo build && cargo test && cargo clippy -- -D warnings` must pass after every change
- Keep commits small and descriptive
- Don't break any existing tests (78 must pass)
