# Project State

## Current Focus
Improved directory tree marker hit detection in file manager

## Context
The previous implementation had inconsistent hit detection for directory tree expand/collapse markers due to hardcoded column assumptions. This change makes the detection more robust by using explicit column lookup and adjusting the hit area buffer.

## Completed
- [x] Refactored marker hit detection to use explicit `FileColumn::Name` lookup
- [x] Increased hit area buffer from 1 to 2 columns for better usability
- [x] Added depth-based marker positioning that accounts for indentation

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Verify the new hit detection works across different terminal sizes
2. Consider adding visual feedback for hit detection during development
