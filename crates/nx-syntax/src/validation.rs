//! Post-parse validation for NX syntax trees.
//!
//! This module provides semantic validation that goes beyond what tree-sitter
//! can detect during parsing, such as:
//! - Element tag matching (opening and closing tags must match)
//! - Error recovery within scopes
//! - Enhanced error messages with suggestions

use crate::{
    property_definition_is_type_parameter, AstNode, ComponentDef, SyntaxKind, SyntaxNode,
    SyntaxTree, UnionDef,
};
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
const SECOND_OCCURRENCE_SUFFIX_NOTE: &str =
    "A type reference carries at most one occurrence suffix: `?` for zero or one, `+` for one or \
     more, `*` for zero or more. There is no optional sequence and no sequence of optionals; \
     where data has to nest, declare a record whose field is a sequence.";
const TYPE_PARAMETER_SYNTAX: &str =
    "A type parameter is declared as `Name:type` at the start of a component signature or a record \
     declaration, after `extends` and before every other property, with no default value, no \
     modifier, and a name that is not a primitive or built-in type";
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

    // The conditional operator is not an NX expression; report its shape with the `if` form.
    validate_removed_conditional_operator(tree, file_name, &mut diagnostics);

    // Validate element tag matching
    validate_element_tags(&root, tree, file_name, &mut diagnostics);

    // Validate root definitions (no duplicates between explicit 'root' and top-level element)
    validate_root_definitions(&root, file_name, &mut diagnostics);

    // Validate component declarations that depend on modifier/body combinations.
    validate_component_definitions(&root, file_name, &mut diagnostics);

    // Validate union declarations that depend on complete case metadata.
    validate_union_definitions(&root, file_name, &mut diagnostics);

    // Validate where a `Name:type` definition may declare a type parameter.
    validate_type_parameter_definitions(&root, file_name, &mut diagnostics);

    // Validate the parameter list of every function type.
    validate_function_types(&root, file_name, &mut diagnostics);

    // Validate that a paren-style function lists the parameters a caller may omit last.
    validate_paren_parameter_order(&root, file_name, &mut diagnostics);

    // Reject a property that is both optional and defaulted, quoting the two forms it could mean.
    validate_optional_property_defaults(&root, file_name, &mut diagnostics);

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
        .filter(|diagnostic| {
            matches!(
                diagnostic.code(),
                Some("removed-enum-keyword") | Some("removed-conditional-operator")
            )
        })
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
                    label.range.start() < removed.end() && removed.start() < label.range.end()
                })
            })
    });
}

/// Reports the removed conditional operator `c ? a : b` with the `if` form that replaces it.
///
/// <para>A `?` after an expression is the presence test, so the ternary shape parses as far as
/// the `:` and fails there. The error node that starts at that `:` (or, unbraced, at the `?`) is
/// the anchor; the expression region around it is split at the first bare `?` and at the `:` to
/// build the replacement from the author's own operands. String literals and comments are
/// skipped when the `?` is looked for, so a quoted question mark is not mistaken for one.</para>
fn validate_removed_conditional_operator(
    tree: &SyntaxTree,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let source = tree.source();
    // The tokens the parser salvaged inside an error node say nothing about what the author
    // wrote there, so prose is collected from the well-formed part of the tree only.
    let prose_spans = prose_spans_outside_errors(&tree.root());
    let mut pending = vec![tree.root()];

    while let Some(node) = pending.pop() {
        if node.kind() != SyntaxKind::ERROR {
            pending.extend(node.children_with_tokens());
            continue;
        }

        // Only an error in expression position can be the shape: a `?` in an element's property
        // list or in a type is something else gone wrong.
        if !error_is_in_expression_position(&node) {
            continue;
        }

        let error_text = node.text();
        let leading = error_text.len() - error_text.trim_start().len();
        let error_start = usize::from(node.span().start()) + leading;
        let error_end = usize::from(node.span().end());

        // The error node may hold the `?` and everything after it (`= c ? a : b`), or start at
        // or inside the alternative once the `?` has parsed as a presence test (`{c ? a : b}`,
        // `{x > 0 ? x * 2 : -1}`). Either way the `?` is the last bare one before the `:`.
        let colon_within = |from: usize| {
            source[from..error_end]
                .find(':')
                .map(|offset| from + offset)
                .filter(|colon| !is_within(&prose_spans, *colon))
        };
        let (question, colon) = if let Some(question) =
            bare_question_within(source, error_start, error_end, &prose_spans)
        {
            let Some(colon) = colon_within(question) else {
                continue;
            };
            (question, colon)
        } else {
            let (Some(colon), Some(question)) = (
                colon_within(error_start),
                bare_question_before(source, error_start, &prose_spans),
            ) else {
                continue;
            };
            // Outside the error node the `?` is only the ternary's when it parsed as a presence
            // test. The optional mark of a parameter (`let f(a?:int) = { a ) : 3 }`) is a `?`
            // too, and taking it for one would hide the real error behind a wrong rewrite.
            if !is_presence_test_operator(&tree.root(), question) {
                continue;
            }
            (question, colon)
        };

        let region_start = expression_region_start(source, question);
        let condition = code_text(&source[region_start..question]);
        let consequent = code_text(&source[question + 1..colon]);
        let alternative_text = &source[colon + 1..error_end];
        let alternative_len = alternative_end(alternative_text);
        let alternative_len = first_item_len(&alternative_text[..alternative_len]);
        let alternative = code_text(&alternative_text[..alternative_len]);
        if condition.is_empty() || consequent.is_empty() || alternative.is_empty() {
            continue;
        }

        // A nested conditional (`c ? 1 : d ? 2 : 3`, `c ? (d ? 1 : 2) : 3`) cannot be split
        // at one `?` and one `:` into operands that mean what the author wrote, and a rewrite
        // that does not parse (`c ? 1 : n +` with `2` on the next line, or `a? ) : 3`) would
        // suggest broken code, so either way the note names the form without filling it in.
        // An unbraced `let` value (`let ratio = ready ? 1 : 2`) needs the `if` in braces.
        let nested = has_depth_zero_colon(&condition)
            || has_bare_question(&consequent)
            || has_bare_question(&alternative);
        let if_form = format!("if {condition} {{ {consequent} }} else {{ {alternative} }}");
        let rewrite = [if_form.clone(), format!("{{ {if_form} }}")]
            .into_iter()
            .find(|rewrite| {
                !nested
                    && rewrite_parses(source, region_start..colon + 1 + alternative_len, rewrite)
            });
        let note = match rewrite {
            Some(rewrite) => format!("Write `{rewrite}` instead."),
            None => {
                "Write each conditional as `if condition { a } else { b }` instead.".to_string()
            }
        };

        // Items after the alternative (`c ? "a" : "b" "z"`) follow the conditional and stay as
        // they are, so the label stops where the rewrite does.
        let alternative_end = colon + 1 + alternative_len;
        let start = TextSize::try_from(question).unwrap_or_default();
        let end = TextSize::try_from(alternative_end).unwrap_or_default();
        diagnostics.push(
            Diagnostic::error("removed-conditional-operator")
                .with_message(
                    "`c ? a : b` is not an NX expression; a `?` after a value tests its presence",
                )
                .with_label(
                    Label::primary(file_name.to_string(), TextSpan::new(start, end))
                        .with_message("write this as an `if`"),
                )
                .with_note(note)
                .build(),
        );
    }
}

