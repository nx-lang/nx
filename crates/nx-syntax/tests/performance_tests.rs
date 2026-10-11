//! Parses generated files of growing size and prints how fast each one went.
//!
//! These tests only check that parsing succeeds. A wall-clock threshold here failed on slow CI
//! runners without any change to the parser, so the speeds are printed rather than asserted.

use nx_syntax::parse_str;
use std::time::Instant;

fn generate_nx_source(num_components: usize) -> String {
    let mut source = String::new();

    // Generate simple valid NX component definitions
    for i in 0..num_components {
        source.push_str(&format!(
            "let <Component{} text:string /> = <div>{{text}}</div>\n\n",
            i
        ));
    }

    source
}

#[test]
fn test_parse_performance_small() {
    // ~200 lines (100 components * 2 lines each)
    let source = generate_nx_source(100);
    let line_count = source.lines().count();

    let start = Instant::now();
    let result = parse_str(&source, "perf_test.nx");
    let duration = start.elapsed();

    if !result.is_ok() {
        eprintln!("Parse failed with {} errors", result.errors.len());
    }

    assert!(result.is_ok(), "Parse should succeed");

    let lines_per_sec = (line_count as f64 / duration.as_secs_f64()) as u64;
    println!(
        "Small file ({} lines): Parsed in {:?} ({} lines/sec)",
        line_count, duration, lines_per_sec
    );
}

#[test]
fn test_parse_performance_medium() {
    // ~2000 lines (1000 components * 2 lines each)
    let source = generate_nx_source(1000);
    let line_count = source.lines().count();

    let start = Instant::now();
    let result = parse_str(&source, "perf_test.nx");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Parse should succeed");

    let lines_per_sec = (line_count as f64 / duration.as_secs_f64()) as u64;
    println!(
        "Medium file ({} lines): Parsed in {:?} ({} lines/sec)",
        line_count, duration, lines_per_sec
    );
}

#[test]
fn test_parse_performance_large() {
    // ~6000 lines (3000 components * 2 lines each)
    let source = generate_nx_source(3000);
    let line_count = source.lines().count();

    let start = Instant::now();
    let result = parse_str(&source, "perf_test.nx");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Parse should succeed");

    let lines_per_sec = (line_count as f64 / duration.as_secs_f64()) as u64;
    println!(
        "Large file ({} lines): Parsed in {:?} ({} lines/sec)",
        line_count, duration, lines_per_sec
    );
}
