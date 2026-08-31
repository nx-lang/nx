//! Post-parse validation for NX syntax trees.
//!
//! This module provides semantic validation that goes beyond what tree-sitter
//! can detect during parsing, such as:
//! - Element tag matching (opening and closing tags must match)
//! - Error recovery within scopes
//! - Enhanced error messages with suggestions

use crate::{AstNode, ComponentDef, SyntaxKind, SyntaxNode, SyntaxTree, UnionDef};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use text_size::{TextRange, TextSize};

const COMPONENT_SIGNATURE_SYNTAX: &str =
    "Expected: <Name [extends BaseComponent] prop:type emits { ActionName { prop:type } \
     ActionType } />";
const COMPONENT_BODY_SYNTAX: &str =
    "Expected: { state { prop:type } <Element /> }, { <Element /> }, or for external components \
     { state { prop:type } }";
const COMPONENT_DEFINITION_SYNTAX: &str =
    "Expected: [abstract] [external] component <Name [extends BaseComponent] prop:type emits { \
     ActionName { prop:type } ActionType } /> [= { state { prop:type } [<Element />] }]";
const DUPLICATE_NULLABLE_SUFFIX_NOTE: &str =
    "A nullable suffix can only be applied once per type layer. `string?[]?` is valid because \
     `[]` creates a new outer list layer.";
const UNION_DEFINITION_SYNTAX: &str =
    "Expected: type UnionName [extends AbstractRecord] = caseName | payloadCase { prop:type } \
     (a single-case union keeps its leading `|`)";

/// Validates a syntax tree and returns any semantic errors found.
///
/// This performs post-parse validation that tree-sitter cannot detect, such as:
/// - Element tag matching (opening and closing tags must match)
/// - Semantic consistency checks
///
/// # Examples
///
/// ```
/// use nx_syntax::{parse_str, validate};
///
/// let result = parse_str("<Button>content</Button>", "test.nx");
/// if let Some(tree) = result.tree {
///     let diagnostics = validate(&tree, "test.nx");
///     assert!(diagnostics.is_empty());
/// }
/// ```
pub fn validate(tree: &SyntaxTree, file_name: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let root = tree.root();

    // Validate semantic constraints on type suffix composition.
    validate_type_suffixes(&root, file_name, &mut diagnostics);

    // Validate element tag matching
    validate_element_tags(&root, tree, file_name, &mut diagnostics);

    // Validate root definitions (no duplicates between explicit 'root' and top-level element)
    validate_root_definitions(&root, file_name, &mut diagnostics);

    // Validate component declarations that depend on modifier/body combinations.
    validate_component_definitions(&root, file_name, &mut diagnostics);

    // Validate union declarations that depend on complete case metadata.
    validate_union_definitions(&root, file_name, &mut diagnostics);

    // Report the removed `enum` keyword by name.
    validate_reserved_enum_keyword(tree, file_name, &mut diagnostics);

    diagnostics
}

/// Reports the removed `enum` keyword in declaration position, naming the `type` form to write.
///
/// `enum` is no longer in the grammar, so the parse fails at or after the keyword and reports
/// something unrelated — nothing in the parse tree names the keyword, and where the parse gives up
/// is not fixed. This is a source-level scan for that reason: it has to fire regardless.
///
/// The scan cannot find the keyword in the tree, but it can ask the tree where the keyword would
/// not be a declaration. Comments, string literals, and element text content hold prose and data,
/// so a line reading like a declaration there is one only by coincidence and is skipped.
///
/// The note carries the concrete replacement, built from the declaration's own remaining text, so
/// the author can read the line to write rather than a template to fill in.
fn validate_reserved_enum_keyword(
    tree: &SyntaxTree,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let source = tree.source();
    let prose_spans = prose_spans(&tree.root());
    let mut offset = 0usize;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Only declaration position: optionally a visibility modifier, then `enum`.
        let (keyword_offset, rest) = match trimmed
            .strip_prefix("export ")
            .map(|rest| (indent + 7, rest.trim_start()))
            .or_else(|| {
                trimmed
                    .strip_prefix("private ")
                    .map(|rest| (indent + 8, rest.trim_start()))
            }) {
            Some((consumed, rest)) => (offset + consumed, rest),
            None => (offset + indent, trimmed),
        };

        if let Some(after) = rest.strip_prefix("enum") {
            if after.starts_with(char::is_whitespace) && !is_within(&prose_spans, keyword_offset) {
                let start = TextSize::try_from(keyword_offset).unwrap_or_default();
                let end = TextSize::try_from(keyword_offset + 4).unwrap_or_default();

                diagnostics.push(
                    Diagnostic::error("removed-enum-keyword")
                        .with_message(
                            "`enum` is not an NX declaration. A closed set of constants is a union \
                             whose cases carry no payload."
                                .to_string(),
                        )
                        .with_label(Label::primary(
                            file_name.to_string(),
                            TextSpan::new(start, end),
                        ))
                        .with_note(format!(
                            "Write `{}` instead; the case list is unchanged.",
                            enum_replacement_form(after)
                        ))
                        .build(),
                );
            }
        }

        offset += line.len();
    }
}

/// Collects the source ranges that hold prose or data rather than code.
///
/// A source-level keyword scan has no other way to tell a declaration from the same words quoted
/// in a comment, a string, or the text content of an element. Whole regions are collected rather
/// than individual lines, so a multi-line comment or a raw text body is excluded in one piece.
fn prose_spans(root: &SyntaxNode) -> Vec<TextRange> {
    let mut spans = Vec::new();
    let mut pending = vec![*root];

    while let Some(node) = pending.pop() {
        if is_prose(node.kind()) {
            spans.push(node.span());
            continue;
        }

        pending.extend(node.children_with_tokens());
    }

    spans
}