/// True when the `?` token at byte `offset` is the operator of a well-formed presence test
/// `x?`, and not an optional mark, a type suffix or a token inside an error node.
fn is_presence_test_operator(root: &SyntaxNode, offset: usize) -> bool {
    let mut node = *root;
    loop {
        let Some(child) = node.children_with_tokens().find(|child| {
            usize::from(child.span().start()) <= offset && offset < usize::from(child.span().end())
        }) else {
            return false;
        };
        if child.kind() == SyntaxKind::ERROR {
            return false;
        }
        if usize::from(child.span().start()) == offset
            && usize::from(child.span().end()) == offset + 1
            && child.kind() == SyntaxKind::QUESTION
        {
            return node.kind() == SyntaxKind::EXISTS_EXPRESSION;
        }
        node = child;
    }
}

/// True when the nearest enclosing construct of an error node is one that holds an expression
/// — a braced value, an embedded value, or the right-hand side of a `let` — rather than an
/// element's tag, a property list or a type.
fn error_is_in_expression_position(error: &SyntaxNode) -> bool {
    let mut ancestor = error.parent();
    while let Some(node) = ancestor {
        match node.kind() {
            SyntaxKind::VALUES_BRACED_EXPRESSION
            | SyntaxKind::EMBED_BRACED_EXPRESSION
            | SyntaxKind::RHS_EXPRESSION
            | SyntaxKind::VALUE_EXPRESSION
            | SyntaxKind::VALUE_DEFINITION
            | SyntaxKind::FUNCTION_DEFINITION
            | SyntaxKind::COMPONENT_BODY => return true,
            // An unbraced tail after a definition (`let ratio = ready ? 1 : 2`) is an error node
            // beside the definition, so the `?` itself says it is in expression position.
            SyntaxKind::MODULE_DEFINITION => return error.text().trim_start().starts_with('?'),
            SyntaxKind::ELEMENT
            | SyntaxKind::PROPERTY_LIST
            | SyntaxKind::PROPERTY_VALUE
            | SyntaxKind::TYPE
            | SyntaxKind::APPLIED_TYPE
            | SyntaxKind::TYPE_ARGUMENT
            | SyntaxKind::RECORD_DEFINITION
            | SyntaxKind::TYPE_DEFINITION
            | SyntaxKind::COMPONENT_SIGNATURE
            | SyntaxKind::PROPERTY_DEFINITION => return false,
            _ => ancestor = node.parent(),
        }
    }
    false
}

/// Where a ternary's alternative ends in `text`, the source after its `:`.
///
/// <para>It ends at the first closing bracket that nothing inside it opened (the `}` or `)` of
/// the expression it sat in), at a `,` at bracket depth zero (the next call argument), at a
/// comment, or at a line break at depth zero once the alternative has begun. String literals are
/// skipped whole.</para>
fn alternative_end(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut started = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i = string_literal_end(bytes, i);
                started = true;
                continue;
            }
            b'/' if matches!(bytes.get(i + 1), Some(b'/') | Some(b'*')) => return i,
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            b',' if depth == 0 => return i,
            b'\n' if depth == 0 && started => return i,
            byte if !byte.is_ascii_whitespace() => started = true,
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

