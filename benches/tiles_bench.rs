//! Criterion benchmarks for Tiles TUI hot paths.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;

/// Benchmark: resolve_path_input (path resolution)
fn bench_resolve_path_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_path_input");
    
    let current = PathBuf::from("/home/user/projects");
    
    group.bench_function("absolute_path", |b| {
        b.iter(|| {
            let input = "/etc/config";
            let trimmed = input.trim();
            let typed = PathBuf::from(trimmed);
            let _ = if typed.is_absolute() {
                std::fs::canonicalize(&typed).unwrap_or(typed)
            } else {
                let joined = current.join(&typed);
                std::fs::canonicalize(&joined).unwrap_or(joined)
            };
        });
    });
    
    group.bench_function("relative_path", |b| {
        b.iter(|| {
            let input = "tiles/src";
            let trimmed = input.trim();
            let typed = PathBuf::from(trimmed);
            let _ = if typed.is_absolute() {
                typed
            } else {
                current.join(&typed)
            };
        });
    });
    
    group.finish();
}

/// Benchmark: push_history (navigation history)
fn bench_push_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_history");
    
    for size in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("history_len", size), &size, |b, &size| {
            b.iter(|| {
                let mut history: Vec<PathBuf> = (0..size)
                    .map(|i| PathBuf::from(format!("/dir{}", i)))
                    .collect();
                let new_path = PathBuf::from("/new_dir");
                // Deduplicate consecutive
                if history.last() != Some(&new_path) {
                    history.push(new_path.clone());
                }
                // Cap at 50
                if history.len() > 50 {
                    let excess = history.len() - 50;
                    history.drain(0..excess);
                }
                black_box(&history);
            });
        });
    }
    
    group.finish();
}

/// Benchmark: is_valid_search_char (search character validation)
fn bench_is_valid_search_char(c: &mut Criterion) {
    c.bench_function("is_valid_search_char", |b| {
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789*_?!"
            .chars()
            .collect();
        b.iter(|| {
            for &c in &chars {
                let bad = (c as u32) < 32 
                    || c == '\x7f' 
                    || c == '\x1b'
                    || matches!(c, '[' | ']' | '~' | '^' | '_' | '=' | '+' | '<' | '>' | '*' | '?' | '!' | '$' | '%' | '&' | '@' | '#' | '{' | '}' | '\\' | '|' | '`');
                black_box(!bad);
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
                let _ = name.to_lowercase().contains(&"main".to_lowercase());
            }
        });
    });
    
    group.bench_function("no_match_query", |b| {
        b.iter(|| {
            for name in &names {
                let _ = name.to_lowercase().contains(&"zzzzz".to_lowercase());
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
    bench_theme_accessors,
);
criterion_main!(benches);


/// Benchmark: theme accessor RwLock overhead
fn bench_theme_accessors(c: &mut Criterion) {
    use crate::ui::theme;
    c.bench_function("theme_accessor_selection_bg", |b| {
        b.iter(|| {
            let _ = theme::selection_bg();
            let _ = theme::selection_fg();
            let _ = theme::fg();
            let _ = theme::bg();
            let _ = theme::accent_primary();
            let _ = theme::accent_secondary();
            let _ = theme::border_active();
            let _ = theme::border_inactive();
            let _ = theme::selection_alt_bg();
            let _ = theme::danger();
        });
    });
    
    c.bench_function("theme_accessor_300_calls", |b| {
        b.iter(|| {
            for _ in 0..30 {
                let _ = theme::selection_bg();
                let _ = theme::selection_fg();
                let _ = theme::fg();
                let _ = theme::bg();
                let _ = theme::accent_primary();
                let _ = theme::accent_secondary();
                let _ = theme::border_active();
                let _ = theme::border_inactive();
                let _ = theme::selection_alt_bg();
                let _ = theme::danger();
            }
        });
    });
}
