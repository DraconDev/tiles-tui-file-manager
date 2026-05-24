# Scroll Past Selected Line Fix - Version 2

## Issue
We still scrolled past the active selected line in the file list. When using arrow keys or scroll controls, the selection would move to another row, causing the viewport to scroll and potentially hide the selected item.

## Root Cause
The previous fix checked viewport visibility based on the *current* selection position, but didn't account for the fact that the selection position changes when using arrow keys. The viewport would scroll before the new selection position was calculated.

## Solution
Modified the `move_up()` and `move_down()` functions in `src/app.rs` to:
1. First move the selection to the new position
2. Then check if the NEW selection position would be visible in the viewport
3. Only scroll if the new position would go out of viewport

### Changes Made

#### `move_down()` function (around line 379)
- Moved selection to `next` position first
- Then checked if new selection position would be in viewport
- Only scroll down if new selection would go out of viewport
- If new selection is already visible, keep viewport unchanged

#### `move_up()` function (around line 329)
- Same logic: move selection first, then check viewport
- Only scroll up if new selection would go out of viewport

### Code Logic

```rust
// For both move_up and move_down:
fs.list.selection.handle_move(next, shift);
fs.view.table_state.select(fs.list.selection.selected);

// Check if NEXT selection is within viewport before scrolling
let capacity = fs.view.view_height.saturating_sub(3);
let current_offset = fs.view.table_state.offset();

// Calculate where the new selection would be in the viewport
let new_sel = fs.list.selection.selected.unwrap_or(0);
let new_screen_row = 3 + new_sel.saturating_sub(current_offset);

if new_screen_row < capacity {
    // New selection is visible in viewport, keep viewport unchanged
} else {
    // New selection would go out of viewport, scroll to keep it visible
    if direction == DOWN {
        *offset_mut() = new_sel.saturating_sub(capacity);
    } else {
        *offset_mut() = offset.saturating_sub(1);
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

## Documentation
- `docs/SCROLL_FIX.md` - Previous version (now obsolete)
- `docs/SCROLL_FIX_V2.md` - This version (current implementation)