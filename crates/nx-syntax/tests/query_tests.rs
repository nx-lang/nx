//! Compiles every shipped Tree-sitter query against the NX grammar.
//!
//! The `.scm` assets under `queries/` are consumed by editors, not by the Rust parser, so nothing
//! else in the suite ever compiles them. A query naming a node or token the grammar no longer has
//! fails at compile time in the consumer and stays green here — which is exactly how the removed
//! `enum` patterns survived. This test closes that gap.

use std::fs;
use std::path::PathBuf;
use tree_sitter::Query;

/// Resolves the query asset directory from either the crate or the workspace root.
fn queries_dir() -> PathBuf {
    let from_crate = PathBuf::from("queries");
    let from_workspace = PathBuf::from("crates/nx-syntax/queries");

    if from_crate.exists() {
        from_crate
    } else {
        from_workspace
    }
}

/// Returns every shipped query asset, so a newly added `.scm` is covered without editing this test.
fn shipped_queries() -> Vec<PathBuf> {
    let dir = queries_dir();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
        .map(|entry| entry.expect("failed to read query directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "scm"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn shipped_queries_compile_against_the_grammar() {
    let language = nx_syntax::language();
    let paths = shipped_queries();
    assert!(!paths.is_empty(), "no shipped query assets were found");

    for path in paths {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if let Err(error) = Query::new(&language, &source) {
            panic!(
                "{} does not compile against the grammar: {error}",
                path.display()
            );
        }
    }
}

#[test]
fn shipped_queries_do_not_reference_the_removed_enum_declaration() {
    for path in shipped_queries() {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            !source.contains("enum"),
            "{} still references the removed `enum` declaration form",
            path.display()
        );
    }
}
