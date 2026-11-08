//! Criterion benchmarks for Tiles TUI hot paths.
//!
//! Every benchmark here calls the production function it measures (via the
//! library crate root), so a regression in the real code shows up here.
//! Nothing in this file reimplements production logic inline.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::PathBuf;

use tiles_tui_file_manager::config::fuzzy_contains;
use tiles_tui_file_manager::events::file_actions::is_valid_search_char;
use tiles_tui_file_manager::nav_helpers::{push_history, resolve_path_input};
use tiles_tui_file_manager::state::{FileColumn, FileState};

/// Benchmark: resolve_path_input (path resolution)
fn bench_resolve_path_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_path_input");

    let current = PathBuf::from("/home/user/projects");

    group.bench_function("absolute_path", |b| {
        b.iter(|| resolve_path_input("/etc/config", &current, false));
    });

    group.bench_function("relative_path", |b| {
        b.iter(|| resolve_path_input("tiles/src", &current, false));
    });

    group.bench_function("tilde_path", |b| {
        b.iter(|| resolve_path_input("~/Documents", &current, false));
    });

    group.finish();
}

fn make_file_state(path: &str, history_len: usize) -> FileState {
    let mut fs = FileState::new(
        PathBuf::from(path),
        None,
        false,
        vec![FileColumn::Name],
        FileColumn::Name,
        true,
    );
    for i in 0..history_len {
        push_history(&mut fs, PathBuf::from(format!("/dir{}", i)));
    }
    fs
}

/// Benchmark: push_history (navigation history)
fn bench_push_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_history");

    for size in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("history_len", size), &size, |b, &size| {
            let mut fs = make_file_state("/", size);
            b.iter(|| {
                // A fresh path each call exercises the append + cap paths;
                // cycling the suffix keeps history from unbounded growth.
                let next = PathBuf::from(format!("/next{}", fs.nav.history_index));
                push_history(&mut fs, next);
                black_box(&fs.nav.history_index);
            });
        });
    }

    group.bench_function("duplicate_path", |b| {
        let mut fs = make_file_state("/home", 10);
        b.iter(|| {
            push_history(&mut fs, PathBuf::from("/home"));
            black_box(&fs.nav.history_index);
        });
    });

    group.finish();
}

/// Benchmark: is_valid_search_char (search character validation)
fn bench_is_valid_search_char(c: &mut Criterion) {
    c.bench_function("is_valid_search_char", |b| {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789*_?!"
            .chars()
            .collect();
        b.iter(|| {
            for &ch in &chars {
                black_box(is_valid_search_char(ch));
            }
        });
    });
}

/// Benchmark: fuzzy_contains (search matching)
fn bench_fuzzy_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_contains");

    let names = vec![
        "main.rs",
        "cargo.toml",
        "event_helpers.rs",
        "file_manager.rs",
        "very_long_filename_with_many_underscores_and_words.rs",
    ];

    group.bench_function("short_query", |b| {
        b.iter(|| {
            for name in &names {
                black_box(fuzzy_contains(name, "main"));
            }
        });
    });

    group.bench_function("no_match_query", |b| {
        b.iter(|| {
            for name in &names {
                black_box(fuzzy_contains(name, "zzzzz"));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_resolve_path_input,
    bench_push_history,
    bench_is_valid_search_char,
    bench_fuzzy_contains,
);
criterion_main!(benches);
