# Project State

## Current Focus
Improved terminal tab/window management for WezTerm emulator

## Context
The change enhances terminal handling by adding support for both new tabs and new windows in WezTerm, which was previously only supporting new windows.

## Completed
- [x] Added conditional logic to use `cli spawn` for new tabs and `start` for new windows in WezTerm
- [x] Maintained existing behavior for other terminal emulators

## In Progress
- [ ] None

## Blockers
- None

## Next Steps
1. Test the new tab/window behavior in WezTerm
2. Verify compatibility with other terminal emulators
