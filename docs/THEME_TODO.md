# Theme Overhaul TODO

## Status: ✅ COMPLETE (Phase 1-4)

All 4 phases completed. 261 hardcoded colors replaced with theme accessors.
5 new high-contrast presets added. Zero hardcoded semantic colors remain.

---

## Phase 1: Expand ThemeStyle to cover all DraconTheme fields

### 1.1 Add missing fields to ThemeStyle
- [ ] `selection_fg: RgbColor` (currently hardcoded `Color::Black`)
- [ ] `danger: RgbColor` (ESC, Quit, delete, kill — currently `Color::Red`)
- [ ] `warning: RgbColor` (caution, unsaved indicator — currently `Color::Yellow`)
- [ ] `success: RgbColor` (enabled, saved, connected — currently `Color::Green`)
- [ ] `muted: RgbColor` (dim labels, hints, DarkGray replacements — currently `Color::DarkGray`)
- [ ] `info: RgbColor` (informational, cyan-type — currently `Color::Cyan`)
- [ ] `file_code: RgbColor` (already in DraconTheme, not in ThemeStyle)
- [ ] `file_config: RgbColor` (already in DraconTheme, not in ThemeStyle)
- [ ] `file_media: RgbColor` (already in DraconTheme, not in ThemeStyle)
- [ ] `file_archive: RgbColor` (already in DraconTheme, not in ThemeStyle)
- [ ] `file_exec: RgbColor` (already in DraconTheme, not in ThemeStyle)

### 1.2 Add theme accessor functions
- [ ] `theme::danger() -> Color`
- [ ] `theme::warning() -> Color`
- [ ] `theme::success() -> Color`
- [ ] `theme::muted() -> Color`
- [ ] `theme::info() -> Color`
- [ ] `theme::selection_fg() -> Color`
- [ ] `theme::file_code() -> Color`
- [ ] `theme::file_config() -> Color`
- [ ] `theme::file_media() -> Color`
- [ ] `theme::file_archive() -> Color`
- [ ] `theme::file_exec() -> Color`

### 1.3 Wire apply_to_theme() for all new fields
- [ ] Update `ThemeStyle::apply_to_theme()` to set all 15+ fields
- [ ] Update all 6 existing presets to provide the new fields
- [ ] Each preset should have high-contrast semantic colors (not gray mud)

---

## Phase 2: Replace hardcoded colors in UI code

### 2.1 ESC/Back/Quit/Cancel buttons (~15 occurrences)
Files: `header.rs`, `footer.rs`, `mod.rs`, `modals.rs`, `small_modals.rs`,
`debug.rs`, `monitor.rs`, `misc.rs`, `git_page.rs`, `git_view.rs`

Replace `Color::Red` → `theme::danger()` for:
- [ ] "Esc" / "Back" button labels and backgrounds
- [ ] "^Q" / "Quit" hints
- [ ] Delete confirmation borders
- [ ] Signal kill labels

### 2.2 DarkGray / muted labels (~20 occurrences)
Files: `header.rs`, `footer.rs`, `modals.rs`, `small_modals.rs`, `mod.rs`

Replace `Color::DarkGray` → `theme::muted()` for:
- [ ] Breadcrumb separator "›"
- [ ] Truncation ellipsis "…"
- [ ] Hint labels ("Enter", "Esc" in small modals)
- [ ] Key display labels in modals
- [ ] Timestamp/date in file list

### 2.3 Status colors (~10 occurrences)
Replace:
- [ ] `Color::Green` (enabled/saved) → `theme::success()`
- [ ] `Color::Yellow` (warning/modified) → `theme::warning()`
- [ ] `Color::Cyan` (info/author) → `theme::info()`
- [ ] `Color::Magenta` (replace mode) → `theme::accent_secondary()` or new field

### 2.4 File-type colors in sidebar and file list
Files: `sidebar.rs`, `file_view.rs`, `footer.rs`

Replace hardcoded file category colors:
- [ ] `.rs`, `.py`, `.js` etc → `theme::file_code()`
- [ ] `.toml`, `.yaml`, `.json` etc → `theme::file_config()`
- [ ] `.png`, `.mp3` etc → `theme::file_media()`
- [ ] `.zip`, `.tar` etc → `theme::file_archive()`
- [ ] Executables → `theme::file_exec()`

### 2.5 Monitor-specific colors
Files: `monitor.rs`, `ui/monitor.rs`

- [ ] `gauge_danger()` and `gauge_warning()` are already themed ✅
- [ ] `monitor_label()`, `monitor_dim()`, `monitor_separator()` already themed ✅
- [ ] Row even/odd colors → add to theme or keep as monitor-specific

### 2.6 Git page colors
Files: `git_page.rs`, `git_view.rs`

- [ ] Status colors: M=yellow, A/??=green, D=red, R=cyan → theme semantic colors
- [ ] Ref label colors: HEAD→green, tag→magenta, origin→cyan → theme colors
- [ ] Stats: +green, -red → `theme::success()`, `theme::danger()`

### 2.7 Selection highlight backgrounds
Files: `git_page.rs`, `file_view.rs`, `sidebar.rs`

- [ ] Selection bg `Color::Rgb(40,40,50)` → `theme::selection_bg()`
- [ ] Multi-selection `Color::Rgb(78,58,112)` → new theme field or `selection_bg()` variant
- [ ] Hover drop target → `theme::accent_secondary()`

---

## Phase 3: New theme presets (aim for high contrast)

Each new preset must define ALL ThemeStyle fields with strong, distinct colors.
No gray mud. Semantic roles (danger/warning/success/muted) must be immediately
distinguishable from each other.

### 3.1 New presets to add
- [ ] **Nord** — Nord palette blues/whites, frost accents
- [ ] **Dracula** — dark purple bg, green/pink/cyan accents
- [ ] **Solarized Dark** — base03 bg, yellow/orange/cyan accents
- [ ] **One Dark** — Atom One Dark purple/teal/red
- [ ] **Tokyo Night** — dark blue bg, blue/purple/green accents

### 3.2 Review existing presets for completeness
- [ ] `preset_warm` — verify all new fields defined, high contrast
- [ ] `preset_cool` — verify all new fields defined, high contrast
- [ ] `preset_forest` — verify all new fields defined, high contrast
- [ ] `preset_sunset` — verify all new fields defined, high contrast
- [ ] `preset_mono` — special case: grayscale but still needs contrast
- [ ] `preset_legacy_red` — verify all new fields defined

### 3.3 Settings UI updates
- [ ] Add new presets to `style_preset_for_index()` in settings_handlers.rs
- [ ] Add new preset rows to settings UI in `draw_style_settings()`
- [ ] Update `STYLE_PRESET_COUNT` constant

---

## Phase 4: Validation

- [ ] `cargo clippy -- -D warnings` clean after each phase
- [ ] `cargo test` passes after each phase
- [ ] Visual review: each preset should be high-contrast and distinguishable
- [ ] Verify no hardcoded `Color::Red`, `Color::Green`, `Color::DarkGray` etc.
  remain in `src/ui/` (search with `grep -rn 'Color::Red\|Color::Green\|Color::DarkGray' src/ui/`)
- [ ] Verify ESC/Quit buttons change color when switching themes
- [ ] Verify file-type colors change when switching themes

---

## Out of scope
- Background color theming (currently always pure black)
- Foreground color theming (currently always pure white)
- Font/typography theming (not supported by ratatui)
- Per-view theme overrides
