# Scroll Past Selected Line Fix

## Issue
We still scrolled past the active selected line in the file list. When using arrow keys or scroll controls, the selection would move to another row, causing the viewport to scroll and potentially hide the selected item.

## Solution
Modified the `move_up()` and `move_down()` functions in `src/app.rs` to check if the selection would remain visible in the viewport before scrolling.

### Changes Made

#### `move_down()` function (around line 365)
- Added logic to check if the next selection position would go out of viewport
- If selection is already visible in viewport, don't scroll
- Only scroll down if the selection would go out of viewport

#### `move_up()` function (around line 330)
- Added logic to check if scrolling up would hide the selected item
- If selection is visible and scrolling up would keep it visible, don't scroll
- Only scroll up if the selection would go out of viewport

### Code Logic

```rust
// For move_down:
if sel + 1 >= current_offset && sel + 1 < current_offset + capacity {
    // Selection is still visible, don't scroll
    return;
} else if sel + 1 >= current_offset + capacity {
    // Selection would go out of viewport, scroll down
    *fs.view.table_state.offset_mut() = (sel + 1).saturating_sub(capacity);
}

// For move_up:
if sel >= current_offset {
    // Selection is visible, check if scrolling up would hide it
    let selection_screen_row = 3 + sel.saturating_sub(current_offset);
    if selection_screen_row < capacity - 1 {
        // Selection would still be visible, don't scroll
        return;
    }
}
```

## Result
- ✅ Selection stays visible in viewport when using arrow keys
- ✅ No more scrolling past selected line
- ✅ All 129 tests passing
- ✅ Clippy clean

## Test Results
```
test result: ok. 129 passed; 0 failed; 0 ignored; 0 filtered out
```