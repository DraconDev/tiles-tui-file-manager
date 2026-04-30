# Project State

## Current Focus
Implement sidebar up/down navigation and dedicated `tree_expanded_folders` state for sidebar tree view

## Completed
- [x] Add `tree_expanded_folders` HashSet to App to track sidebar tree expanded folders separately from main `expanded_folders`
- [x] Add `sidebar_move_up` and `sidebar_move_down` methods to App with bounds checking for sidebar selection index
- [x] Wire Up/Down arrow keys to trigger sidebar navigation when sidebar is focused, bypassing main file manager movement
- [x] Update sidebar tree rendering and item collection to reference `tree_expanded_folders` for folder expansion status