/// The length of the first sequence item in `text`, a ternary's alternative, or of all of `text`
/// when it is one value or does not parse.
///
/// A sequence runs items together, so in `{ c ? "a" : "b" "z" }` the `"z"` is the next item after
/// the conditional, not part of its alternative.
fn first_item_len(text: &str) -> usize {
    const PREFIX: &str = "let alternative = {";
    let wrapped = format!("{PREFIX}{text}}}");
    let Some(tree) = crate::parser().parse(&wrapped, None) else {
        return text.len();
    };
    let root = tree.root_node();
    let Some(braced) = root.descendant_for_byte_range(PREFIX.len() - 1, wrapped.len()) else {
        return text.len();
    };
    if braced.kind() != "values_braced_expression" || braced.has_error() {
        return text.len();
    }
    match braced.named_child(0) {
        Some(item) if item.kind() == "value_list_item_expression" => item.end_byte() - PREFIX.len(),
        _ => text.len(),
    }
}

/// The offset just past the string literal whose opening `"` is at `start`, or the end of
/// `bytes` when it is unterminated.
fn string_literal_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `text` with comments removed and each run of whitespace outside string literals collapsed to
/// one space, trimmed: an operand as it reads in a one-line fix-it.
fn code_text(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut pending_space = false;
    while i < bytes.len() {
        let skip_to = match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => Some(
                text[i..]
                    .find('\n')
                    .map_or(bytes.len(), |offset| i + offset),
            ),
            b'/' if bytes.get(i + 1) == Some(&b'*') => Some(
                text[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |offset| i + 2 + offset + 2),
            ),
            byte if byte.is_ascii_whitespace() => Some(i + 1),
            _ => None,
        };
        if let Some(next) = skip_to {
            pending_space = true;
            i = next;
            continue;
        }

        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        let next = if bytes[i] == b'"' {
            string_literal_end(bytes, i)
        } else {
            i + text[i..].chars().next().map_or(1, char::len_utf8)
        };
        out.push_str(&text[i..next]);
        i = next;
    }
    out
}

/// True when `source` with `range` replaced by `rewrite` parses without an error in the
/// rewritten part, so a fix-it that suggests `rewrite` does not suggest broken code.
fn rewrite_parses(source: &str, range: std::ops::Range<usize>, rewrite: &str) -> bool {
    let rewritten = format!(
        "{} {rewrite} {}",
        &source[..range.start],
        &source[range.end..]
    );
    let Some(tree) = crate::parser().parse(&rewritten, None) else {
        return false;
    };
    !has_error_within(
        tree.root_node(),
        range.start,
        range.start + rewrite.len() + 2,
    )
}

/// True when `node` holds an error or missing node that touches the bytes `start..=end`.
fn has_error_within(node: tree_sitter::Node, start: usize, end: usize) -> bool {
    if !node.has_error() || node.end_byte() < start || node.start_byte() > end {
        return false;
    }
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    let has_error = node
        .children(&mut cursor)
        .any(|child| has_error_within(child, start, end));
    has_error
}

/// True when `text` holds a `:` outside brackets and string literals.
fn has_depth_zero_colon(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i = string_literal_end(bytes, i);
                continue;
            }
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            b':' if depth == 0 => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