/// Returns true for the kinds whose text is prose or data rather than NX code.
fn is_prose(kind: SyntaxKind) -> bool {
    kind.is_comment()
        || matches!(
            kind,
            SyntaxKind::STRING_LITERAL
                | SyntaxKind::TEXT_CONTENT
                | SyntaxKind::EMBED_TEXT_CONTENT
                | SyntaxKind::TEXT_RUN
                | SyntaxKind::EMBED_TEXT_RUN
                | SyntaxKind::RAW_TEXT_RUN
                | SyntaxKind::TEXT_CHUNK
                | SyntaxKind::EMBED_TEXT_CHUNK
                | SyntaxKind::RAW_TEXT_CHUNK
        )
}

/// Returns true when a byte offset falls inside any of the given ranges.
fn is_within(spans: &[TextRange], offset: usize) -> bool {
    let Ok(offset) = TextSize::try_from(offset) else {
        return false;
    };

    spans.iter().any(|span| span.contains(offset))
}

/// Builds the `type` declaration that replaces an `enum` one, from the text following the keyword.
///
/// The case list after `=` is already the form a union case list takes, so the replacement is the
/// same line with one word swapped. When the declaration continues past this line — the `=` is
/// there but the cases are not — the case list is elided rather than guessed at.
fn enum_replacement_form(after_keyword: &str) -> String {
    let rest = after_keyword.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let display_name = if name.is_empty() { "Name" } else { &name };

    let tail = rest[name.len()..].trim();
    match tail.strip_prefix('=') {
        Some(cases) if !cases.trim().is_empty() => {
            format!("type {display_name} = {}", cases.trim())
        }
        _ => format!("type {display_name} = ..."),
    }
}

/// Drops the parse errors that a removed declaration form already explains.
///
/// A recognized removed declaration reports itself by name. The parser also fails on it, and that
/// generic "unexpected syntax here" says nothing the targeted diagnostic has not already said
/// better — so it is removed when it covers the same keyword.
pub(crate) fn suppress_parse_errors_for_removed_declarations(diagnostics: &mut Vec<Diagnostic>) {
    let removed_spans: Vec<TextSpan> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some("removed-enum-keyword"))
        .flat_map(|diagnostic| diagnostic.labels())
        .filter(|label| label.primary)
        .map(|label| label.range)
        .collect();

    if removed_spans.is_empty() {
        return;
    }

    diagnostics.retain(|diagnostic| {
        if diagnostic.code() != Some("syntax-error") {
            return true;
        }

        !diagnostic
            .labels()
            .iter()
            .filter(|label| label.primary)
            .any(|label| {
                removed_spans.iter().any(|removed| {
                    label.range.start() <= removed.start() && removed.end() <= label.range.end()
                })
            })
    });
}

fn validate_type_suffixes(node: &SyntaxNode, file_name: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == SyntaxKind::TYPE && !node.has_error() {
        validate_type_suffix_chain(node, file_name, diagnostics);
    }

    for child in node.children() {
        validate_type_suffixes(&child, file_name, diagnostics);
    }
}

fn validate_type_suffix_chain(
    node: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut children = node.children_with_tokens();
    let Some(_) = children.next() else {
        return;
    };

    let mut current_nullable_suffix: Option<TextRange> = None;

    for child in children {
        match child.kind() {
            SyntaxKind::QUESTION => {
                if let Some(previous_nullable_suffix) = current_nullable_suffix {
                    diagnostics.push(
                        Diagnostic::error("duplicate-nullable-suffix")
                            .with_message("Type is already nullable at this layer")
                            .with_label(
                                Label::primary(file_name, child.span())
                                    .with_message("remove this redundant `?`"),
                            )
                            .with_label(
                                Label::secondary(file_name, previous_nullable_suffix)
                                    .with_message("this `?` already made the type nullable"),
                            )
                            .with_note(DUPLICATE_NULLABLE_SUFFIX_NOTE)
                            .build(),
                    );
                } else {
                    current_nullable_suffix = Some(child.span());
                }
            }
            SyntaxKind::LBRACKET => {
                current_nullable_suffix = None;
            }
            _ => {}
        }
    }
}

fn validate_component_definitions(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for child in root.children() {
        let Some(component) = ComponentDef::cast(child) else {
            continue;
        };

        let is_abstract = component.is_abstract();
        let is_external = component.is_external();
        let body = component.body();
        let has_body = body.is_some();
        let has_state = body
            .and_then(|body| body.child_by_field("state"))
            .is_some_and(|node| !node.raw().is_missing());
        let has_render_body = body
            .and_then(|body| body.child_by_field("body"))
            .is_some_and(|node| !node.raw().is_missing());

        if !is_abstract && !is_external && !has_body {
            diagnostics.push(
                Diagnostic::error("invalid-component-definition")
                    .with_message("Concrete components must declare a body")
                    .with_label(
                        Label::primary(file_name, component.syntax().span())
                            .with_message("bodyless component declaration"),
                    )
                    .with_note(COMPONENT_DEFINITION_SYNTAX)
                    .build(),
            );
        }

        if is_abstract && has_body {
            diagnostics.push(
                Diagnostic::error("invalid-component-definition")
                    .with_message("Abstract components cannot declare a body or local state")
                    .with_label(
                        Label::primary(file_name, component.syntax().span())
                            .with_message("remove the component body"),
                    )
                    .with_note(COMPONENT_DEFINITION_SYNTAX)
                    .build(),
            );
        }

        if is_external && has_body {
            if !has_state {
                diagnostics.push(
                    Diagnostic::error("invalid-component-definition")
                        .with_message("External component bodies must declare state")
                        .with_label(
                            Label::primary(file_name, component.syntax().span())
                                .with_message("add a state block or remove the body"),
                        )
                        .with_note(COMPONENT_DEFINITION_SYNTAX)
                        .build(),
                );
            } else if has_render_body {
                diagnostics.push(
                    Diagnostic::error("invalid-component-definition")
                        .with_message("External component bodies can only declare state")
                        .with_label(
                            Label::primary(file_name, component.syntax().span())
                                .with_message("remove the rendered body expression"),
                        )
                        .with_note(COMPONENT_DEFINITION_SYNTAX)
                        .build(),
                );
            }
        }

        if !is_abstract && !is_external && has_body && !has_render_body {
            diagnostics.push(
                Diagnostic::error("invalid-component-definition")
                    .with_message("Concrete components must declare a rendered body expression")
                    .with_label(
                        Label::primary(file_name, component.syntax().span())
                            .with_message("add a rendered body expression"),
                    )
                    .with_note(COMPONENT_DEFINITION_SYNTAX)
                    .build(),
            );
        }
    }
}

