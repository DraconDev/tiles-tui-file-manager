# Project State

## Current Focus
Added signal for new remote server creation in settings modal

## Context
This change supports the remote server management system by distinguishing between adding a new server and editing an existing one.

## Completed
- [x] Added `open_with_index = usize::MAX` to signal new server creation

## In Progress
- [x] Remote server management system implementation

## Blockers
- Missing manifest for dependency `dracon-files`

## Next Steps
1. Resolve dependency issue for `dracon-files`
2. Complete server configuration module integration
