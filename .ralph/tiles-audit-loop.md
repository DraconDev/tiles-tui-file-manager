# Tiles Project Audit & Cleanup Loop

Goal: Work through the TODO.md checklist systematically to clean up and audit the project.

## Iteration 1: Code Audit - TODO/FIXME/HACK Search & Syntax Fix

### Tasks
- [ ] Search all `.rs` files for TODO, FIXME, HACK, XXX, BUG, NOTE, OPTIMIZE, PERF, RFE markers
- [ ] Fix `src/app.rs` syntax error (unclosed delimiter at line 604)
- [ ] Run `cargo build --release` and confirm clean build

### Notes
- `src/app.rs` has a syntax error preventing compilation
- Need to verify all TODO/FIXME/HACK comments are cleaned up
- Target: 0 actionable comments, clean build