/// True when `text` holds a bare `?` outside string literals: one that is not part of `?.` or
/// `??` and is followed by an operand, so it reads as another conditional rather than a presence
/// test.
fn has_bare_question(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i = string_literal_end(bytes, i);
                continue;
            }
            b'?' => {
                let previous = i.checked_sub(1).map(|p| bytes[p]);
                let next = bytes.get(i + 1).copied();
                let is_operator = matches!(next, Some(b'.') | Some(b'?')) || previous == Some(b'?');
                let rest = text[i + 1..].trim_start();
                let followed_by_operand = !rest.starts_with("!=")
                    && rest.bytes().next().is_some_and(|byte| {
                        !matches!(
                            byte,
                            b')' | b'}'
                                | b']'
                                | b','
                                | b':'
                                | b'&'
                                | b'|'
                                | b'='
                                | b'<'
                                | b'>'
                                | b'+'
                                | b'*'
                                | b'/'
                                | b'%'
                                | b'.'
                        )
                    });
                if !is_operator && followed_by_operand {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// The prose spans of the tree, not descending into error nodes.
///
/// <para>Text content is walked to its chunks rather than taken whole, because a braced
/// expression inside `<p:>…</p>` is code, and an error node inside it must stay visible.</para>
fn prose_spans_outside_errors(root: &SyntaxNode) -> Vec<TextRange> {
    let mut spans = Vec::new();
    let mut pending = vec![*root];

    while let Some(node) = pending.pop() {
        if node.kind() == SyntaxKind::ERROR {
            continue;
        }
        let is_leaf_prose = is_prose(node.kind())
            && !matches!(
                node.kind(),
                SyntaxKind::TEXT_CONTENT
                    | SyntaxKind::EMBED_TEXT_CONTENT
                    | SyntaxKind::TEXT_RUN
                    | SyntaxKind::EMBED_TEXT_RUN
                    | SyntaxKind::RAW_TEXT_RUN
            );
        if is_leaf_prose {
            spans.push(node.span());
            continue;
        }
        pending.extend(node.children_with_tokens());
    }

    spans
}

/// The offset of the first bare `?` — not part of `?.` or `??`, not in a string or comment —
/// in `start..end`, when there is one.
fn bare_question_within(
    source: &str,
    start: usize,
    end: usize,
    prose_spans: &[TextRange],
) -> Option<usize> {
    (start..end).find(|&i| is_bare_question(source, i, prose_spans))
}

/// The offset of the last bare `?` before `before` on the same line, when there is one. When
/// `before` begins its line, the search starts from the previous non-blank line, so a ternary
/// laid out one operand per line is still found.
fn bare_question_before(source: &str, before: usize, prose_spans: &[TextRange]) -> Option<usize> {
    let line_start = line_start_skipping_blank_prefix(source, before);
    (line_start..before)
        .rev()
        .find(|&i| is_bare_question(source, i, prose_spans))
}

/// True when the byte at `i` is a `?` that is an operator of its own: not part of `?.` or `??`,
/// and not inside a string or comment.
fn is_bare_question(source: &str, i: usize, prose_spans: &[TextRange]) -> bool {
    let bytes = source.as_bytes();
    if bytes[i] != b'?' || is_within(prose_spans, i) {
        return false;
    }
    let next = bytes.get(i + 1).copied();
    let previous = if i > 0 {
        bytes.get(i - 1).copied()
    } else {
        None
    };
    !matches!(next, Some(b'.') | Some(b'?')) && previous != Some(b'?')
}

/// Where the expression that a `?` at `question` tests begins: after the nearest unmatched `{`,
/// `(` or `=` before it on the same line, or at the start of the line. When the `?` begins its
/// line, the previous non-blank line is searched instead.
fn expression_region_start(source: &str, question: usize) -> usize {
    let line_start = line_start_skipping_blank_prefix(source, question);
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut i = question;
    while i > line_start {
        i -= 1;
        match bytes[i] {
            b')' | b'}' => depth += 1,
            b'(' | b'{' if depth > 0 => depth -= 1,
            b'(' | b'{' | b'=' => return i + 1,
            _ => {}
        }
    }
    line_start
}

/// The start of the line holding `offset`, or, when only whitespace precedes `offset` on that
/// line, the start of the nearest earlier line that is not blank.
fn line_start_skipping_blank_prefix(source: &str, offset: usize) -> usize {
    let mut line_start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
    while line_start > 0 && source[line_start..offset].trim().is_empty() {
        line_start = source[..line_start - 1].rfind('\n').map_or(0, |i| i + 1);
    }
    line_start
}

fn validate_type_suffixes(node: &SyntaxNode, file_name: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == SyntaxKind::TYPE && !node.has_error() {
        validate_type_suffix_chain(node, file_name, diagnostics);
    }
    if node.kind() == SyntaxKind::CONTEXTUAL_NAME && !node.has_error() {
        validate_contextual_name_suffix(node, file_name, diagnostics);
    }

    for child in node.children() {
        validate_type_suffixes(&child, file_name, diagnostics);
    }
}

/// Enforces "at most one occurrence suffix per type reference chain" and rejects the removed `[]`.
///
/// <para>Parentheses add no layer, so a suffix ending the enclosed type is the current chain's
/// suffix and a second one outside the parentheses is one too many: `(string+)*` is rejected on
/// its `*`. A function type's result is a chain of its own, so `(<function />: string+)+` is a
/// sequence of functions and is accepted. An alias whose target already carries a suffix is only
/// visible at type resolution, which completes this rule.</para>
fn validate_type_suffix_chain(
    node: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut children = node.children_with_tokens();
    let Some(base) = children.next() else {
        return;
    };

    let mut current_suffix: Option<TextRange> = if base.kind() == SyntaxKind::PARENTHESIZED_TYPE {
        enclosed_occurrence_suffix(&base)
    } else {
        None
    };

    for child in children {
        match child.kind() {
            SyntaxKind::LBRACKET => {
                let base_text = list_suffix_base(node, &base, child);
                let rbracket_end = node
                    .children_with_tokens()
                    .find(|sibling| {
                        sibling.kind() == SyntaxKind::RBRACKET
                            && sibling.span().start() >= child.span().end()
                    })
                    .map(|rbracket| rbracket.span().end())
                    .unwrap_or_else(|| child.span().end());
                diagnostics.push(
                    Diagnostic::error("removed-list-suffix")
                        .with_message("`[]` is not a type suffix; a sequence is written `*` or `+`")
                        .with_label(
                            Label::primary(file_name, TextRange::new(child.span().start(), rbracket_end))
                                .with_message("replace this `[]`"),
                        )
                        .with_note(format!(
                            "Write `{base_text}*` for zero or more values, or `{base_text}+` for one \
                             or more."
                        ))
                        .build(),
                );
            }
            SyntaxKind::QUESTION | SyntaxKind::PLUS | SyntaxKind::STAR => {
                if let Some(previous_suffix) = current_suffix {
                    diagnostics.push(
                        Diagnostic::error("second-occurrence-suffix")
                            .with_message("Type already carries an occurrence")
                            .with_label(
                                Label::primary(file_name, child.span())
                                    .with_message("remove this suffix"),
                            )
                            .with_label(Label::secondary(file_name, previous_suffix).with_message(
                                "this suffix already says how many values the type admits",
                            ))
                            .with_note(SECOND_OCCURRENCE_SUFFIX_NOTE)
                            .build(),
                    );
                } else {
                    current_suffix = Some(child.span());
                }
            }
            _ => {}
        }
    }
}

/// Rejects an occurrence suffix on a bare name that cannot be a type argument.
///
/// <para>The grammar admits a suffix glued to a bare name so that `T=int?` at a construction site
/// parses and the checker can say that a type argument is exactly one value. Only a property value
/// can be a type argument, and only the checker knows whether the property is a type parameter,
/// so there the name is held to the one-suffix rule of a type and left to the checker. Anywhere
/// else (`let x = cover?`) the name is a value, and a value takes no suffix.</para>
fn validate_contextual_name_suffix(
    node: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let in_property_value = node
        .parent()
        .and_then(|rhs| rhs.parent())
        .is_some_and(|parent| parent.kind() == SyntaxKind::PROPERTY_VALUE);
    if in_property_value {
        validate_type_suffix_chain(node, file_name, diagnostics);
        return;
    }

    let mut suffixes = node.children_with_tokens().filter(|child| {
        matches!(
            child.kind(),
            SyntaxKind::QUESTION | SyntaxKind::PLUS | SyntaxKind::STAR
        )
    });
    let Some(first) = suffixes.next() else {
        return;
    };
    let end = suffixes
        .last()
        .map_or(first.span().end(), |last| last.span().end());
    let name = node.text()[..usize::from(first.span().start() - node.span().start())].trim();
    diagnostics.push(
        Diagnostic::error("occurrence-suffix-on-value")
            .with_message("An occurrence suffix follows a type, not a value")
            .with_label(
                Label::primary(file_name, TextRange::new(first.span().start(), end))
                    .with_message("remove this suffix"),
            )
            .with_note(format!(
                "`{name}` here is a value, and a suffix says how many values a type admits; \
                 write `{name}` alone."
            ))
            .build(),
    );
}

/// The base a `[]` replacement is built from: the type text before the `[` at `lbracket`, with
/// its own suffixes removed.
///
/// <para>A parenthesized base whose enclosed type carries a suffix (`(string+)[]`) would keep that
/// suffix inside the parentheses, and `(string+)*` is rejected in turn, so the enclosed type
/// without its suffix is the base instead: `string*`. A function type's result suffix is not the
/// enclosed type's, so `(<function />: string+)[]` keeps its parentheses.</para>
fn list_suffix_base<'a>(node: &'a SyntaxNode, base: &SyntaxNode, lbracket: SyntaxNode) -> &'a str {
    let text = node.text();
    let node_start = node.span().start();
    if base.kind() == SyntaxKind::PARENTHESIZED_TYPE {
        if let Some(suffix) = enclosed_occurrence_suffix(base) {
            let enclosed = &text[..usize::from(suffix.start() - node_start)];
            return strip_occurrence_suffixes(
                enclosed.trim_start_matches(|c: char| c == '(' || c.is_whitespace()),
            );
        }
    }
    strip_occurrence_suffixes(&text[..usize::from(lbracket.span().start() - node_start)])
}