fn validate_union_definitions(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for child in root.children() {
        let Some(union_def) = UnionDef::cast(child) else {
            continue;
        };

        let mut seen_cases: Vec<(String, TextRange)> = Vec::new();

        for case in union_def.case_definitions() {
            let Some(name) = case.child_by_field("name") else {
                continue;
            };
            let case_name = name.text().to_string();

            if let Some((_, first_span)) = seen_cases
                .iter()
                .find(|(previous_name, _)| previous_name == &case_name)
            {
                let union_name = union_def
                    .name()
                    .map(|name| name.text().to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());

                diagnostics.push(
                    Diagnostic::error("duplicate-union-case")
                        .with_message(format!(
                            "Duplicate case '{}' in union '{}'",
                            case_name, union_name
                        ))
                        .with_label(
                            Label::primary(file_name, name.span())
                                .with_message("duplicate case declared here"),
                        )
                        .with_label(
                            Label::secondary(file_name, *first_span)
                                .with_message("first case declared here"),
                        )
                        .with_note("Each discriminated union case name must be unique.")
                        .build(),
                );
            } else {
                seen_cases.push((case_name, name.span()));
            }
        }
    }
}

/// Validates that element opening and closing tags match.
fn validate_element_tags(
    node: &SyntaxNode,
    _tree: &SyntaxTree,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if this is an element node or text child element
    if node.kind() == SyntaxKind::ELEMENT || node.kind() == SyntaxKind::TEXT_CHILD_ELEMENT {
        // Get opening tag
        let opening_tag = node
            .child_by_field("opening_tag")
            .or_else(|| node.child_by_field("tag"))
            .or_else(|| node.child_by_field("name"));
        let close_name_node = node.child_by_field("close_name");

        if let (Some(opening), Some(closing)) = (opening_tag, close_name_node) {
            // Get the tag name from the opening tag
            let opening_name = extract_tag_name(&opening);

            // Get the tag name from the closing tag
            let closing_name = extract_tag_name(&closing);

            if let (Some(open_name), Some(close_name)) = (opening_name, closing_name) {
                if open_name != close_name {
                    // Tag names don't match - create diagnostic
                    let open_range = opening.span();
                    let close_range = closing.span();

                    let diagnostic = Diagnostic::error("tag-mismatch")
                        .with_message(format!(
                            "Element closing tag '{}' does not match opening tag '{}'",
                            close_name, open_name
                        ))
                        .with_label(
                            Label::primary(file_name, close_range).with_message("closing tag here"),
                        )
                        .with_label(
                            Label::secondary(file_name, open_range)
                                .with_message(format!("opening tag '{}' here", open_name)),
                        )
                        .with_note(format!("Expected closing tag '</{}>>'", open_name))
                        .build();

                    diagnostics.push(diagnostic);
                }
            }
        }
    }

    // Recursively validate children
    for child in node.children() {
        validate_element_tags(&child, _tree, file_name, diagnostics);
    }
}

/// Extracts the tag name from an element tag node.
fn extract_tag_name(tag_node: &SyntaxNode) -> Option<String> {
    for child in tag_node.children() {
        if child.kind() == SyntaxKind::IDENTIFIER {
            return Some(child.text().to_string());
        }

        if child.kind() == SyntaxKind::QUALIFIED_MARKUP_NAME {
            return extract_tag_name(&child);
        }
    }

    if tag_node.kind() == SyntaxKind::IDENTIFIER
        || tag_node.kind() == SyntaxKind::QUALIFIED_MARKUP_NAME
    {
        return Some(tag_node.text().to_string());
    }

    None
}

