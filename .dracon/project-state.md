# Project State

## Current Focus
Removed Konsole tab spawning fallback logic in favor of unified terminal spawning

## Context
This change simplifies the terminal spawning logic by removing the conditional fallback to Konsole-specific DBus calls, which were previously used when the `new_tab` flag was set. The new implementation now directly uses the unified `spawn_terminal_at` function from the terminal engine.

## Completed
- [x] Removed Konsole-specific DBus interaction code
- [x] Simplified terminal spawning to use unified `spawn_terminal_at` function
- [x] Fixed argument passing in `SpawnDetached` event handler

## In Progress
- [ ] None

## Blockers
- None

## Next Steps
1. Verify terminal spawning behavior across different environments
2. Test edge cases for terminal spawning with and without commands
