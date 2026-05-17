# Theme Hardcoded Color Audit — ✅ COMPLETE

All 16 TODO items resolved. Only 12 `Color::` references remain in `src/ui/`,
all of which are intentional and correct.

## What was fixed

### P1 — Semantic fixes
- [x] `Color::Black` fg on themed bg → `theme::selection_fg()` (67 occurrences, 14 files)
- [x] `Color::White` fg → `theme::fg()` (32 occurrences, 9 files)
- [x] `Color::Gray` in sidebar → `theme::muted()` (2 occurrences)
- [x] `Color::LightRed`/`Color::Gray` in settings preset labels → preset accent colors
- [x] `Color::Blue` in misc.rs bookmark button → kept as literal blue swatch (correct)
- [x] `Color::White` bg in modals.rs → `theme::selection_bg()`

### P2 — Structural color routing
- [x] Added `theme::border_subtle()` (10 occurrences of Rgb(40,45,55))
- [x] `Rgb(60, 65, 75)` → `theme::monitor_label()` (6 occurrences)
- [x] `Rgb(0,0,0)`/`Rgb(8,8,12)` bg → `theme::bg()` (5 occurrences)
- [x] `Rgb(190,190,200)` debug text → `theme::fg()` (1 occurrence)
- [x] `Rgb(60,60,70)` line numbers → `theme::muted()` (1 occurrence)
- [x] `Rgb(100,100,110)` breadcrumbs → `theme::muted()` (2 occurrences)
- [x] Footer stat bar → `theme::stat_cpu_*()` (8 occurrences)
- [x] `Rgb(78,58,112)` multi-select → `theme::selection_alt_bg()` (1 occurrence)
- [x] `Rgb(40,40,50)` git rows → `theme::row_alt_bg()` (2 occurrences)

### P3 — Consistency polish
- [x] Added `theme::fg()`/`theme::bg()` accessors, replaced `THEME.fg`/`THEME.bg` (22 occurrences)
- [x] Fixed sparkline.rs default `Color::White` → `theme::fg()`
- [x] Removed stale `THEME` imports from 10 files
- [x] Removed unused `Color` imports from 3 files

## Remaining Color:: in src/ui/ (all intentional)

| Line | Usage | Why OK |
|------|-------|--------|
| header.rs:117,122 | `Color::Reset` | "No background" when not modified |
| mod.rs:228 | `Color::Reset` | "No color" fallback |
| misc.rs:65 | `Color::Rgb(color.r,g,b)` | Dynamic user-chosen color swatch |
| misc.rs:185 | `Color::Blue` | Literal blue highlight swatch button |
| misc.rs:188 | `Color::Reset` | "No color" for X/Close button |
| settings.rs:657 | `Color::Rgb(rgb.r,g,b)` | Dynamic color swatch display |
| file_view.rs:124,133 | `Color::Rgb(r,g,b)` | Image pixel color rendering |
| file_view.rs:292,294 | `Color::Reset` | "No highlight" fallback |
| panes/editor.rs:139 | `Color::Reset` | Editor default style |

## Summary stats
- Before: ~261 hardcoded colors → After: 12 intentional `Color::` (6 Reset, 3 dynamic Rgb, 1 Blue swatch, 2 image Rgb)
- ThemeStyle fields: 6 → 22
- Theme presets: 6 → 11
- Theme accessors: 6 → 28
