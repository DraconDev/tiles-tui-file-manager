# Tiles Project - Audit & Exploration Todo

## 🔍 Code Audit

### TODO/FIXME/HACK Comments
- [ ] Search all `.rs` files for TODO, FIXME, HACK, XXX, BUG, NOTE, OPTIMIZE, PERF, RFE markers
- [ ] Review and resolve any found comments
- [ ] Confirm clean state (0 actionable comments expected)

### Syntax & Compilation
- [ ] Fix `src/app.rs` syntax error (unclosed delimiter at line 604)
- [ ] Run `cargo build --release` and confirm clean build
- [ ] Run `cargo clippy` and resolve all warnings
- [ ] Run `cargo test` and confirm all tests pass (129 expected)

### File & Directory Cleanup
- [ ] Remove outdated TODO files: `docs/TODO.md`, `docs/THEME_TODO.md`
- [ ] Remove `.ralph/` directory contents (state files from old sessions)
- [ ] Remove `note.md` if still exists
- [ ] Clean up `docs/LEFTOVER_TODO.md`, `docs/SCROLL_FIX.md`, `docs/SCROLL_FIX_V2.md`
- [ ] Remove `1231.txt` if not needed
- [ ] Review `.ralph/` directory - keep only active state files

## 🏗️ Architecture Review

### Project Structure
- [ ] Verify `src/` directory structure is logical and organized
- [ ] Check `state/` subdirectory for proper separation of concerns
- [ ] Review `events/` subdirectory for event handling patterns
- [ ] Check `config.rs` for proper configuration management

### Dependencies
- [ ] Review `Cargo.toml` for outdated dependencies
- [ ] Check `dracon-terminal-engine` dependency version compatibility
- [ ] Verify no unused dependencies in `Cargo.toml`

### Code Quality
- [ ] Check for `unwrap()` usage in production code (should be minimal)
- [ ] Review error handling patterns across the codebase
- [ ] Check for dead code (unused functions, variables, imports)
- [ ] Review public API surface for consistency

## 🐛 Bug Fixes

### Known Issues
- [ ] Fix viewport scrolling in `move_up()` / `move_down()` (selection visibility)
- [ ] Fix marquee drag UX in `src/events/file_mouse.rs`
- [ ] Fix editor cursor placement in `dracon-terminal-engine`
- [ ] Verify all fixes hold under release build conditions

### Testing
- [ ] Run full test suite: `cargo test`
- [ ] Run smoke tests: `cargo test --test smoke`
- [ ] Verify 129 unit tests pass
- [ ] Verify 4 smoke tests pass

## 📝 Documentation

### Project Docs
- [ ] Update `CHANGELOG.md` with recent fixes
- [ ] Review `README.md` for accuracy
- [ ] Review `CONTRIBUTING.md` for completeness
- [ ] Update `plan/blueprint.md` if architecture changed

### Code Documentation
- [ ] Check for missing doc comments on public functions
- [ ] Review inline comments for accuracy
- [ ] Update any outdated comments

## 🎨 UI/UX Review

### File Manager
- [ ] Test marquee selection behavior
- [ ] Test file drag and drop
- [ ] Test shift-click selection
- [ ] Test viewport scrolling with many files
- [ ] Test split pane navigation

### Editor
- [ ] Test cursor placement after newline
- [ ] Test syntax highlighting
- [ ] Test undo/redo functionality

### Sidebar
- [ ] Test folder navigation
- [ ] Test favorites management
- [ ] Test recent files list

## 🚀 Performance

### Code Review
- [ ] Check for unnecessary allocations
- [ ] Review hot path performance (file list rendering, scrolling)
- [ ] Check for excessive cloning or borrowing
- [ ] Review event handling for efficiency

### Resource Management
- [ ] Check for memory leaks or unbounded collections
- [ ] Review file handle management
- [ ] Check for proper cleanup on exit

## 📊 Final Checklist

- [ ] All tests pass (129 unit + 4 smoke)
- [ ] Clean clippy (0 warnings)
- [ ] Clean release build
- [ ] No TODO/FIXME/HACK comments
- [ ] No outdated documentation files
- [ ] No dead code
- [ ] No unused dependencies
- [ ] All known bugs fixed
- [ ] Documentation up to date
- [ ] Project builds and runs correctly
