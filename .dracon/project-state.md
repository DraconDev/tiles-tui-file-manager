# Project State

## Current Focus
Removed remote bookmark persistence from the application state.

## Context
This change was prompted by the refactoring of remote bookmark management to use the server configuration model. The remote bookmarks were previously stored in the persistent state but are now managed separately.

## Completed
- [x] Removed `remote_bookmarks` field from `PersistentState` struct
- [x] Removed `remote_bookmarks` from state serialization
- [x] Removed unused import of `RemoteBookmark`

## In Progress
- [ ] None

## Blockers
- None

## Next Steps
1. Update any remaining code that might still reference remote bookmarks
2. Verify that the server configuration model properly handles remote bookmark persistence
