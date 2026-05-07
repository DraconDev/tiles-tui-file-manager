# Project State

## Current Focus
Added remote server name prefix to tab titles in the UI

## Context
This change improves the user experience by making it clearer which tabs are connected to remote servers. The previous implementation only showed the file path, which could be confusing when multiple remote connections were open.

## Completed
- [x] Added conditional display of remote server name in tab titles
- [x] Maintained existing behavior for local tabs
- [x] Preserved the existing icon display for remote tabs

## In Progress
- [x] This change is complete

## Blockers
- None identified

## Next Steps
1. Verify the change works with multiple remote connections
2. Consider adding visual distinction (color/icon) for remote tabs
