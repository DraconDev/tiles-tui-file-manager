# Project State

## Current Focus
Added caching for sidebar tree items to avoid re-reading directories every frame

## Context
The Dolphin-style sidebar was recently implemented, but it was reading directory contents on every frame, which is inefficient. This change adds caching to improve performance.

## Completed
- [x] Added `sidebar_tree_cache` field to store tree items
- [x] Added `sidebar_tree_cache_key` for cache invalidation
- [x] Updated documentation for new fields

## In Progress
- [ ] Implement cache invalidation logic when `tree_expanded_folders` or `show_hidden` changes

## Blockers
- Need to implement cache invalidation logic before this can be fully utilized

## Next Steps
1. Implement cache invalidation when `tree_expanded_folders` or `show_hidden` changes
2. Add performance metrics to verify the caching improvements
