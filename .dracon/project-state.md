# Project State

## Current Focus
Make tree depth tracking mutable in spawn_blocking to allow in-place rebuild of aligned files and depths during tree-mode filtering.

## Completed
- [x] Promote `tree_depths` to mutable in the spawn_blocking return tuple so it can be updated in-place.
- [x] Remove redundant lowercase conversion of the search filter during the “rebuild aligned files + depths” pass, relying instead on precomputed depth structures.
