# Project State

## Current Focus
Refactored configuration constants to centralized locations for better maintainability.

## Context
This change moves hardcoded constants (like `MAX_RECENT_FOLDERS`, `PREVIEW_MAX_MB`, and `MAX_TABS`) to a centralized configuration module, making them easier to manage and modify across the application.

## Completed
- [x] Moved `MAX_RECENT_FOLDERS` and `PREVIEW_MAX_MB` from `app.rs` to centralized config
- [x] Moved `MAX_TABS` from `state/mod.rs` to centralized config
- [x] Updated all references to use the centralized constants

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Verify all references to these constants are properly updated
2. Consider adding validation for these configuration values
