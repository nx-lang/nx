use nx_syntax::{SyntaxKind, SyntaxNode};

#[allow(dead_code)]
pub fn contains_kind(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    if node.kind() == kind {
        return true;
    }

    node.children().any(|child| contains_kind(&child, kind))
}

#[allow(dead_code)]
pub fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    let mut count = if node.kind() == kind { 1 } else { 0 };
    for child in node.children() {
        count += count_kind(&child, kind);
    }
    count
}

#[allow(dead_code)]
pub fn find_first_kind<'tree>(
    node: &SyntaxNode<'tree>,
    kind: SyntaxKind,
) -> Option<SyntaxNode<'tree>> {
    if node.kind() == kind {
        return Some(*node);
    }

    for child in node.children() {
        if let Some(found) = find_first_kind(&child, kind) {
            return Some(found);
        }
    }

    None
}

#[allow(dead_code)]
pub fn collect_kinds<'tree>(
    node: &SyntaxNode<'tree>,
    kind: SyntaxKind,
    out: &mut Vec<SyntaxNode<'tree>>,
) {
    if node.kind() == kind {
        out.push(*node);
    }

    for child in node.children() {
        collect_kinds(&child, kind, out);
    }
}

/// True when any node in the subtree was inserted by error recovery rather than read from source.
///
/// Distinct from `is_error`: tree-sitter recovers a missing token with a zero-width `MISSING` node,
/// which is not an `ERROR` node, so a tree can look well-formed by kind alone and still be one the
/// parser repaired.
#[allow(dead_code)]
pub fn contains_missing(node: &SyntaxNode) -> bool {
    if node.raw().is_missing() {
        return true;
    }

    node.children_with_tokens()
        .any(|child| contains_missing(&child))
}
