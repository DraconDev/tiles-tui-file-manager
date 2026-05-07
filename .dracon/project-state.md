# Project State

## Current Focus
Refactored sidebar to use server configuration instead of remote bookmarks

## Context
The sidebar was previously using remote bookmarks for server connections. This change aligns with the new server configuration system introduced in recent commits.

## Completed
- [x] Updated sidebar to display server information from `app.servers` instead of `app.remote_bookmarks`
- [x] Modified the "No remotes" message to check `app.servers.is_empty()` instead of `app.remote_bookmarks.is_empty()`

## In Progress
- [ ] None (this is a complete refactoring)

## Blockers
- None (this change is complete)

## Next Steps
1. Verify sidebar behavior with the new server configuration
2. Ensure all related UI components are updated to use the server configuration system