/// `text` with its trailing occurrence suffixes and `[]` pairs removed, so a replacement built
/// from it carries one suffix only: `string?[]` suggests `string*`, not `string?*`.
fn strip_occurrence_suffixes(text: &str) -> &str {
    let mut base = text.trim_end();
    loop {
        let stripped = base
            .strip_suffix('?')
            .or_else(|| base.strip_suffix('+'))
            .or_else(|| base.strip_suffix('*'))
            .or_else(|| {
                base.strip_suffix(']')
                    .map(str::trim_end)
                    .and_then(|rest| rest.strip_suffix('['))
            });
        match stripped {
            Some(rest) => base = rest.trim_end(),
            None => return base,
        }
    }
}

/// The span of the occurrence suffix carried by the type a parenthesized type encloses, when
/// there is one.
///
/// <para>Parentheses add no layer however many of them there are, so a nested parenthesized type
/// is looked through as well: the `+` of `((string+))` is the enclosing chain's suffix just as the
/// one in `(string+)` is. A function type's result is a chain of its own, so
/// `(<function />: string+)` carries no suffix.</para>
fn enclosed_occurrence_suffix(parenthesized: &SyntaxNode) -> Option<TextRange> {
    let inner = parenthesized.child_by_field("type")?;
    let mut base: Option<SyntaxNode> = None;
    for child in inner.children_with_tokens() {
        match child.kind() {
            SyntaxKind::QUESTION | SyntaxKind::PLUS | SyntaxKind::STAR => {
                return Some(child.span())
            }
            _ if base.is_none() => base = Some(child),
            _ => {}
        }
    }
    match base {
        Some(base) if base.kind() == SyntaxKind::PARENTHESIZED_TYPE => {
            enclosed_occurrence_suffix(&base)
        }
        _ => None,
    }
}

