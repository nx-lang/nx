//! Type checks generated files of growing size and prints how long each one took.
//!
//! These tests only check that type checking finishes. Wall-clock thresholds fail on slow CI
//! runners without any change to the checker, so the times are printed rather than asserted.

use nx_types::check_str;
use std::time::Instant;

fn generate_nx_source_with_types(num_components: usize) -> String {
    let mut source = String::new();

    // Generate NX component definitions with type checking
    for i in 0..num_components {
        source.push_str(&format!(
            "let <Component{} text:string count:int /> = <div>{{text}} - {{count}}</div>\n\n",
            i
        ));
    }

    source
}

#[test]
fn test_typecheck_performance_small() {
    // ~200 lines (100 components)
    let source = generate_nx_source_with_types(100);
    let line_count = source.lines().count();

    let start = Instant::now();
    let _result = check_str(&source, "typecheck_perf_test.nx");
    let duration = start.elapsed();

    println!(
        "Small file ({} lines): Type checked in {:?}",
        line_count, duration
    );
}

#[test]
fn test_typecheck_performance_medium() {
    // ~2000 lines (1000 components)
    let source = generate_nx_source_with_types(1000);
    let line_count = source.lines().count();

    let start = Instant::now();
    let _result = check_str(&source, "typecheck_perf_test.nx");
    let duration = start.elapsed();

    println!(
        "Medium file ({} lines): Type checked in {:?}",
        line_count, duration
    );
}

#[test]
fn test_typecheck_performance_large() {
    // ~10000 lines (5000 components)
    let source = generate_nx_source_with_types(5000);
    let line_count = source.lines().count();

    let start = Instant::now();
    let _result = check_str(&source, "typecheck_perf_test.nx");
    let duration = start.elapsed();

    println!(
        "Large file ({} lines): Type checked in {:?}",
        line_count, duration
    );
}

#[test]
fn test_typecheck_performance_with_errors() {
    // Generate code with type errors to test error path performance
    let mut source = String::new();

    for i in 0..1000 {
        // Mix valid and invalid components
        if i % 2 == 0 {
            source.push_str(&format!(
                "let <Valid{} text:string /> = <div>{{text}}</div>\n\n",
                i
            ));
        } else {
            source.push_str(&format!(
                "let <Invalid{} text:string /> = <div>{{undefined_var}}</div>\n\n",
                i
            ));
        }
    }

    let line_count = source.lines().count();

    let start = Instant::now();
    let result = check_str(&source, "typecheck_perf_test.nx");
    let duration = start.elapsed();

    println!(
        "File with errors ({} lines, {} errors): Type checked in {:?}",
        line_count,
        result.all_diagnostics().len(),
        duration
    );
}
