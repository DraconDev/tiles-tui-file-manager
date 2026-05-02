# Project State

## Current Focus
Refactored folder expansion state tracking in the sidebar to use a more consistent naming convention.

## Context
The previous code used `tree_expanded_folders` to track expanded folders, but this was inconsistent with the rest of the application which uses `expanded_folders`. This change aligns the naming convention for better maintainability.

## Completed
- [x] Renamed `tree_expanded_folders` to `expanded_folders` in the sidebar drawing function

## In Progress
- [ ] None

## Blockers
- None

## Next Steps
1. Verify the sidebar rendering still works correctly with the new naming
2. Check for any other instances of inconsistent naming that might need similar updates