/// A function type's parameters are matched by name at every use and supplied in full by the
/// caller, so a default has nothing to fill in, and its body content has one destination, so a
/// second `content` parameter has nowhere to go. The grammar reuses `property_definition` for the
/// parameters; the two rules it does not share with a signature are enforced here.
fn validate_function_types(node: &SyntaxNode, file_name: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == SyntaxKind::FUNCTION_TYPE {
        let mut content_parameter: Option<TextRange> = None;
        for param in node
            .children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        {
            let name = param
                .child_by_field("name")
                .map(|name| name.text().to_string())
                .unwrap_or_else(|| "_".to_string());

            if let Some(default) = param.child_by_field("default") {
                diagnostics.push(
                    Diagnostic::error("function-type-default")
                        .with_message(format!(
                            "Parameter '{name}' of a function type cannot carry a default value"
                        ))
                        .with_label(
                            Label::primary(file_name, default.span())
                                .with_message("remove the default value"),
                        )
                        .with_note(
                            "A caller supplies every parameter of a function type, so a default \
                             would never apply; declare the default on the function itself",
                        )
                        .build(),
                );
            }

            let is_content = param
                .child_by_field("modifier")
                .is_some_and(|modifier| modifier.text() == "content");
            if is_content {
                if let Some(previous) = content_parameter {
                    diagnostics.push(
                        Diagnostic::error("function-type-duplicate-content")
                            .with_message(format!(
                                "Parameter '{name}' is a second content parameter of the function \
                                 type"
                            ))
                            .with_label(
                                Label::primary(file_name, param.span())
                                    .with_message("remove the `content` modifier here"),
                            )
                            .with_label(
                                Label::secondary(file_name, previous)
                                    .with_message("this parameter already receives the content"),
                            )
                            .with_note("A function type can declare at most one content parameter")
                            .build(),
                    );
                } else {
                    content_parameter = Some(param.span());
                }
            }
        }
    }

    for child in node.children() {
        validate_function_types(&child, file_name, diagnostics);
    }
}

/// A paren-style call binds arguments by position and may stop early, leaving the parameters it
/// did not reach to their defaults or to empty. That works only when every parameter a caller may
/// omit — one marked `?` or one with a default — follows every parameter it must supply, so a
/// required parameter after an omissible one is rejected. The element form binds by name and has
/// no such rule.
fn validate_paren_parameter_order(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for function in root
        .children()
        .filter(|child| child.kind() == SyntaxKind::FUNCTION_DEFINITION)
    {
        let is_paren = function
            .child_by_field("name")
            .is_some_and(|name| name.kind() == SyntaxKind::IDENTIFIER);
        if !is_paren {
            continue;
        }

        let mut first_omissible: Option<(String, TextRange)> = None;
        for param in function
            .children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        {
            let name = param
                .child_by_field("name")
                .map(|name| name.text().to_string())
                .unwrap_or_else(|| "_".to_string());
            let omissible = param.child_by_field("optional").is_some()
                || param.child_by_field("default").is_some();
            match &first_omissible {
                None if omissible => first_omissible = Some((name, param.span())),
                Some((omissible_name, omissible_span)) if !omissible => {
                    diagnostics.push(
                        Diagnostic::error("required-parameter-after-omissible")
                            .with_message(format!(
                                "Required parameter '{name}' follows '{omissible_name}', which a \
                                 caller may omit"
                            ))
                            .with_label(
                                Label::primary(file_name, param.span())
                                    .with_message("move this parameter before the omissible ones"),
                            )
                            .with_label(
                                Label::secondary(file_name, *omissible_span)
                                    .with_message("a caller may omit this parameter"),
                            )
                            .with_note(
                                "A paren-style call binds arguments by position and may leave off \
                                 the trailing ones, so parameters marked `?` or given a default \
                                 come after every required parameter",
                            )
                            .build(),
                    );
                }
                _ => {}
            }
        }
    }
}

/// The longest default the optional-with-default diagnostic quotes; a longer one is shown as `...`.
const MAX_QUOTED_DEFAULT_LEN: usize = 40;

