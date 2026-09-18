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

/// Runs the highlights query over `source` and returns `(capture name, matched text)` pairs in
/// source order.
fn highlight_captures(source: &str) -> Vec<(String, String)> {
    let language = nx_syntax::language();
    let query_source = fs::read_to_string(queries_dir().join("highlights.scm")).unwrap();
    let query = Query::new(&language, &query_source).unwrap();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut captures = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    use tree_sitter::StreamingIterator;
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let name = query.capture_names()[capture.index as usize].to_string();
            let text = capture
                .node
                .utf8_text(source.as_bytes())
                .unwrap()
                .to_string();
            captures.push((capture.node.start_byte(), name, text));
        }
    }
    captures.sort();
    captures
        .into_iter()
        .map(|(_, name, text)| (name, text))
        .collect()
}

#[test]
fn highlights_capture_the_parts_of_a_function_type() {
    let captures = highlight_captures("type T = (<function Item:Contact Index:int />: DrawnNode)?");
    let has = |name: &str, text: &str| captures.iter().any(|(n, t)| n == name && t == text);
    assert!(has("keyword", "function"), "{captures:?}");
    assert!(has("variable.parameter", "Item"), "{captures:?}");
    assert!(has("variable.parameter", "Index"), "{captures:?}");
    assert!(has("type", "Contact"), "{captures:?}");
    assert!(has("type.builtin", "int"), "{captures:?}");
    assert!(has("type", "DrawnNode"), "{captures:?}");
    assert!(has("punctuation.bracket", "("), "{captures:?}");
    assert!(has("punctuation.bracket", ")"), "{captures:?}");
}

#[test]
fn highlights_do_not_treat_the_identifier_function_as_a_keyword() {
    let captures = highlight_captures("let function = 1\nlet v = {function}");
    assert!(
        !captures
            .iter()
            .any(|(name, text)| name == "keyword" && text == "function"),
        "{captures:?}"
    );
}
