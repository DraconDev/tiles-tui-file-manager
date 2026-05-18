## Goal
Add tests for untested critical modules (P1 from TODO.md).

## Checklist
- [x] Add tests for `event_helpers.rs` — +7 tests (update_commands filtering, get_context_menu_actions for file/folder/empty-space/zip)
- [x] Add tests for `modules/files.rs` — +6 tests (check_file_suitability, read_dir, read_dir_recursive, get_run_command, copy_recursive)
- [x] Add tests for `events/file_manager.rs` — +4 tests (reselect_after_filter: match, miss, none, offset adjustment)
- [x] Add tests for `events/file_mouse.rs` — +6 tests (fs_mouse_index, marquee_rect active/inactive/normalized, clear_marquee)

## Results
- 106 → **129 tests** (+23, +22%)
- All pass, clippy clean