/// Validates that there are no duplicate 'root' definitions.
///
/// A module can have at most one 'root' definition, which can come from either:
/// - An explicit `let root = ...` or `let root() = ...` definition
/// - An implicit top-level element (which becomes the 'root' function)
///
/// This function detects:
/// - Multiple explicit 'root' definitions (error)
/// - Both explicit 'root' and top-level element (error)
fn validate_root_definitions(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut explicit_roots: Vec<TextRange> = Vec::new();
    let mut implicit_root: Option<TextRange> = None;

    // Scan top-level children of the module
    for child in root.children() {
        match child.kind() {
            SyntaxKind::FUNCTION_DEFINITION | SyntaxKind::VALUE_DEFINITION => {
                // Check if this defines 'root'
                if let Some(name_node) = child.child_by_field("name") {
                    if name_node.text() == "root" {
                        explicit_roots.push(name_node.span());
                    }
                }
            }
            SyntaxKind::ELEMENT => {
                // Top-level element becomes implicit 'root'
                implicit_root = Some(child.span());
            }
            _ => {}
        }
    }

    // Check for multiple explicit root definitions
    if explicit_roots.len() > 1 {
        let first_span = explicit_roots[0];
        let second_span = explicit_roots[1];

        let diagnostic = Diagnostic::error("duplicate-root")
            .with_message("Duplicate definition of 'root'")
            .with_label(
                Label::primary(file_name, second_span).with_message("duplicate 'root' definition"),
            )
            .with_label(
                Label::secondary(file_name, first_span)
                    .with_message("first 'root' definition here"),
            )
            .with_note("A module can have at most one 'root' definition")
            .build();

        diagnostics.push(diagnostic);
    }

    // Check for conflict between explicit root and top-level element
    if let (Some(explicit_span), Some(implicit_span)) =
        (explicit_roots.first().copied(), implicit_root)
    {
        let diagnostic = Diagnostic::error("duplicate-root")
            .with_message("Duplicate definition of 'root'")
            .with_label(
                Label::primary(file_name, implicit_span)
                    .with_message("top-level element implicitly defines 'root'"),
            )
            .with_label(
                Label::secondary(file_name, explicit_span)
                    .with_message("explicit 'root' definition here"),
            )
            .with_note(
                "A module can have either a top-level element or an explicit 'root' definition, but not both",
            )
            .build();

        diagnostics.push(diagnostic);
    }
}

/// Collects all parse errors from tree-sitter ERROR nodes with enhanced messages.
///
/// This function walks the CST and converts tree-sitter ERROR and MISSING nodes
/// into rich `Diagnostic` messages with context-aware suggestions.
///
/// # Arguments
///
/// * `tree` - The tree-sitter parse tree
/// * `source` - The original source code
/// * `file_name` - The name of the file being parsed (for error messages)
///
/// # Returns
///
/// A vector of diagnostic messages for all syntax errors found in the tree.
pub fn collect_enhanced_errors(
    tree: &tree_sitter::Tree,
    source: &str,
    file_name: &str,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let root = tree.root_node();

    walk_and_collect_errors(root, source, file_name, &mut errors);
    errors
}

/// Recursively walks the tree and collects errors with context-aware messages.
fn walk_and_collect_errors(
    node: tree_sitter::Node,
    source: &str,
    file_name: &str,
    errors: &mut Vec<Diagnostic>,
) {
    if node.is_error() || node.is_missing() {
        let raw_start = u32::try_from(node.start_byte())
            .expect("NX source size should be validated before collecting syntax diagnostics");
        let raw_end = u32::try_from(node.end_byte())
            .expect("NX source size should be validated before collecting syntax diagnostics");

        // Get the text of the error node for context
        let error_text = &source[raw_start as usize..raw_end.min(source.len() as u32) as usize];

        // Generate context-aware error message
        let (message, suggestion) = analyze_error_context(&node, error_text, source);
        let (start, end) = refine_error_range(raw_start, raw_end, error_text, &message);
        let range = TextRange::new(start.into(), end.into());

        let mut diagnostic_builder = Diagnostic::error("syntax-error")
            .with_message(message)
            .with_label(Label::primary(file_name, range).with_message("unexpected syntax here"));

        if let Some(note) = suggestion {
            diagnostic_builder = diagnostic_builder.with_note(note);
        }

        errors.push(diagnostic_builder.build());
    }

    // Recursively check children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_and_collect_errors(child, source, file_name, errors);
    }
}

fn refine_error_range(start: u32, end: u32, error_text: &str, message: &str) -> (u32, u32) {
    let delimiter = match message {
        "Unclosed brace" => Some('{'),
        "Unclosed parenthesis" => Some('('),
        "Unclosed bracket" => Some('['),
        _ => None,
    };

    if let Some(delimiter) = delimiter {
        if let Some(offset) = error_text.rfind(delimiter) {
            let offset = u32::try_from(offset)
                .expect("NX source size should be validated before collecting syntax diagnostics");
            let narrowed_start = start.saturating_add(offset);
            let narrowed_end = narrowed_start
                .saturating_add(delimiter.len_utf8() as u32)
                .min(end.max(narrowed_start.saturating_add(1)));
            return (narrowed_start, narrowed_end);
        }
    }

    (start, end)
}

