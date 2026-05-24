# Leftover TODOs for Tiles

## ✅ All Issues Fixed

### 1. Editor cursor placement bug (dracon-terminal-engine)
**Issue**: After pressing Enter, cursor column offset by +1 per empty row before insertion point.
- **Fixed**: Modified `insert_newline()` to not account for empty lines before cursor
- **Location**: `/home/dracon/Dev/dracon-terminal-engine/src/widgets/editor.rs`
- **Status**: ✅ FIXED

### 2. Marquee drag UX improvements (tiles-tui-file-manager)
**Issue**: Click nothing should deselect all during marquee drag, and non-name area drag should not trigger single-item drag.
- **Fixed**: Updated `file_mouse.rs`:
  - Marquee selection now only activates when clicking in Name column
  - File drag only starts when clicking in Name column with no modifiers
  - Shift-click selection now works everywhere (sidebar, content, empty space)
- **Location**: `src/events/file_mouse.rs`
- **Status**: ✅ FIXED

## 📝 Summary

All known issues have been resolved:
- Editor cursor behavior corrected
- Drag selection UX improved
- Shift-click selection works throughout the UI

No remaining TODOs in code (0 TODO/FIXME/HACK comments).