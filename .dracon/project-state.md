# Project State

## Current Focus
Refactored terminal spawning behavior to always open in a new tab

## Context
The change was prompted by a need to standardize terminal spawning behavior in the context menu. Previously, terminals were only opened in new tabs when explicitly requested via `ContextMenuAction::RunTerminal`, but this was changed to always open in new tabs for consistency.

## Completed
- [x] Modified terminal spawning logic to always create new tabs (`new_tab: true`)

## In Progress
- [x] Behavior change is complete

## Blockers
- None identified

## Next Steps
1. Verify terminal behavior across different context menu actions
2. Consider adding configuration options for terminal spawning behavior
