# Project State

## Current Focus
Fix serialization of `git_stashes` field by adding missing `#[serde(skip)]` attribute

## Completed
- [x] fix(state): add `#[serde(skip)]` to `git_stashes` field in `FileState` struct to prevent serialization of this runtime-only data
