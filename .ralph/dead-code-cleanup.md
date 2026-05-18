## Goal
Clean up all dead code identified in the full audit — less code, same functionality.

## Checklist
- [ ] Delete `src/ui/layout.rs` (5 lines, unused placeholder)
- [ ] Remove `EventLoopCtx` dead methods: `app_lock`, `send_event`, `mark_refresh`, `drain_refreshes`
- [ ] Remove `App.tile_queue` field (never read after construction)
- [ ] Remove `FileListState.path_colors` field (shadowed by SelectionState.path_colors)
- [ ] Remove `SystemState.last_update` field (never read)
- [ ] Remove false-positive `#[allow(dead_code)]` from DragState fields and other used types
- [ ] Check if `DraconTheme` struct is dead (replaced by ThemeStyle?)
- [ ] Fix production `.expect()` in `remote.rs:386`
- [ ] Verify: cargo build + test + clippy after each batch
