## Goal
Add tests for untested critical modules (P1 from TODO.md).

## Checklist
- [ ] Add tests for `event_helpers.rs` (850 lines, core navigation/command execution)
- [ ] Add tests for `events/file_manager.rs` (1082 lines, keyboard handler)
- [ ] Add tests for `events/file_mouse.rs` (647 lines, mouse handler)
- [ ] Add tests for `modules/files.rs` (file operations, read_dir_with_metadata)

## Constraints
- Run `cargo build && cargo test && cargo clippy -- -D warnings` after each batch
- `#[cfg(test)]` required on test modules
- Use `--test-threads=1` for test runs (flaky thread race in parallel)
- Keep tests focused on logic, not TUI rendering
