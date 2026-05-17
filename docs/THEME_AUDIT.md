# Theme Hardcoded Color Audit — Remaining Work

Full audit of all `Color::` usages in `src/ui/` (excluding `theme.rs`).
Zero `Color::Red/DarkGray/Green/Yellow/Cyan/Magenta` remain — those are done.

## Category 1: `Color::Black` as contrast fg on themed bg (67 occurrences)

These are `.fg(Color::Black)` paired with a themed `.bg()` — used for text on
colored button backgrounds. Should be `theme::selection_fg()` for consistency
with how ESC/Cancel buttons were already fixed.

**Files:** modals.rs(3), header.rs(2), small_modals.rs(7), settings.rs(9),
footer.rs(3), debug.rs(2), misc.rs(4), context_menu.rs(1), monitor.rs(5),
git_view.rs(5), git_page.rs(0), mod.rs(0), file_view.rs(3),
panes/sidebar.rs(9)

### Special case: `Color::Black` as `.bg()` (background)
- `mod.rs:264,273` — `Block::default().style(Style::default().bg(Color::Black))` → these are the main app background, could use `THEME.bg` (already available)
- `git_view.rs:23` — `Block::default().style(Style::default().bg(Color::Black))` → same

## Category 2: `Color::White` as fg (32 occurrences)

These are text-on-dark-background colors. Should become `theme::fg()` or
stay as `THEME.fg`. Many are in monitor.rs for bold labels.

**Files:** monitor.rs(14), small_modals.rs(5), modals.rs(3), misc.rs(1),
footer.rs(2), git_view.rs(2), sparkline.rs(1), file_view.rs(1),
panes/sidebar.rs(1)

### `Color::White` as `.bg()` (background)
- `modals.rs:333` — `.bg(Color::White)` for rename input highlight → should use `theme::selection_bg()` or similar

## Category 3: `Color::Rgb` structural/layout colors (~30 occurrences)

These are NOT semantic — they're dark borders, backgrounds, and gradients.
Most should become theme fields for full themability.

### 3a. Subtle borders: `Color::Rgb(40, 45, 55)` (10 occurrences)
Very dark blue-gray used as `.border_style().fg()`. Too dark to be `muted()`.
→ Needs a new `theme::border_subtle()` field, or use `border_inactive()`.

**Files:** debug.rs(1), settings.rs(3), git_page.rs(2), monitor.rs(2)

### 3b. Monitor labels: `Color::Rgb(60, 65, 75)` (6 occurrences)
Dark gray for section labels in monitor. Already exists as `theme::monitor_label()`.
→ Replace with `theme::monitor_label()` calls.

**Files:** monitor.rs(6)

### 3c. Background blacks: `Color::Rgb(0, 0, 0)` (4 occurrences)
Explicit black bg on blocks. Should use `THEME.bg`.

**Files:** mod.rs(1), settings.rs(1), git_page.rs(1), plus debug.rs Rgb(8,8,12)

### 3d. Footer stat bar gradients (8 occurrences)
CPU bar colors and highlight tints. Need `theme::stat_*` accessors.
- `Rgb(88, 166, 255)` — CPU bar blue ×2
- `Rgb(255, 170, 0)` — CPU bar yellow
- `Rgb(140, 165, 210)` — CPU bar light blue
- `Rgb(80, 200, 255)` — CPU bar cyan
- `Rgb(85, 80, 20)` — task progress highlight bg
- `Rgb(30, 30, 35)` / `Rgb(20, 25, 30)` — footer dim text/bg

### 3e. File view / sidebar structural
- `Rgb(78, 58, 112)` — multi-selection row bg in file_view.rs → needs `theme::selection_alt_bg()`
- `Rgb(60, 60, 70)` — line number muted in file_view.rs → `theme::muted()`

### 3f. Breadcrumbs separator
- `Rgb(100, 100, 110)` — dim separator dot → `theme::muted()`

