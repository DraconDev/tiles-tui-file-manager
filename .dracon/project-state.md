# Project State

## Current Focus
Implement recursive directory listing for tree mode, respecting expanded folders and limiting depth.

## Completed
- [x] Add tree mode toggle handling when refreshing panes, passing `tree_mode` and `tree_expanded` info.
- [x] Implement recursive walk inside `run_tty` to collect files for tree mode, up to a maximum depth of 10, and skip hidden files.
- [x] Replace flat file list with tree‑structured list when tree mode is active, preserving metadata for existing paths.
