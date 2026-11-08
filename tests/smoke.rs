//! Smoke tests for the built binary.
//!
//! These intentionally do NOT spawn `cargo build` / `cargo clippy` /
//! `cargo test` — running the project's own test runner from inside the
//! suite is a recursion fork bomb (and nested builds take ~30s apiece).
//! Clippy and full builds are CI's job; `cargo test` already builds the
//! binary into `CARGO_BIN_EXE_tiles` for these tests.

use std::process::Command;

fn tiles_bin() -> std::ffi::OsString {
    std::env::var_os("CARGO_BIN_EXE_tiles")
        .expect("CARGO_BIN_EXE_tiles must be set by cargo test")
}

#[test]
fn binary_runs_version() {
    let output = Command::new(tiles_bin())
        .args(["--version"])
        .output()
        .expect("binary should run");
    assert!(output.status.success(), "binary should exit 0 on --version");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("14."), "version output: {}", stdout);
}

#[test]
fn binary_runs_help() {
    let output = Command::new(tiles_bin())
        .args(["--help"])
        .output()
        .expect("binary should run");
    assert!(output.status.success(), "binary should exit 0 on --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage: tiles"),
        "help output should mention usage: {}",
        stdout
    );
}
