# Fix ESC/Back/Quit/Cancel buttons — use accent_primary not danger

## Problem
ESC/Back/Quit/Cancel navigation buttons all use `theme::danger()` (always red).
They should use `theme::accent_primary()` so they follow the active theme's accent color.
In Dracula, ESC should be purple; in Nord, frost blue; in Warm, amber — NOT always red.

`theme::danger()` should be reserved for actual destructive/warning actions:
- Delete modal "Yes" buttons
- Kill process (SIGKILL)
- Error messages
- "Disabled" boolean status in settings
- Red file highlight color (highlight code 1)
- Git "D" (deleted) status
- Low disk space warning

## Changes needed

### Navigation buttons → accent_primary()
These `theme::danger()` calls should become `theme::accent_primary()`:

**ESC button bg + Back/Quit/Cancel fg:**
- `settings.rs:44,47` — Esc bg, Back fg
- `debug.rs:37,40` — Esc bg, Back fg
- `monitor.rs:36,39` — Esc bg, Back fg
- `git_page.rs:134,137` — Esc bg, Back fg
- `git_view.rs:117,119` — Esc bg, Back to Git fg
- `mod.rs:181,182` — HotkeyHint Esc Back, ^Q Quit
- `footer.rs:132,142` — HotkeyHint ^Q Quit, Esc Back
- `modals.rs:78` — HotkeyHint Esc Cancel
- `small_modals.rs:77` — Cancel span fg
- `misc.rs:91,154` — Esc bg (both confirm modals)
- `panes/sidebar.rs` — any Back fg

### Keep as theme::danger() — actual danger/warning actions:
- `footer.rs:42` — Warning/Danger stat bar color
- `footer.rs:202` — hidden toggle (showing "OFF" in red)
- `modals.rs:292,323,327` — delete modal yes_style
- `header.rs:165,182,272,289` — +pending count, branch danger
- `small_modals.rs:60,135,137` — SIGKILL, cancel_style hover
- `misc.rs:102,116` — error message, error border
- `misc.rs:182` — (1, " R ", danger) highlight swatch
- `monitor.rs:543,708` — CPU color danger
- `settings.rs:534` — Some(false) => Disabled status
- `settings.rs:622` — Legacy Red label color
- `file_view.rs:286` — highlight code 1 (red)
- `git_page.rs` — git "D" deleted status (already danger, keep)
- `panes/sidebar.rs` — low disk space warning

### Also fix: Legacy Red preset doesn't feel "red"
The preset label in settings uses `theme::danger()` which is the same red
as every preset. Should use the preset's own accent color like the others.

## Rules
- Run `cargo clippy -- -D warnings && cargo test --bin tiles` after changes
- Keep commits small
- Do NOT change behavior — only color sources