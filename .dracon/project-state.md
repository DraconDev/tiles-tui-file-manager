# Project State

## Current Focus
Rework directory navigation to avoid re‑sorting tree listings, clear expanded state on directory change, and simplify span of metadata handling.

## Completed
- [x] Clear `app.expanded_folders` when moving into a new directory to reset expanded view.
- [x] Remove explicit re‑sorting of file pairs in tree mode; rely on `walk_tree` to provide a folders‑first, alphabetical order.
- [x] Simplify metadata acquisition: discard temporary copies, directly use metadata from recursive read.
- [x] Refactor file listing task to avoid unnecessary tuple packing and keep API consistent.
- [x] Delete unused sorting code that previously scattered child entries under parents.
- [x] Adjust function signatures in the file listing module to match new, simpler return types.