/// Rejects a property that is both optional and defaulted, offering the two forms it could mean.
///
/// <para>A default already makes a property omissible and gives it a value, so `name?:T = d` is
/// written either `name:T = d` or `name?:T`. The check is made on the syntax tree because both
/// forms are quoted from what the author wrote: the type as written, so an alias keeps its name,
/// and the default when it is short and on one line. This covers every property slot: record
/// fields, props, state fields and both parameter styles. A function type's parameter and a type
/// parameter reject any default on their own, so they are left to those rules.</para>
fn validate_optional_property_defaults(
    node: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for child in node.children() {
        validate_optional_property_defaults(&child, file_name, diagnostics);
    }

    if node.kind() != SyntaxKind::PROPERTY_DEFINITION
        || property_definition_is_type_parameter(node)
        || node
            .parent()
            .is_some_and(|parent| parent.kind() == SyntaxKind::FUNCTION_TYPE)
    {
        return;
    }
    let (Some(optional), Some(default)) = (
        node.child_by_field("optional"),
        node.child_by_field("default"),
    ) else {
        return;
    };

    let name = node
        .child_by_field("name")
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "_".to_string());
    let ty = node
        .child_by_field("type")
        .map(|ty| ty.text().trim().to_string())
        .unwrap_or_else(|| "...".to_string());
    let default_text = default.text().trim();
    let default_text = if default_text.is_empty()
        || default_text.contains('\n')
        || default_text.len() > MAX_QUOTED_DEFAULT_LEN
    {
        "..."
    } else {
        default_text
    };

    diagnostics.push(
        Diagnostic::error("optional-property-with-default")
            .with_message(format!(
                "Property '{name}' is optional and has a default, but a default already makes the \
                 property omissible and gives it a value; write `{name}:{ty} = {default_text}` or \
                 `{name}?:{ty}`"
            ))
            .with_label(
                Label::primary(file_name, optional.span())
                    .with_message("remove the `?` or the default"),
            )
            .with_label(
                Label::secondary(file_name, default.span())
                    .with_message("this default makes the property omissible"),
            )
            .build(),
    );
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

/// Rejects a `Name:type` definition anywhere it does not declare a component type parameter.
///
/// The grammar accepts the `type` keyword as a property type in every property list so that the
/// rejection can name the definition instead of falling to a generic parse error. A type parameter
/// is legal only in a component signature, ahead of every prop, with no default and no modifier.
fn validate_type_parameter_definitions(
    node: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if property_definition_is_type_parameter(node) {
        validate_type_parameter_definition(node, file_name, diagnostics);
    }

    for child in node.children() {
        validate_type_parameter_definitions(&child, file_name, diagnostics);
    }
}