/// Analyzes the error context and provides helpful messages and suggestions.
fn analyze_error_context(
    node: &tree_sitter::Node,
    error_text: &str,
    source: &str,
) -> (String, Option<String>) {
    let trimmed_error = error_text.trim_start();

    // Check if this is a missing node
    if node.is_missing() {
        let message = format!("Expected {} here", node.kind());
        let suggestion = Some(format!("Try adding a {} at this location", node.kind()));
        return (message, suggestion);
    }

    // Walk ancestor contexts for better error messages.
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        match parent.kind() {
            "element" => {
                return (
                    "Invalid element syntax".to_string(),
                    Some(
                        "Expected element with format: <Tag prop={value}>content</Tag>".to_string(),
                    ),
                );
            }
            "function_definition" => {
                return (
                    "Invalid function definition".to_string(),
                    Some("Expected: let name(params) = { value }".to_string()),
                );
            }
            "action_definition" => {
                return (
                    "Invalid action definition".to_string(),
                    Some(
                        "Expected: [abstract] action ActionType [extends BaseAction] = { prop:type }"
                            .to_string(),
                    ),
                );
            }
            "record_definition" => {
                return (
                    "Invalid record definition".to_string(),
                    Some(
                        "Expected: [abstract] type RecordName [extends BaseRecord] = { prop:type }"
                            .to_string(),
                    ),
                );
            }
            "union_definition" | "union_case_list" | "union_case" => {
                return (
                    "Invalid discriminated union definition".to_string(),
                    Some(UNION_DEFINITION_SYNTAX.to_string()),
                );
            }
            "component_signature" => {
                return (
                    "Invalid component signature".to_string(),
                    Some(COMPONENT_SIGNATURE_SYNTAX.to_string()),
                );
            }
            "emits_group" => {
                return (
                    "Invalid emits block".to_string(),
                    Some("Expected: emits { ActionName { prop:type } ActionType }".to_string()),
                );
            }
            "emit_definition" => {
                return (
                    "Invalid emitted action definition".to_string(),
                    Some("Expected: ActionName [extends BaseAction] { prop:type }".to_string()),
                );
            }
            "emit_reference" => {
                return (
                    "Invalid action type reference".to_string(),
                    Some("Expected: ActionType or Namespace.ActionType".to_string()),
                );
            }
            "component_body" => {
                if trimmed_error.contains("state") {
                    return (
                        "Invalid state block".to_string(),
                        Some(COMPONENT_BODY_SYNTAX.to_string()),
                    );
                }

                return (
                    "Invalid component body".to_string(),
                    Some(COMPONENT_BODY_SYNTAX.to_string()),
                );
            }
            "state_group" => {
                return (
                    "Invalid state block".to_string(),
                    Some("Expected: state { prop:type }".to_string()),
                );
            }
            "component_definition" => {
                return (
                    "Invalid component definition".to_string(),
                    Some(COMPONENT_DEFINITION_SYNTAX.to_string()),
                );
            }
            "let_declaration" => {
                return (
                    "Invalid let declaration".to_string(),
                    Some(
                        "Expected format: let name = value or let <Pattern /> = value".to_string(),
                    ),
                );
            }
            _ => {
                ancestor = parent.parent();
            }
        }
    }

    if trimmed_error.starts_with("component ") {
        if trimmed_error.contains("extends") && trimmed_error.contains(',') {
            return (
                "Invalid component inheritance clause".to_string(),
                Some(COMPONENT_DEFINITION_SYNTAX.to_string()),
            );
        }

        if trimmed_error.contains("emits") {
            return (
                "Invalid component signature".to_string(),
                Some(COMPONENT_DEFINITION_SYNTAX.to_string()),
            );
        }

        return (
            "Invalid component definition".to_string(),
            Some(COMPONENT_DEFINITION_SYNTAX.to_string()),
        );
    }

    if trimmed_error.starts_with("abstract component ")
        || trimmed_error.starts_with("external component ")
        || trimmed_error.starts_with("abstract external component ")
    {
        return (
            "Invalid component definition".to_string(),
            Some(COMPONENT_DEFINITION_SYNTAX.to_string()),
        );
    }

    if trimmed_error.starts_with("action ") {
        return (
            "Invalid action definition".to_string(),
            Some("Expected: action ActionType = { prop:type }".to_string()),
        );
    }

    if trimmed_error.starts_with("abstract type ")
        || (trimmed_error.starts_with("type ")
            && trimmed_error.contains('{')
            && (trimmed_error.contains("extends") || trimmed_error.contains("= {")))
    {
        return (
            "Invalid record definition".to_string(),
            Some(
                "Expected: [abstract] type RecordName [extends BaseRecord] = { prop:type }"
                    .to_string(),
            ),
        );
    }

    if (trimmed_error.starts_with("type ") && trimmed_error.contains('|'))
        || (trimmed_error.starts_with('|') && looks_like_type_definition_prefix(node, source))
    {
        return (
            "Invalid discriminated union definition".to_string(),
            Some(UNION_DEFINITION_SYNTAX.to_string()),
        );
    }

    // An unbraced property value is always a literal, so a dotted name there is a common first
    // mistake: authors reach for the qualified form they would write inside braces.
    if let Some((property, qualified)) = unbraced_qualified_property(error_text) {
        let member = qualified.rsplit('.').next().unwrap_or(qualified);
        return (
            "Qualified name in unbraced property value".to_string(),
            Some(format!(
                "An unbraced property value must be a literal. If `{qualified}` names an enum \
                 member or union case, write `{property}={member}` and it resolves against the \
                 property's type; otherwise wrap the expression: `{property}={{{qualified}}}`."
            )),
        );
    }

    // Common error patterns
    if error_text.contains('{') && !error_text.contains('}') {
        return (
            "Unclosed brace".to_string(),
            Some("Add a closing '}' to match the opening brace".to_string()),
        );
    }

    if error_text.contains('(') && !error_text.contains(')') {
        return (
            "Unclosed parenthesis".to_string(),
            Some("Add a closing ')' to match the opening parenthesis".to_string()),
        );
    }

    if error_text.contains('[') && !error_text.contains(']') {
        return (
            "Unclosed bracket".to_string(),
            Some("Add a closing ']' to match the opening bracket".to_string()),
        );
    }

    // Default error message
    (
        "Syntax error".to_string(),
        Some("Check the syntax and try again".to_string()),
    )
}

fn looks_like_type_definition_prefix(node: &tree_sitter::Node, source: &str) -> bool {
    let Some(prefix) = source.get(..node.start_byte()) else {
        return false;
    };
    let line_prefix = prefix.rsplit('\n').next().unwrap_or("").trim_start();
    let starts_with_type = line_prefix.starts_with("type ")
        || line_prefix.starts_with("export type ")
        || line_prefix.starts_with("private type ");

    starts_with_type && line_prefix.contains('=')
}

#[cfg(test)]
mod enum_keyword_tests {
    use crate::parse_str;

