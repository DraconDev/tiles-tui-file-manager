# Project State

## Current Focus
Align tree UI markers with directory state and clarify space-key action label.

## Completed
- [x] Use `is_dir && !marker.is_empty()` instead of `is_expanded` for directory marker logic to ensure only valid directories show expansion state.
- [x] Rename space-key hint from "Editor" to "Expand/Edit" to accurately reflect its dual behavior in tree mode.
