# Project State

## Current Focus
Conditional folder expansion logic that distinguishes tree sidebar mode from regular mode and refreshes only on non‑tree expansion

## Completed
- [x] Modified `handle_enter_key` to use `expanded_set` and toggle between `expanded_folders` and `tree_expanded_folders` based on `sidebar_scope`
- [x] Added `is_tree_mode` check to avoid refreshing files when expanding a folder in tree mode
- [x] Updated `handle_sidebar_mouse` with identical tree‑mode aware expansion logic
- [x] Removed redundant file navigation and history push when expanding in tree mode
