# Project State

## Current Focus
Remove explicit tree mode toggle and simplify directory expansion logic to always use an inline tree view, updating event handling accordingly.

## Completed
- [x] Remove Ctrl+W shortcut for toggling tree mode; tree mode is now implicit.
- [x] Refactor space key handling to toggle folder expansion when a directory is selected.
- [x] Simplify enter key handling by eliminating tree‑mode specific behavior; now it only opens files or folders normally.
- [x] Update file listing routine to always walk expanded folders, removing the tree‑mode flag and associated depth logic.
- [x] Adjust main event loop to remove tree‑mode parameter and clean up related variables.
