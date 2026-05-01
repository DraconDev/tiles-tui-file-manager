# Project State

## CurrentFocus
Add a `debug_tree` helper that logs diagnostic messages to `/tmp/tiles_tree.log` only when the `debug_tree` feature is enabled, and use it to trace file manager marker clicks and UI rendering events.

## Completed
- [x] Introduced `debug_tree` in `src/events/file_manager.rs` with conditional logging to `/tmp/tiles_tree.log` and added necessary `use` imports for `App` types.
- [x] Replaced `eprintln!` with `debug_tree` calls in `handle_file_mouse` to log click coordinates, marker rectangle checks, and successful match events.
- [x] Added `debug_tree` in `src/ui/mod.rs` with identical conditional logging behavior.
- [x] Inserted a `debug_tree` call in `draw_file_view` to log rendering details such as column, depth, row, marker rectangle, and file name.
- [x] Regenerated `Cargo.lock` and updated `Cargo.toml` to reflect resolved dependency versions after refactoring.
