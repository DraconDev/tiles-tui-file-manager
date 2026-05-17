# Theme Overhaul — Tiles TUI

## Goal
Eliminate all 261 hardcoded `Color::` usages in `src/ui/` by routing every visible color through the theme system. Add semantic color roles (danger, warning, success, muted, info, selection_fg, file-type colors) to ThemeStyle, add accessor functions, replace all hardcoded colors in UI code, add 5+ new high-contrast theme presets.

## Reference
- Full plan: `docs/THEME_TODO.md`
- Theme system: `src/ui/theme.rs`
- UI files with hardcoded colors: header.rs, footer.rs, mod.rs, modals.rs, small_modals.rs, debug.rs, monitor.rs, misc.rs, git_page.rs, git_view.rs, file_view.rs, sidebar.rs

## Checklist

### Phase 1: Expand ThemeStyle
- [ ] Add fields to ThemeStyle: danger, warning, success, muted, info, selection_fg, file_code, file_config, file_media, file_archive, file_exec
- [ ] Add accessor functions in theme.rs for each new field
- [ ] Wire apply_to_theme() for all new fields
- [ ] Update all 6 existing presets with the new fields (high contrast, no gray mud)
- [ ] cargo clippy + cargo test clean

### Phase 2: Replace hardcoded colors in UI code
- [x] Replace Color::Red (ESC/Back/Quit/Cancel) → theme::danger() — all files
- [x] Replace Color::DarkGray (muted labels) → theme::muted() — all files
- [x] Replace Color::Green (enabled/saved) → theme::success() — all files
- [x] Replace Color::Yellow (warning/modified) → theme::warning() — all files
- [x] Replace Color::Cyan (info) → theme::info() — all files
- [x] Replace Color::Magenta (accent) → theme::accent_secondary() — all files
- [ ] Replace remaining file-type hardcoded colors → theme::file_code/config/media/archive/exec
- [ ] Replace remaining selection highlight hardcoded Rgbs → theme::selection_bg()
- [x] Replace in footer.rs, mod.rs, debug.rs, misc.rs, monitor.rs, settings.rs, file_view.rs, panes/sidebar.rs, sparkline.rs, panes/editor.rs, panes/breadcrumbs.rs, context_menu.rs
- [x] Verify zero remaining hardcoded Color::Red/DarkGray/Green/Yellow/Cyan in src/ui/
- [x] cargo clippy + cargo test clean after each sub-batch

### Phase 3: New theme presets
- [ ] Add preset_nord()
- [ ] Add preset_dracula()
- [ ] Add preset_solarized_dark()
- [ ] Add preset_one_dark()
- [ ] Add preset_tokyo_night()
- [ ] Update style_preset_for_index() in settings_handlers.rs
- [ ] Update draw_style_settings() UI rows in settings.rs
- [ ] Update STYLE_PRESET_COUNT
- [ ] cargo clippy + cargo test clean

### Phase 4: Validation
- [ ] grep audit: no Color::Red, Color::DarkGray, Color::Green etc in src/ui/
- [ ] cargo clippy -- -D warnings clean
- [ ] cargo test clean
- [ ] cargo doc --no-deps clean
- [ ] cargo build --release clean

## Rules
- Run `cargo clippy -- -D warnings && cargo test --bin tiles` after every change
- Keep commits small and descriptive
- Each preset must be high-contrast — semantic roles (danger/warning/success/muted) must be immediately distinguishable
- All new ThemeStyle fields must have values in ALL presets (including existing 6)
- Do NOT change behavior — only color sources