# Project State

## Current Focus
Added scroll offset tracking for sidebar navigation to maintain visible position of selected items

## Context
The sidebar was displaying all items at once, making navigation difficult for long lists. This change implements a scroll offset system to keep the selected item visible while navigating.

## Completed
- [x] Added scroll offset tracking for sidebar items
- [x] Implemented auto-scroll to keep selected item in view
- [x] Adjusted bounds calculation for visible items only
- [x] Limited scroll offset to prevent empty space at bottom

## In Progress
- [ ] None (this change is complete)

## Blockers
- None (dependency `dracon-files` failed to load, but this change is independent)

## Next Steps
1. Test scroll behavior with different list lengths
2. Verify performance impact with very large item lists