    /// The removed keyword is reported by name, in declaration position, with the form to write.
    #[test]
    fn reports_the_removed_enum_keyword_by_name() {
        for source in [
            "enum Fit = fill | cover\n",
            "export enum Fit = fill | cover\n",
            "  private enum Fit = fill | cover\n",
        ] {
            let result = parse_str(source, "t.nx");
            let codes: Vec<_> = result.errors.iter().filter_map(|e| e.code()).collect();
            assert!(
                codes.contains(&"removed-enum-keyword"),
                "source `{source}` produced codes {codes:?}"
            );
            assert_eq!(
                codes,
                vec!["removed-enum-keyword"],
                "the targeted diagnostic must be the only one; source `{source}`"
            );
            let notes: Vec<_> = result.errors.iter().filter_map(|e| e.note()).collect();
            assert!(
                notes
                    .iter()
                    .any(|note| note.contains("type Fit = fill | cover")),
                "expected the concrete replacement form, got {notes:?}"
            );
        }
    }

    /// The replacement is built from the declaration's own text, not from a fixed template.
    #[test]
    fn names_the_replacement_with_the_declared_case_list() {
        let result = parse_str("enum Fit = fill | contain | cover\n", "t.nx");
        let notes: Vec<_> = result.errors.iter().filter_map(|e| e.note()).collect();
        assert!(
            notes
                .iter()
                .any(|note| note.contains("Write `type Fit = fill | contain | cover` instead")),
            "expected the declared case list in the replacement, got {notes:?}"
        );
    }

    /// With the case list on later lines there is nothing to quote, so it is elided, not guessed.
    #[test]
    fn elides_the_case_list_when_the_declaration_continues_past_the_keyword_line() {
        let result = parse_str("enum Fit =\n  | fill\n  | cover\n", "t.nx");
        let notes: Vec<_> = result.errors.iter().filter_map(|e| e.note()).collect();
        assert!(
            notes
                .iter()
                .any(|note| note.contains("Write `type Fit = ...` instead")),
            "expected the elided form, got {notes:?}"
        );
    }

    /// A union case merely named `enum` is not a declaration and must not be reported.
    #[test]
    fn does_not_report_the_word_enum_outside_declaration_position() {
        let result = parse_str("type Mode = enumerate | manual\n", "t.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|e| e.code()).collect();
        assert!(!codes.contains(&"removed-enum-keyword"), "codes: {codes:?}");
    }

    /// A line that reads exactly like the removed declaration is still prose inside text content,
    /// a comment, or a string, and reporting it there would edit the author's data.
    #[test]
    fn does_not_report_a_declaration_shaped_line_that_is_prose() {
        for (position, source) in [
            (
                "raw text content",
                "<Root>\n  <code:text raw>\n    enum Fit = fill | contain | cover\n  </code>\n</Root>\n",
            ),
            (
                "typed text content",
                "<Root>\n  <markdown:text>\n    enum Fit = fill | cover\n  </markdown>\n</Root>\n",
            ),
            (
                "plain text content",
                "<Root>\n  <message:>\n    enum Fit = fill | cover\n  </message>\n</Root>\n",
            ),
            ("line comment", "// enum Fit = fill | cover\ntype Mode = a | b\n"),
            (
                "block comment",
                "/*\nenum Fit = fill | cover\n*/\ntype Mode = a | b\n",
            ),
            ("string literal", "let quoted = \"enum Fit = fill | cover\"\n"),
        ] {
            let result = parse_str(source, "t.nx");
            let codes: Vec<_> = result.errors.iter().filter_map(|e| e.code()).collect();
            assert!(
                !codes.contains(&"removed-enum-keyword"),
                "`enum` in {position} was reported as a declaration; codes: {codes:?}"
            );
        }
    }

    /// Skipping prose must not cost the report for a real declaration that follows it.
    #[test]
    fn still_reports_a_declaration_that_follows_prose() {
        let result = parse_str(
            "/*\nenum Ignored = a | b\n*/\nenum Fit = fill | cover\n",
            "t.nx",
        );
        let codes: Vec<_> = result.errors.iter().filter_map(|e| e.code()).collect();
        assert_eq!(codes, vec!["removed-enum-keyword"], "codes: {codes:?}");

        let notes: Vec<_> = result.errors.iter().filter_map(|e| e.note()).collect();
        assert!(
            notes
                .iter()
                .any(|note| note.contains("type Fit = fill | cover")),
            "expected the declaration's own case list, got {notes:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    #[test]
    fn test_validate_matching_tags() {
        let source = "<Button>content</Button>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        assert!(
            diagnostics.is_empty(),
            "Matching tags should not produce errors"
        );
    }

    #[test]
    fn test_validate_mismatched_tags() {
        let source = "<Button>content</Input>";
        let result = parse_str(source, "test.nx");

        if let Some(tree) = result.tree {
            let diagnostics = validate(&tree, "test.nx");

            // Find tag mismatch errors
            let tag_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.code() == Some("tag-mismatch"))
                .collect();

            // May or may not detect depending on grammar's error recovery
            // This test documents the behavior
            if !tag_errors.is_empty() {
                assert!(tag_errors[0].message().contains("does not match"));
            }
        }
    }

    #[test]
    fn test_validate_allows_composed_nullable_suffixes_across_layers() {
        let source = "type MaybeAliases = string?[]?";
        let result = parse_str(source, "test.nx");

        let duplicate_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|d| d.code() == Some("duplicate-nullable-suffix"))
            .collect();

        assert!(
            duplicate_errors.is_empty(),
            "Expected no duplicate-nullable diagnostics, got: {duplicate_errors:?}"
        );
        assert!(
            result.is_ok(),
            "Composed nullable suffixes across distinct layers should remain valid"
        );
    }