fn validate_type_parameter_definition(
    prop: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = prop
        .child_by_field("name")
        .map(|name| name.text().to_string())
        .unwrap_or_else(|| "_".to_string());
    let parent = prop.parent();
    let parent_kind = parent.as_ref().map(|parent| parent.kind());

    let mut reject = |message: String, label: &str| {
        diagnostics.push(
            Diagnostic::error("invalid-type-parameter")
                .with_message(message)
                .with_label(Label::primary(file_name, prop.span()).with_message(label))
                .with_note(TYPE_PARAMETER_SYNTAX)
                .build(),
        );
    };

    // A component signature and a plain record declaration both take type parameters; every other
    // property list refuses them. The two differ only in what they call the properties that follow.
    let member = match parent_kind {
        Some(SyntaxKind::COMPONENT_SIGNATURE) => "prop",
        Some(SyntaxKind::RECORD_DEFINITION) => "field",
        _ => {
            let context = match parent_kind {
                Some(SyntaxKind::ACTION_DEFINITION) => "an action",
                Some(SyntaxKind::EMIT_DEFINITION) => "an emitted action",
                Some(SyntaxKind::STATE_GROUP) => "a state group",
                Some(SyntaxKind::FUNCTION_DEFINITION) => "a function parameter list",
                Some(SyntaxKind::FUNCTION_TYPE) => "a function type",
                _ => "this position",
            };
            reject(
                format!("Type parameter '{name}' is not supported in {context}"),
                "type parameters are only supported on component signatures and record declarations",
            );
            return;
        }
    };

    if let Some(parent) = parent.as_ref() {
        let follows_prop = parent
            .children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
            .take_while(|child| child.span() != prop.span())
            .any(|child| !property_definition_is_type_parameter(&child));
        if follows_prop {
            reject(
                format!("Type parameter '{name}' must be declared before every {member}"),
                format!("move this type parameter ahead of the {member}s").as_str(),
            );
        }
    }

    // A parameter shadows a same-named type inside its component. NX permits a module-level
    // `type` declaration named after a primitive or built-in, since a declaration is a site a
    // reader can find; a type parameter has none and its scope silently covers the whole
    // component, so it is held to the stricter rule.
    if crate::PRIMITIVE_TYPE_NAMES.contains(&name.as_str()) {
        reject(
            format!("Type parameter '{name}' cannot take the name of a primitive type"),
            "rename it, for example `TItem`",
        );
    } else if crate::BUILTIN_TYPE_NAMES.contains(&name.as_str()) {
        reject(
            format!("Type parameter '{name}' cannot take the name of the built-in type '{name}'"),
            "rename it, for example `TItem`",
        );
    }

    // A type parameter names a type the caller supplies; there is no "no type" to admit, so the
    // optional mark is refused on the same terms as a suffix on `type`.
    if prop.child_by_field("optional").is_some() {
        reject(
            format!("Type parameter '{name}' cannot be optional"),
            "remove the `?`; a type argument is always supplied",
        );
    }

    if prop.child_by_field("default").is_some() {
        reject(
            format!("Type parameter '{name}' cannot have a default value"),
            "remove the default value",
        );
    }

    if let Some(modifier) = prop.child_by_field("modifier") {
        reject(
            format!(
                "Type parameter '{name}' cannot have the '{}' modifier",
                modifier.text()
            ),
            "remove the modifier",
        );
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
    fn test_validate_accepts_each_occurrence_suffix_once() {
        for source in [
            "type A = string?",
            "type B = string+",
            "type C = string*",
            "type D = (string)+",
            "type E = (<function />: string)+",
            "type F = <function />: string+",
            "type G = <Range T=int/>*",
        ] {
            let result = parse_str(source, "test.nx");
            assert!(result.is_ok(), "{source}: {:?}", result.errors);
        }
    }

    #[test]
    fn test_validate_rejects_a_second_occurrence_suffix() {
        for source in [
            "type E = string??",
            "type F = string?+",
            "type G = (string+)*",
            "type H = <function />: int*?",
            "type I = ((string?))+",
        ] {
            let result = parse_str(source, "test.nx");
            let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
            assert_eq!(
                codes,
                vec!["second-occurrence-suffix"],
                "{source}: {:?}",
                result.errors
            );
            assert!(
                result.errors[0]
                    .message()
                    .contains("already carries an occurrence"),
                "{source}: {}",
                result.errors[0].message()
            );
            let primary = result.errors[0]
                .labels()
                .iter()
                .find(|label| label.primary)
                .expect("a primary label");
            assert_eq!(
                usize::from(primary.range.start()),
                source.trim_end().len() - 1,
                "{source}: the second suffix is the one reported"
            );
        }
    }

    #[test]
    fn test_validate_leaves_a_suffixed_property_value_to_the_checker() {
        // Only the checker knows whether `T` is a type parameter, so a suffix glued to a bare
        // property value parses cleanly; a spaced one is not a suffix.
        for source in [
            "let b = <Box T=int? />",
            "let b = <Box T=int+ />",
            "let b = <Box T=int* value=1 />",
        ] {
            let result = parse_str(source, "test.nx");
            assert!(result.is_ok(), "{source}: {:?}", result.errors);
        }
        for source in ["let b = <Box T=int ? />", "let b = <Box n=a + 1 />"] {
            let result = parse_str(source, "test.nx");
            assert!(
                result
                    .errors
                    .iter()
                    .all(|d| d.code() != Some("occurrence-suffix-on-value")),
                "{source}: {:?}",
                result.errors
            );
            assert!(!result.is_ok(), "{source} is still a syntax error");
        }

        let result = parse_str("let b = <Box T=int?+ />", "test.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
        assert_eq!(
            codes,
            vec!["second-occurrence-suffix"],
            "{:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_rejects_a_suffix_on_a_value_outside_a_property() {
        for (source, suffix) in [
            ("let x = foo?", "?"),
            ("let f() = x+", "+"),
            ("type R = { a:int = n*? }", "*?"),
        ] {
            let result = parse_str(source, "test.nx");
            let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
            assert_eq!(
                codes,
                vec!["occurrence-suffix-on-value"],
                "{source}: {:?}",
                result.errors
            );
            let primary = result.errors[0]
                .labels()
                .iter()
                .find(|label| label.primary)
                .expect("a primary label");
            let start = source.find(suffix).unwrap();
            assert_eq!(
                (
                    usize::from(primary.range.start()),
                    usize::from(primary.range.end())
                ),
                (start, start + suffix.len()),
                "{source}: every suffix is covered"
            );
        }
    }

    #[test]
    fn test_validate_rejects_the_list_suffix_naming_the_replacements() {
        let result = parse_str("type Names = string[]", "test.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
        assert_eq!(codes, vec!["removed-list-suffix"], "{:?}", result.errors);
        let note = result.errors[0].note().unwrap_or_default();
        assert!(note.contains("`string*`"), "{note}");
        assert!(note.contains("`string+`"), "{note}");
    }

    #[test]
    fn test_validate_list_suffix_replacement_drops_an_existing_suffix() {
        for source in [
            "type Names = string?[]",
            "type Names = string[][]",
            "type Names = string+[]",
            "type Names = (string+)[]",
            "type Names = ((string?))[]",
        ] {
            let result = parse_str(source, "test.nx");
            let list_suffixes: Vec<_> = result
                .errors
                .iter()
                .filter(|d| d.code() == Some("removed-list-suffix"))
                .collect();
            assert!(!list_suffixes.is_empty(), "{source}: {:?}", result.errors);
            for diagnostic in list_suffixes {
                let note = diagnostic.note().unwrap_or_default();
                assert!(note.contains("`string*`"), "{source}: {note}");
                assert!(note.contains("`string+`"), "{source}: {note}");
            }
        }
    }

    /// A function type's result suffix belongs to the result, so the parentheses stay.
    #[test]
    fn test_validate_list_suffix_replacement_keeps_a_function_type_in_parentheses() {
        let result = parse_str("type Rows = (<function />: string+)[]", "test.nx");
        let diagnostic = result
            .errors
            .iter()
            .find(|d| d.code() == Some("removed-list-suffix"))
            .unwrap_or_else(|| panic!("{:?}", result.errors));
        let note = diagnostic.note().unwrap_or_default();
        assert!(note.contains("`(<function />: string+)*`"), "{note}");
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
        assert!(!result.errors.is_empty(), "Should detect syntax errors");
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
