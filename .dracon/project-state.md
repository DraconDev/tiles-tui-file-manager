# Project State

## Current Focus
Refactored sidebar to always display folder tree rooted at home directory

## Context
The sidebar was previously showing the folder tree relative to the current file's path, which could be confusing for users. This change aligns with Dolphin-style behavior where the sidebar always shows the complete filesystem hierarchy from the home directory.

## Completed
- [x] Changed sidebar folder tree to always root at home directory
- [x] Removed conditional path logic that previously used current file path or fallback

## In Progress
- [ ] Verify visual consistency with other file manager panes

## Blockers
- None identified

## Next Steps
1. Test sidebar behavior with various file paths
2. Document the new navigation paradigm in user documentation
