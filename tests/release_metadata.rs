//! Release-bookkeeping guards.
//!
//! The changelog drifted once already — versions 14.113.0 and 14.114.0 were
//! tagged and published while `CHANGELOG.md` jumped straight from 14.112.0 to
//! 14.115.0, and the version string disagreed between Cargo.toml, Cargo.lock,
//! and flake.nix. These tests pin the invariant that would have caught it:
//! the crate version, its lock entry, the Nix package version, and the newest
//! changelog section all name the same release.
//!
//! They read repository files only — no builds, no network, no git tags
//! (shallow CI checkouts do not have tags, so tag-level coverage belongs to
//! review, not here).

use std::path::Path;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(name: &str) -> String {
    let path = manifest_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Version from the `[package]` table: the first `version = "..."` line.
fn cargo_toml_version() -> String {
    let text = read_repo_file("Cargo.toml");
    text.lines()
        .find_map(|line| {
            let rest = line.strip_prefix("version = ")?;
            Some(rest.trim().trim_matches('"').to_string())
        })
        .expect("Cargo.toml must declare a package version")
}

/// Version from `[[package]] name = "tiles-tui-file-manager"` in the lockfile.
fn cargo_lock_version() -> String {
    let text = read_repo_file("Cargo.lock");
    let mut in_pkg = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "name = \"tiles-tui-file-manager\"" {
            in_pkg = true;
            continue;
        }
        if in_pkg {
            if let Some(rest) = trimmed.strip_prefix("version = ") {
                return rest.trim().trim_matches('"').to_string();
            }
            // Left the package block without a version line.
            in_pkg = false;
        }
    }
    panic!("Cargo.lock must contain this crate's package entry");
}

/// Version of the `tiles` derivation in flake.nix (first `version = "..."`).
fn flake_version() -> String {
    let text = read_repo_file("flake.nix");
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("version = ")?;
            Some(rest.trim().trim_end_matches(';').trim().trim_matches('"').to_string())
        })
        .next()
        .expect("flake.nix must declare the package version")
}

/// Every `## [x.y.z]` heading in the changelog, newest first.
fn changelog_versions() -> Vec<String> {
    let text = read_repo_file("CHANGELOG.md");
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("## [")?;
            let end = rest.find(']')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

fn parse_semver(v: &str) -> (u64, u64, u64) {
    let mut parts = v.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

#[test]
fn version_sources_agree_with_the_crate() {
    let crate_version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        cargo_toml_version(),
        crate_version,
        "Cargo.toml version must be the crate version"
    );
    assert_eq!(
        cargo_lock_version(),
        crate_version,
        "Cargo.lock package version drifted from Cargo.toml"
    );
    assert_eq!(
        flake_version(),
        crate_version,
        "flake.nix package version drifted from Cargo.toml"
    );
}

/// A released version must be documented: the changelog's newest section is
/// the version the crate currently claims. Bumping the version without adding
/// a section (the 14.113.0/14.114.0 drift) fails here.
#[test]
fn changelog_documents_the_current_version() {
    let versions = changelog_versions();
    let newest = versions.first().expect("CHANGELOG.md must have at least one version section");
    assert_eq!(
        newest,
        env!("CARGO_PKG_VERSION"),
        "unreleased version missing from CHANGELOG.md; newest section is {newest}"
    );
}

/// Sections are unique and ordered newest-first, so a duplicated or
/// re-inserted release is visible instead of silently splitting the history.
#[test]
fn changelog_sections_are_unique_and_ordered() {
    let versions = changelog_versions();
    assert!(versions.len() > 1, "changelog should list several releases");
    let parsed: Vec<(u64, u64, u64)> = versions.iter().map(|v| parse_semver(v)).collect();
    for window in parsed.windows(2) {
        assert!(
            window[0] > window[1],
            "changelog versions must descend (newest first): {:?} then {:?}",
            window[0],
            window[1]
        );
    }
}
