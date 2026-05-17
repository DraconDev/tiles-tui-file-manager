# Theme Audit Fix — All 16 TODO items

Work through every item in `docs/THEME_AUDIT.md`.

## P1 — Semantic fixes
1. Replace `Color::Black` fg on themed bg → `theme::selection_fg()` (67 occurrences, 14 files)
2. Replace `Color::White` fg → `theme::fg()` or `THEME.fg` (32 occurrences, 9 files)
3. Replace `Color::Gray` in sidebar → `theme::muted()` (2 occurrences)
4. Replace `Color::LightRed`/`Color::Gray` in settings preset labels → preset accent/muted (2 occurrences)
5. Replace `Color::Blue` in misc.rs bookmark button → `theme::info()`
6. Replace `Color::White` bg in modals.rs → `theme::selection_bg()`

## P2 — Structural color routing
7. Add `theme::border_subtle()` accessor + ThemeStyle field (for Rgb(40,45,55)) — 10 occurrences
8. Replace `Rgb(60, 65, 75)` → `theme::monitor_label()` — 6 occurrences in monitor.rs
9. Replace `Color::Rgb(0,0,0)` / `Rgb(8,8,12)` bg → `THEME.bg` — 5 occurrences
10. Replace `Rgb(190,190,200)` debug text → `THEME.fg` — 1 occurrence
11. Replace `Rgb(60,60,70)` file_view line numbers → `theme::muted()` — 1 occurrence
12. Replace `Rgb(100,100,110)` breadcrumbs separator → `theme::muted()` — 1 occurrence
13. Add footer stat bar accessors — 8 occurrences
14. Add `theme::selection_alt_bg()` for file_view multi-select `Rgb(78,58,112)` — 1 occurrence

## P3 — Consistency polish
15. Add `theme::fg()` / `theme::bg()` accessors, replace `THEME.fg`/`THEME.bg` direct reads — 22 occurrences
16. Fix sparkline.rs default `Color::White` → `THEME.fg` — 1 occurrence

## Rules
- Run `cargo clippy -- -D warnings && cargo test --bin tiles` after every change
- Keep commits small and descriptive
- Do NOT change behavior — only color sources
- Do NOT change theme.rs test module