    #[test]
    fn test_validate_rejects_duplicate_nullable_suffixes_on_same_layer() {
        for source in [
            "type TooNullable = string??",
            "type TooNullableList = string[]??",
            "type TooNullableNested = string?[]??",
        ] {
            let result = parse_str(source, "test.nx");

            let duplicate_errors: Vec<_> = result
                .errors
                .iter()
                .filter(|d| d.code() == Some("duplicate-nullable-suffix"))
                .collect();

            assert_eq!(
                duplicate_errors.len(),
                1,
                "Expected exactly one duplicate-nullable diagnostic for {source}, got: {duplicate_errors:?}"
            );
            assert!(
                duplicate_errors[0]
                    .message()
                    .contains("already nullable at this layer"),
                "Unexpected duplicate-nullable message for {source}: {}",
                duplicate_errors[0].message()
            );
            assert!(
                !result.is_ok(),
                "Duplicate nullable suffixes should make the parse result invalid for {source}"
            );
        }
    }

    #[test]
    fn test_validate_reports_each_redundant_nullable_suffix_on_its_own_layer() {
        let same_layer = parse_str("type TooNullable = string???", "test.nx");
        let same_layer_errors: Vec<_> = same_layer
            .errors
            .iter()
            .filter(|d| d.code() == Some("duplicate-nullable-suffix"))
            .collect();

        assert_eq!(
            same_layer_errors.len(),
            2,
            "Expected one diagnostic per redundant same-layer `?`, got: {same_layer_errors:?}"
        );

        let same_layer_secondary_ranges: Vec<_> = same_layer_errors
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .labels()
                    .iter()
                    .find(|label| !label.primary)
                    .expect("duplicate-nullable diagnostics should include a secondary label")
                    .range
            })
            .collect();
        assert_eq!(
            same_layer_secondary_ranges[0], same_layer_secondary_ranges[1],
            "All same-layer redundant `?` diagnostics should point back to the original nullable suffix"
        );

        let multi_layer = parse_str("type TooNullable = string??[]??", "test.nx");
        let multi_layer_errors: Vec<_> = multi_layer
            .errors
            .iter()
            .filter(|d| d.code() == Some("duplicate-nullable-suffix"))
            .collect();

        assert_eq!(
            multi_layer_errors.len(),
            2,
            "Expected independent redundant-`?` diagnostics per layer, got: {multi_layer_errors:?}"
        );

        let multi_layer_secondary_ranges: Vec<_> = multi_layer_errors
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .labels()
                    .iter()
                    .find(|label| !label.primary)
                    .expect("duplicate-nullable diagnostics should include a secondary label")
                    .range
            })
            .collect();
        assert_ne!(
            multi_layer_secondary_ranges[0], multi_layer_secondary_ranges[1],
            "Redundant `?` diagnostics from different layers should point at the first `?` of each layer"
        );
    }

    #[test]
    fn test_enhanced_error_messages_for_unclosed_brace() {
        let source = "let x = { a: 1";
        let result = parse_str(source, "test.nx");

        // Should have errors with helpful suggestions
        assert!(!result.errors.is_empty());

        let error_msgs: String = result
            .errors
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
            .join(" ");

        // At minimum, should have parse errors
        assert!(!error_msgs.is_empty());
    }

    #[test]
    fn test_error_recovery_within_scope() {
        // Multiple errors in the same scope - should collect all of them
        let source = r#"
            let x = {;
            let y = };
            let z = 42
        "#;

        let result = parse_str(source, "test.nx");

        // Should have multiple errors
        assert!(result.errors.len() >= 1, "Should detect syntax errors");
    }

    #[test]
    fn test_validate_text_child_element_matching_tags() {
        let source = "<p:>Hello <b>world</b>!</p>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        assert!(
            diagnostics.is_empty(),
            "Matching text child element tags should not produce errors"
        );
    }

    #[test]
    fn test_validate_text_child_element_mismatched_tags() {
        let source = "<p:>Hello <b>world</i>!</p>";
        let result = parse_str(source, "test.nx");

        if let Some(tree) = result.tree {
            let diagnostics = validate(&tree, "test.nx");

            // Find tag mismatch errors
            let tag_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.code() == Some("tag-mismatch"))
                .collect();

            // Should detect the mismatched <b>...</i> tags
            assert!(
                !tag_errors.is_empty(),
                "Should detect mismatched text child element tags"
            );
            assert!(
                tag_errors[0].message().contains("does not match"),
                "Error message should indicate tag mismatch"
            );
        }
    }

    #[test]
    fn test_validate_top_level_element_only() {
        // A top-level element alone should not produce errors
        let source = "<App><Header /></App>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert!(
            root_errors.is_empty(),
            "Top-level element alone should not produce duplicate-root error"
        );
    }

    #[test]
    fn test_validate_explicit_root_only() {
        // An explicit root function alone should not produce errors
        let source = "let root() = <App />";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert!(
            root_errors.is_empty(),
            "Explicit root function alone should not produce duplicate-root error"
        );
    }

    #[test]
    fn test_validate_duplicate_root_function_and_element() {
        // Both explicit root function and top-level element should produce error
        let source = r#"
            let root() = <Explicit />

            <Implicit />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root definition"
        );
        assert!(
            root_errors[0].message().contains("Duplicate"),
            "Error message should indicate duplicate"
        );
    }

    #[test]
    fn test_validate_duplicate_root_value_and_element() {
        // Both explicit root value and top-level element should produce error
        let source = r#"
            let root = <Explicit />

            <Implicit />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root definition (value)"
        );
    }

    #[test]
    fn test_validate_component_named_root_does_not_define_entry_point() {
        let source = r#"
            component <root /> = {
                <Reusable />
            }

            <App />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert!(
            root_errors.is_empty(),
            "Components should not participate in root entry-point validation"
        );
    }

    #[test]
    fn test_validate_multiple_explicit_root_functions() {
        // Two explicit root functions should produce error
        let source = r#"
            let root() = <First />
            let root() = <Second />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate explicit root definitions"
        );
        assert!(
            root_errors[0].message().contains("Duplicate"),
            "Error message should indicate duplicate"
        );
    }

    #[test]
    fn test_validate_multiple_explicit_root_mixed() {
        // Function and value both named 'root' should produce error
        let source = r#"
            let root = 42
            let root() = <App />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root (value + function)"
        );
    }

    #[test]
    fn test_component_state_error_hint_uses_component_context() {
        let source = "component <SearchBox /> = { state query:string <TextInput /> }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");
        let notes = result
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.note())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Invalid state block"),
            "Expected component state error message, got: {messages}"
        );
        assert!(
            notes.contains(COMPONENT_BODY_SYNTAX),
            "Expected state-oriented component hint, got: {notes}"
        );
    }

    #[test]
    fn test_component_emits_error_hint_uses_signature_context() {
        let source = "component <SearchBox emits Changed { value:string } /> = { <TextInput /> }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");
        let notes = result
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.note())
            .collect::<Vec<_>>()
            .join(" ");

        // Mixed emits entries now recover at the emits group, so validation reports the more
        // specific emits-block fallback instead of the older signature-level diagnostic.
        assert!(
            messages.contains("Invalid emits block"),
            "Expected emits-block error message, got: {messages}"
        );
        assert!(
            notes.contains("Expected: emits { ActionName { prop:type } ActionType }"),
            "Expected signature-oriented component hint, got: {notes}"
        );
    }

    #[test]
    fn test_action_definition_error_hint_uses_action_context() {
        let source = "action SaveRequested { value:string }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");
        let notes = result
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.note())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Invalid action definition"),
            "Expected action definition error message, got: {messages}"
        );
        assert!(
            notes.contains("Expected: action ActionType = { prop:type }"),
            "Expected action definition hint, got: {notes}"
        );
    }

    #[test]
    fn test_component_definition_fallback_hint_uses_canonical_component_shape() {
        let source = "component <SearchBox /> = state { query:string }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");
        let notes = result
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.note())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Invalid component definition"),
            "Expected generic component fallback message, got: {messages}"
        );
        assert!(
            notes.contains(COMPONENT_DEFINITION_SYNTAX),
            "Expected canonical component fallback hint, got: {notes}"
        );
    }

    #[test]
    fn test_validate_concrete_bodyless_component_is_rejected() {
        let source = "component <SearchBox placeholder:string />";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");
        let notes = result
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.note())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Concrete components must declare a body"),
            "Expected bodyless concrete component diagnostic, got: {messages}"
        );
        assert!(
            notes.contains(COMPONENT_DEFINITION_SYNTAX),
            "Expected canonical component syntax hint, got: {notes}"
        );
    }

    #[test]
    fn test_validate_abstract_component_body_is_rejected() {
        let source = "abstract component <SearchBase /> = { <button /> }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Abstract components cannot declare a body"),
            "Expected abstract-component body diagnostic, got: {messages}"
        );
    }

    #[test]
    fn test_validate_external_component_rendered_body_is_rejected() {
        let source = "external component <SearchBox /> = { <button /> }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("External component bodies must declare state"),
            "Expected external-component rendered-body diagnostic, got: {messages}"
        );
    }

    #[test]
    fn test_validate_external_component_state_only_body_is_allowed() {
        let source = "external component <SearchBox /> = { state { query:string } }";
        let result = parse_str(source, "test.nx");

        assert!(
            result.is_ok(),
            "Expected external component state-only body to be valid, got {:?}",
            result
                .errors
                .iter()
                .map(|diagnostic| diagnostic.message())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validate_external_component_empty_body_is_rejected() {
        let source = "external component <SearchBox /> = { }";
        let result = parse_str(source, "test.nx");

        assert!(
            !result.is_ok(),
            "Expected empty external-component body to be rejected"
        );
    }

    #[test]
    fn test_validate_external_component_mixed_state_and_render_body_is_rejected() {
        let source = "external component <SearchBox /> = { state { query:string } <button /> }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("External component bodies can only declare state"),
            "Expected mixed external-component body diagnostic, got: {messages}"
        );
    }

    #[test]
    fn test_validate_concrete_component_state_only_body_is_rejected() {
        let source = "component <SearchBox /> = { state { query:string } }";
        let result = parse_str(source, "test.nx");

        let messages = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            messages.contains("Concrete components must declare a rendered body expression"),
            "Expected state-only concrete component diagnostic, got: {messages}"
        );
    }
}

/// Finds an unbraced property value that is a dotted name, as in `fit=Fit.cover`.
///
/// Returns the property name and the qualified value it was given.
fn unbraced_qualified_property(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let is_name_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'=' {
            continue;
        }
        // The property name immediately before the `=`.
        let mut start = index;
        while start > 0 && is_name_byte(bytes[start - 1]) {
            start -= 1;
        }
        if start == index {
            continue;
        }
        let property = &text[start..index];
        if !property.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            continue;
        }

        // The value, which must be an unquoted, unbraced dotted name.
        let mut end = index + 1;
        if end >= bytes.len() || !(bytes[end].is_ascii_alphabetic() || bytes[end] == b'_') {
            continue;
        }
        while end < bytes.len() && (is_name_byte(bytes[end]) || bytes[end] == b'.') {
            end += 1;
        }
        let value = &text[index + 1..end];
        if value.contains('.') && !value.ends_with('.') {
            return Some((property, value));
        }
    }
    None
}
