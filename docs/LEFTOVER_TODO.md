# Leftover TODOs for Tiles

## 🐛 Bug Reports (Needs Fix in dracon-terminal-engine)

### Editor cursor placement
**Issue**: After pressing Enter, cursor column offset by +1 per empty row before insertion point.
- **Location**: dracon-terminal-engine crate
- **Status**: Needs reproduction and fix in `/home/dracon/Dev/dracon-terminal-engine`

## 📋 Known Issues

1. **Click nothing should deselect all during marquee drag** - Now fixed (shift-click works everywhere)
2. **Non-name area drag triggers single-item drag** - Now fixed (marquee only activates from Name column)
3. **Editor cursor +1 offset per empty row** - See above

## ✅ Recent Improvements

- **Shift-click selection** - Now works everywhere (sidebar, content, empty space)
- **Drag behavior** - Marquee selection only from Name column, file drag only when appropriate