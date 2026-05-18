## Goal
Clean up all dead code identified in the full audit — less code, same functionality.

## Checklist
- [x] Delete `src/ui/layout.rs` (5 lines, unused placeholder)
- [x] Remove dead `EventLoopCtx::app_lock()`, `EventLoopCtx::drain_refreshes()` (truly unused)
- [x] Restore `EventLoopCtx::mark_refresh()`, `EventLoopCtx::send_event()` (used internally — audit correction)
- [x] Remove `App.tile_queue` field (never read after construction)
- [x] Remove `FileListState.path_colors` field (shadowed by SelectionState.path_colors)
- [x] Remove false-positive `#[allow(dead_code)]` from DragState, ClosedTab, MarqueeRect, BackgroundTask, DraconTheme, FILE_LIST_START_ROW
- [x] Keep `#[allow(dead_code)]` on AppEvent and UndoAction (protocol enums with unconstructed variants)
- [x] Remove `panes_needing_refresh` parameter from `handle_event` → `setup::handle_event` → `events::handle_event` (dead parameter chain)
- [x] Deduplicate `is_valid_search_char` — remove from file_manager.rs, use `crate::events::file_actions::is_valid_search_char`
- [x] Clean up dead test imports (Arc, Mutex, TilePlacement) in 4 test modules
- [x] Remove unused `use std::collections::HashSet` from events/mod.rs
- [x] Audit correction: SystemState.last_update IS used (modules/system.rs). Removed `#[allow(dead_code)]`.
- [x] Audit correction: DraconTheme IS used within theme.rs. Removed `#[allow(dead_code)]`.
- [x] Audit correction: All production `.expect()` calls are in test code. Zero runtime panic risk.

## Results
- 106 tests pass, clippy clean
- Total source: ~20,735 lines
- Removed: ~30 lines dead code + false-positive suppressions
