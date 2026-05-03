# Project State

## Current Focus
Improved folder navigation state persistence by tracking both selection and scroll position.

## Context
This change addresses a need to maintain user interface state when navigating between folders, ensuring both the selected item and scroll position are preserved when returning to a previously visited folder.

## Completed
- [x] Added `folder_selections` field to store (selected_index, scroll_offset) pairs per folder path
- [x] Documented the purpose of the new field with a doc comment

## In Progress
- [ ] Implementation of actual state restoration logic (not yet in this commit)

## Blockers
- Need to implement the actual state restoration logic in folder navigation code

## Next Steps
1. Implement state restoration in folder navigation code
2. Add unit tests for the new state persistence behavior
