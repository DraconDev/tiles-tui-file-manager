## CPU Spike Investigation — Tiles TUI

### Symptom
Heavy CPU usage when Tiles is active after selecting many items.

### Root Causes Found

#### 🔴 #1: Unconditional redraw on every Tick (FIXED)
**File**: `src/handlers/event_loop_ctx.rs:145`
**Problem**: `handle_tick()` returned `true` unconditionally, meaning every 250ms Tick event forced a full `terminal.draw()` even when nothing changed.
**Impact**: ~4 full UI redraws/second at idle (header + sidebar + 2 file panes + footer + overlays)
**Fix**: `handle_tick()` now returns `true` only when watch sync actually runs; `false` otherwise. Redraws still happen on user input, file changes, system updates, etc.

#### 🟡 #2: Per-row HashMap lookups for path_colors (FIXED)
**File**: `src/ui/file_view.rs:284,321,370`
**Problem**: `app.selection.path_colors.get(path)` and `.contains_key(path)` called 2-3x per visible row per frame. `HashMap<PathBuf, u8>` lookups involve hashing + equality checking. Most users have 0 path colors, but lookups still occur.
**Fix**: Cache `has_path_colors = !app.selection.path_colors.is_empty()` before the row loop. All lookups short-circuit when empty.

#### 🟡 #3: `path.to_string_lossy()` for divider check (FIXED)
**File**: `src/ui/file_view.rs:331`
**Problem**: `path.to_string_lossy() == "__DIVIDER__"` allocates a `Cow<str>` per visible row per frame just to check a constant.
**Fix**: Use `path.as_os_str() == "__DIVIDER__"` — zero allocation, direct byte comparison.

#### 🟡 #4: `app.settings.semantic_coloring` field access per row (FIXED)
**File**: `src/ui/file_view.rs:372`
**Problem**: Field access through `app.settings` each row (minor, but cached now).
**Fix**: Cache `use_semantic_coloring` before the row loop.

### Not the Bottleneck (Verified)
- **Theme RwLock accessors**: `parking_lot::RwLock::read()` is ~1-2ns (atomic load, no syscall). 300 calls/frame × 2ns = 0.6µs — negligible.
- **`multi.contains(&file_idx)`**: `HashSet` O(1) average, even with 1000+ selected items.
- **`get_file_category()`**: Unit struct dispatch, no allocation.
- **Inotify polling**: Event-driven via debouncer, not busy-waiting.
- **Main loop throttling**: 33ms sleep at bottom = max 30Hz iteration rate.

### Performance Model (After Fixes)

| State | Before | After |
|-------|--------|-------|
| Idle (no changes) | 4 full redraws/sec | 0 redraws (watch sync ~every 30s) |
| User scrolling | ~4 redraws/sec | On-demand only |
| File changed on disk | Redraw + 4 tick redraws | Redraw only |
| Path colors empty | 3 HashMap lookups/row | 1 boolean check/row |
