# Project State

## Current Focus
Refactored sidebar hidden file visibility to match focused pane's settings

## Context
The sidebar previously used the global `default_show_hidden` setting, which didn't match the file pane's visibility state. This change ensures consistent behavior between panes.

## Completed
- [x] Modified hidden file filtering to use focused pane's `show_hidden` state
- [x] Added fallback to `default_show_hidden` when pane state isn't available

## In Progress
- [ ] None (change is complete)

## Blockers
- None (dependency `dracon-files` failed to load, but this change is independent)

## Next Steps
1. Verify sidebar visibility matches file pane in all cases
2. Consider adding a sidebar-specific visibility toggle if needed