### 3g. Debug view
- `Rgb(8, 8, 12)` — debug panel bg → `THEME.bg`
- `Rgb(190, 190, 200)` — debug text → `THEME.fg`

### 3h. Settings color swatch display (OK — dynamic)
- `Color::Rgb(rgb.r, rgb.g, rgb.b)` / `Color::Rgb(color.r, color.g, color.b)` — these render the user's chosen color, NOT hardcoded → leave as-is

## Category 4: Named colors used as preset label swatches (3 occurrences)

These show the preset's representative color as a dot/label in settings:
- `settings.rs:621` — `Color::LightRed` for "Sunset" preset dot
- `settings.rs:622` — `Color::Gray` for "Mono" preset dot
- `misc.rs:186` — `Color::Blue` for "B" (Bookmark) button
- `misc.rs:189` — `Color::Reset` for "X" (Close) button

→ Sunset dot should use the preset's accent. Mono dot should use its muted.
→ Bookmark button: `theme::info()`. Close button: leave as Reset.

## Category 5: `Color::Reset` (4+ occurrences)

Used as "no color" / transparent — these are intentional and correct.
Leave as-is.

## Category 6: `Color::Gray` in sidebar (2 occurrences)

- `panes/sidebar.rs:345` — `Style::default().fg(Color::Gray)` for unmounted/empty mount
- `panes/sidebar.rs` second Gray for mount placeholder

→ `theme::muted()` — this is the same semantic meaning.

## Category 7: `sparkline.rs` — `Color::White` default

- `sparkline.rs:15` — `color: Color::White` as default struct field
→ Should use `THEME.fg` but struct init may not have access. Low priority.

## Category 8: `THEME.fg` / `THEME.bg` direct access (22 occurrences)

These read from the static `THEME` directly instead of going through accessor
functions. They work correctly but are inconsistent with the accessor pattern.
→ Low priority: consider `theme::fg()` / `theme::bg()` accessors for consistency.

---

# TODO (Priority Order)

## P1 — Semantic fixes (inconsistent with theme system)
1. [ ] Replace `Color::Black` fg on themed bg → `theme::selection_fg()` (67 occurrences, 14 files)
2. [ ] Replace `Color::White` fg → `theme::fg()` or `THEME.fg` (32 occurrences, 9 files)
3. [ ] Replace `Color::Gray` in sidebar → `theme::muted()` (2 occurrences)
4. [ ] Replace `Color::LightRed`/`Color::Gray` in settings preset labels → preset accent/muted (2 occurrences)
5. [ ] Replace `Color::Blue` in misc.rs bookmark button → `theme::info()`
6. [ ] Replace `Color::White` bg in modals.rs → `theme::selection_bg()`

## P2 — Structural color routing (adds theme fields)
7. [ ] Add `theme::border_subtle()` accessor (for Rgb(40,45,55)) — 10 occurrences
8. [ ] Replace `Rgb(60, 65, 75)` → `theme::monitor_label()` — 6 occurrences in monitor.rs
9. [ ] Replace `Color::Rgb(0,0,0)` / `Rgb(8,8,12)` bg → `THEME.bg` — 5 occurrences
10. [ ] Replace `Rgb(190,190,200)` debug text → `THEME.fg` — 1 occurrence
11. [ ] Replace `Rgb(60,60,70)` file_view line numbers → `theme::muted()` — 1 occurrence
12. [ ] Replace `Rgb(100,100,110)` breadcrumbs separator → `theme::muted()` — 1 occurrence
13. [ ] Add `theme::stat_cpu_*()` accessors for footer CPU bar gradient — 8 occurrences
14. [ ] Add `theme::selection_alt_bg()` for file_view multi-select `Rgb(78,58,112)` — 1 occurrence

## P3 — Consistency polish (low priority)
15. [ ] Add `theme::fg()` / `theme::bg()` accessors, replace `THEME.fg`/`THEME.bg` direct reads — 22 occurrences
16. [ ] Fix sparkline.rs default `Color::White` → `THEME.fg` — 1 occurrence
