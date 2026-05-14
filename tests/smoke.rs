use std::process::Command;

#[test]
fn binary_compiles_and_runs() {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .output()
        .expect("cargo build should succeed");
    assert!(output.status.success(), "build should succeed");
}

#[test]
fn clippy_passes() {
    let output = Command::new("cargo")
        .args(["clippy", "--", "-D", "warnings"])
        .output()
        .expect("cargo clippy should succeed");
    assert!(output.status.success(), "clippy should pass with -D warnings");
}

#[test]
fn path_resolution_dotdot() {
    let output = Command::new("cargo")
        .args(["test", "resolve_double_dot_multiple", "--quiet", "--", "--exact"])
        .output()
        .expect("test should run");
    assert!(output.status.success(), "dotdot resolution test should pass");
}

#[test]
fn path_resolution_relative() {
    let output = Command::new("cargo")
        .args(["test", "resolve_relative_path", "--quiet", "--", "--exact"])
        .output()
        .expect("test should run");
    assert!(output.status.success(), "relative path test should pass");
}
