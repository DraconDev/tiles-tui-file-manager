# Project State

## Current Focus
Improved terminal and tab management in the file manager UI

## Context
The changes refactor the keyboard shortcuts for terminal and tab management to provide clearer user experience and fix an issue with the terminal engine.

## Completed
- [x] Reorganized keyboard shortcuts for terminal operations (Ctrl+N, Ctrl+K, Ctrl+T)
- [x] Fixed terminal tab behavior (previously opened windows instead of tabs)
- [x] Updated hotkey documentation to reflect new behavior

## In Progress
- [ ] No active work in progress

## Blockers
- The `dracon_terminal_engine` dependency is failing to load its manifest (blocking further terminal integration work)

## Next Steps
1. Investigate and resolve the dependency issue with `dracon_terminal_engine`
2. Verify terminal tab behavior works correctly with the new shortcut assignments
