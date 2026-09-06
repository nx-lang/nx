//! Resolves an editor position to the NX construct it sits in.
//!
//! <para>Hover and completions both need to know what the cursor is on. Deciding that from the
//! text preceding the cursor on its own line makes the answer a function of layout: a tag written
//! across lines stops being a tag. Deciding it from the analyzed document makes the answer a
//! function of the construct, which is what the editor was asking about.</para>
//!
//! <para>The syntax tree names the context and the lowered module fills it in — a property name, a
//! type annotation, and a tag name are not expressions and have no `ExprId`, while a type and the
//! declaration a tag resolves to exist only in the analysis. See design D1.</para>

use nx_syntax::{SyntaxKind, SyntaxNode, SyntaxTree};
use rustc_hash::FxHashSet;
use text_size::TextRange;

/// What the cursor is on, as the syntax tree describes it.
///
/// <para>This is a syntactic answer. Which declaration a tag names, and what type an expression
/// has, are attached by the caller from the module analysis.</para>
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PositionContext {
    /// The name of a declaration, at the declaration itself.
    Declaration { name: String, span: TextRange },
    /// A name written where it refers to something declared elsewhere.
    Reference { name: String, span: TextRange },
    /// The name of an element's opening tag.
    ComponentTag { tag: String, span: TextRange },
    /// A property-name slot inside an opening tag, with what the tag already supplies.
    ///
    /// <para>`property` is the name already written there, and is absent at an empty slot, where
    /// the position is a place a name may go rather than a name.</para>
    PropertyName {
        tag: String,
        property: Option<String>,
        supplied: FxHashSet<String>,
        span: TextRange,
    },
    /// The value slot of `name=` inside an opening tag, where a bare name is accepted.
    PropertyValue {
        tag: String,
        property: String,
        span: TextRange,
    },
    /// A type annotation, whether or not a type has been written yet.
    ///
    /// <para>`name` is the type named at the position, and is absent where the annotation is still
    /// empty or the position is not on a name.</para>
    TypeAnnotation {
        name: Option<String>,
        span: TextRange,
    },
    /// Inside an expression, but not on a name.
    Expression { span: TextRange },
    /// No construct the language service can identify.
    Unresolved,
}

impl PositionContext {
    /// The source range the context covers, for a hover result.
    pub(crate) fn span(&self) -> Option<TextRange> {
        match self {
            Self::Declaration { span, .. }
            | Self::Reference { span, .. }
            | Self::ComponentTag { span, .. }
            | Self::PropertyName { span, .. }
            | Self::PropertyValue { span, .. }
            | Self::TypeAnnotation { span, .. }
            | Self::Expression { span } => Some(*span),
            Self::Unresolved => None,
        }
    }
}

/// Resolves the context of a byte offset in one parsed document.
pub(crate) fn resolve(tree: &SyntaxTree, offset: usize) -> PositionContext {
    let chain = ancestor_chain(tree.root(), offset);
    let Some(innermost) = chain.last().copied() else {
        return PositionContext::Unresolved;
    };

    if let Some(context) = element_context(&chain, offset) {
        return context;
    }
    if let Some(context) = type_annotation_context(&chain, offset) {
        return context;
    }
    if let Some(context) = declaration_context(&chain) {
        return context;
    }
    if let Some(context) = name_context(&chain) {
        return context;
    }
    if chain
        .iter()
        .any(|node| is_expression_container(node.kind()))
    {
        let span = literal_span(&chain).unwrap_or_else(|| innermost.span());
        return PositionContext::Expression { span };
    }

    PositionContext::Unresolved
}

/// The path from the root to the innermost node covering `offset`, tokens included.
///
/// <para>`SyntaxTree::node_at` cannot be used for this. It asks tree-sitter for the smallest
/// descendant spanning the empty range at the offset, which declines to enter a node the offset
/// merely touches and declines to enter a zero-width node at all. Both are exactly where an editor
/// asks: `&lt;Panel\n  mode=|\n/&gt;` puts the cursor at the end of `property_list` and inside a
/// zero-width `rhs_expression`, and `node_at` answers `element` for it. Reaching the cursor's own
/// node means descending with the boundaries included.</para>
fn ancestor_chain(root: SyntaxNode<'_>, offset: usize) -> Vec<SyntaxNode<'_>> {
    let mut chain = vec![root];
    let mut node = root;

    loop {
        let Some(child) = child_at(node, offset) else {
            break;
        };
        chain.push(child);
        node = child;
    }

    chain
}

/// The child of `node` the cursor is in, preferring the one that most nearly owns the offset.
///
/// <para>Several children can cover one offset once boundaries count: the node the cursor is
/// inside, the node that ends there, the node that starts there, and a zero-width node standing in
/// for text not typed yet. A cursor belongs to what it is inside, then to what was just typed
/// before it, then to what comes after — which is the order an editor's own selection follows.</para>
fn child_at(node: SyntaxNode<'_>, offset: usize) -> Option<SyntaxNode<'_>> {
    let mut interior = None;
    let mut empty = None;
    let mut ends_here = None;
    let mut starts_here = None;

    for child in node.children_with_tokens() {
        let start = child.start_byte();
        let end = child.end_byte();
        if start > offset || end < offset {
            continue;
        }

        if start < offset && offset < end {
            interior = Some(child);
        } else if start == end {
            empty = Some(child);
        } else if end == offset {
            ends_here = Some(child);
        } else {
            starts_here.get_or_insert(child);
        }
    }

    interior.or(empty).or(ends_here).or(starts_here)
}

/// Property-name, property-value, and tag-name positions inside an element's opening tag.
fn element_context(chain: &[SyntaxNode<'_>], offset: usize) -> Option<PositionContext> {
    let (element_index, element) = chain
        .iter()
        .enumerate()
        .rev()
        .find(|(_, node)| is_element(node.kind()))?;
    let tag = element_tag_name(*element)?;
    let name_node = element_name_node(*element)?;

    // The cursor is on the tag name itself.
    if name_node.start_byte() <= offset && offset <= name_node.end_byte() {
        return Some(PositionContext::ComponentTag {
            tag,
            span: name_node.span(),
        });
    }
    if offset < name_node.end_byte() {
        return None;
    }

    let below = &chain[element_index + 1..];

    // Inside `name=`: the value slot, but only where a bare name would be accepted. A quoted or
    // braced value is not a contextual-name position, so it is left to the expression rules.
    if let Some(property) = below
        .iter()
        .position(|node| node.kind() == SyntaxKind::PROPERTY_VALUE)
    {
        let property_value = below[property];
        let inside = &below[property + 1..];
        let in_value = inside
            .iter()
            .any(|node| node.kind() == SyntaxKind::RHS_EXPRESSION);

        if !in_value {
            // The cursor is on the property's own name. The name being edited is not a name
            // already supplied: counting it would offer every property except the one being
            // typed.
            let mut supplied = supplied_properties(*element);
            let property = property_value
                .child(0)
                .map(|name| name.text().trim().to_string())
                .filter(|name| !name.is_empty());
            if let Some(name) = property.as_deref() {
                supplied.remove(name);
            }
            return Some(PositionContext::PropertyName {
                tag,
                property,
                supplied,
                span: property_value.span(),
            });
        }

        let is_bare = !inside.iter().any(|node| {
            matches!(
                node.kind(),
                SyntaxKind::LITERAL
                    | SyntaxKind::STRING_LITERAL
                    | SyntaxKind::VALUES_BRACED_EXPRESSION
                    | SyntaxKind::ELEMENTS_BRACED_EXPRESSION
            )
        });
        let property_name = property_value
            .child(0)
            .map(|child| child.text().to_string());

        match (is_bare, property_name) {
            (true, Some(property)) => Some(PositionContext::PropertyValue {
                tag,
                property,
                span: TextRange::new((offset as u32).into(), (offset as u32).into()),
            }),
            // A quoted or braced value is not a contextual-name position, so it is left to the
            // expression rules.
            _ => None,
        }
    } else {
        // An empty slot in the opening tag is covered by no child at all — design D4. Anything
        // else below the element is a child expression, not a property-name position.
        if below
            .iter()
            .any(|node| is_element(node.kind()) || is_expression_container(node.kind()))
        {
            return None;
        }
        if offset > element_terminator_offset(*element) {
            return None;
        }

        Some(PositionContext::PropertyName {
            tag,
            property: None,
            supplied: supplied_properties(*element),
            span: TextRange::new((offset as u32).into(), (offset as u32).into()),
        })
    }
}

/// A type annotation, written or still empty.
fn type_annotation_context(chain: &[SyntaxNode<'_>], offset: usize) -> Option<PositionContext> {
    if let Some(node) = chain
        .iter()
        .rev()
        .find(|node| node.kind() == SyntaxKind::TYPE)
    {
        // `type` is also the keyword that opens a type declaration; only the annotation node has a
        // property definition or a value definition over it.
        if chain.iter().any(|ancestor| {
            matches!(
                ancestor.kind(),
                SyntaxKind::PROPERTY_DEFINITION
                    | SyntaxKind::PARAM
                    | SyntaxKind::VALUE_DEFINITION
                    | SyntaxKind::TYPE_DEFINITION
                    | SyntaxKind::RECORD_DEFINITION
            )
        }) {
            return Some(PositionContext::TypeAnnotation {
                name: name_at(chain),
                span: node.span(),
            });
        }
    }

    // Where error recovery produced no annotation node at all — `let value:| = 1` recovers as one
    // flat error span — the colon that introduces the annotation is still a token in the tree.
    // Reading it there rather than from the line keeps the answer layout-independent: the colon
    // counts wherever it was written. This applies only inside a recovered region; a well-formed
    // document is classified by its shape.
    if !chain.iter().any(|node| node.kind() == SyntaxKind::ERROR) {
        return None;
    }
    let (kind, _) = last_significant_token(*chain.first()?, offset)?;
    (kind == SyntaxKind::COLON).then(|| PositionContext::TypeAnnotation {
        // Nothing has been written after the colon yet, so there is no name to be on.
        name: None,
        span: TextRange::new((offset as u32).into(), (offset as u32).into()),
    })
}

/// The plain name the innermost node spells, where it spells one.
///
/// <para>Taking the annotation node's whole text would answer `Mode[]` for a cursor on `Mode`, and
/// would answer with the punctuation of a still-empty annotation. The innermost node is the name
/// the cursor is actually on, when it is on one at all.</para>
fn name_at(chain: &[SyntaxNode<'_>]) -> Option<String> {
    let text = chain.last()?.text().trim().to_string();
    let mut characters = text.chars();
    let leads = characters
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_');
    let follows = characters.all(|next| next.is_alphanumeric() || next == '_' || next == '.');
    (leads && follows).then_some(text)
}

/// The last token of `node`'s subtree that ends at or before `offset`.
fn last_significant_token(node: SyntaxNode<'_>, offset: usize) -> Option<(SyntaxKind, TextRange)> {
    let mut best: Option<(SyntaxKind, TextRange)> = None;

    fn walk(node: SyntaxNode<'_>, offset: usize, best: &mut Option<(SyntaxKind, TextRange)>) {
        let mut children = node.children_with_tokens().peekable();
        if children.peek().is_none() {
            if node.end_byte() <= offset && node.start_byte() < node.end_byte() {
                match best {
                    Some((_, span)) if usize::from(span.end()) >= node.end_byte() => {}
                    _ => *best = Some((node.kind(), node.span())),
                }
            }
            return;
        }
        for child in children {
            if child.start_byte() > offset {
                break;
            }
            walk(child, offset, best);
        }
    }

    walk(node, offset, &mut best);
    best
}

/// The cursor on the name of a top-level declaration.
fn declaration_context(chain: &[SyntaxNode<'_>]) -> Option<PositionContext> {
    let innermost = chain.last()?;
    if !matches!(
        innermost.kind(),
        SyntaxKind::IDENTIFIER | SyntaxKind::MARKUP_IDENTIFIER
    ) {
        return None;
    }

    let (index, definition) = chain
        .iter()
        .enumerate()
        .rev()
        .find(|(_, node)| is_definition(node.kind()))?;
    // Only the name, not everything under the declaration.
    let name = definition_name_node(*definition)?;
    if !name.span().contains_range(innermost.span()) {
        return None;
    }
    // A name nested under an element inside the declaration body is not the declaration's own.
    if chain[index + 1..]
        .iter()
        .any(|node| is_expression_container(node.kind()))
    {
        return None;
    }

    Some(PositionContext::Declaration {
        name: innermost.text().to_string(),
        span: innermost.span(),
    })
}

/// The cursor on a name written in expression position.
fn name_context(chain: &[SyntaxNode<'_>]) -> Option<PositionContext> {
    let innermost = chain.last()?;
    matches!(
        innermost.kind(),
        SyntaxKind::IDENTIFIER
            | SyntaxKind::MARKUP_IDENTIFIER
            | SyntaxKind::QUALIFIED_NAME
            | SyntaxKind::QUALIFIED_MARKUP_NAME
            | SyntaxKind::CONTEXTUAL_NAME
    )
    .then(|| PositionContext::Reference {
        name: innermost.text().to_string(),
        span: innermost.span(),
    })
}

/// Every property the opening tag already supplies, read from the whole tag rather than from one
/// line of it.
fn supplied_properties(element: SyntaxNode<'_>) -> FxHashSet<String> {
    let mut supplied = FxHashSet::default();
    for list in element
        .children()
        .filter(|child| child.kind() == SyntaxKind::PROPERTY_LIST)
    {
        collect_supplied(list, &mut supplied);
    }
    supplied
}

fn collect_supplied(node: SyntaxNode<'_>, supplied: &mut FxHashSet<String>) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::PROPERTY_VALUE | SyntaxKind::PROPERTY => {
                if let Some(name) = child.child(0) {
                    let text = name.text().trim();
                    if !text.is_empty() {
                        supplied.insert(text.to_string());
                    }
                }
            }
            // A property supplied inside a conditional is still supplied.
            _ => collect_supplied(child, supplied),
        }
    }
}

fn element_name_node(element: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
    element
        .child_by_field("name")
        .or_else(|| {
            element
                .child_by_field("open_tag")
                .and_then(|tag| tag.child_by_field("name"))
        })
        .or_else(|| {
            element
                .children()
                .find(|child| child.kind() == SyntaxKind::ELEMENT_NAME)
        })
}

fn element_tag_name(element: SyntaxNode<'_>) -> Option<String> {
    let name = element_name_node(element)?.text().trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Where the opening tag stops accepting properties.
///
/// <para>An unterminated tag has no terminator token, and the cursor is inside the tag wherever it
/// is, so the element's own end stands in.</para>
fn element_terminator_offset(element: SyntaxNode<'_>) -> usize {
    element
        .children_with_tokens()
        .find(|child| matches!(child.kind(), SyntaxKind::SLASH | SyntaxKind::GT))
        .map(|child| child.start_byte())
        .unwrap_or_else(|| element.end_byte())
}

/// The span of the whole literal the cursor is on, `-` included, or `None` where it is on no
/// literal.
///
/// <para>A literal is one expression however many tokens spell it, so the whole literal is what the
/// cursor is on. Reporting the innermost token instead would leave `-42` unanswerable: lowering
/// folds `-` into the literal and records the span of the written form, and a bound of `42`
/// excludes `-42`.</para>
///
/// <para>The fold does not depend on how `-42` parsed, so neither can this. Unbraced it is one
/// `signed_numeric_literal`; braced or nested it is a prefix `-` over `42`, and the cursor may be
/// on the sign rather than in the digits. The chain runs root to innermost, so the first node that
/// spells the literal and nothing else is the widest spelling of it.</para>
fn literal_span(chain: &[SyntaxNode<'_>]) -> Option<TextRange> {
    chain
        .iter()
        .find(|node| spans_a_literal(**node))
        .map(|node| node.span())
}

/// True where `node` spells one literal and nothing besides, a folded `-` included.
fn spans_a_literal(node: SyntaxNode<'_>) -> bool {
    is_literal(node.kind())
        || negates_a_literal(node)
        // The grammar wraps an operand in nodes that spell nothing of their own —
        // `value_expression`, `value_list_item_expression`, `rhs_expression`. Each spans exactly
        // what it wraps, which is what tells it from a node that adds something of its own.
        || node
            .children()
            .any(|child| child.span() == node.span() && spans_a_literal(child))
}

/// True for a prefix `-` applied to a literal, which lowering folds into the literal itself.
///
/// Recursive because the fold is: `- -42` folds twice, onto one literal spanning both signs.
fn negates_a_literal(node: SyntaxNode<'_>) -> bool {
    node.kind() == SyntaxKind::PREFIX_UNARY_EXPRESSION
        && node
            .children_with_tokens()
            .any(|child| child.kind() == SyntaxKind::MINUS)
        && node.children().any(spans_a_literal)
}

/// The kinds that spell one literal value.
///
/// <para>Only the kinds `grammar.js` actually produces. `unit_literal` is one it produces and this
/// is not: `()` lowers to no literal expression, so widening to it would bound a lookup that finds
/// nothing.</para>
fn is_literal(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::LITERAL
            | SyntaxKind::STRING_LITERAL
            | SyntaxKind::INT_LITERAL
            | SyntaxKind::REAL_LITERAL
            | SyntaxKind::HEX_LITERAL
            | SyntaxKind::BOOL_LITERAL
            | SyntaxKind::NULL_LITERAL
            | SyntaxKind::SIGNED_NUMERIC_LITERAL
    )
}

fn is_element(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::ELEMENT | SyntaxKind::SELF_CLOSING_ELEMENT)
}

fn is_definition(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::FUNCTION_DEFINITION
            | SyntaxKind::VALUE_DEFINITION
            | SyntaxKind::TYPE_DEFINITION
            | SyntaxKind::RECORD_DEFINITION
            | SyntaxKind::ACTION_DEFINITION
            | SyntaxKind::UNION_DEFINITION
            | SyntaxKind::COMPONENT_DEFINITION
    )
}

/// The name a declaration declares, as the declaration's own signature spells it.
fn definition_name_node(definition: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
    definition
        .child_by_field("name")
        .or_else(|| {
            definition
                .child_by_field("signature")
                .and_then(|signature| {
                    signature.child_by_field("name").or_else(|| {
                        signature
                            .children()
                            .find(|child| child.kind() == SyntaxKind::ELEMENT_NAME)
                    })
                })
        })
        .or_else(|| {
            definition.children().find(|child| {
                matches!(
                    child.kind(),
                    SyntaxKind::IDENTIFIER | SyntaxKind::ELEMENT_NAME
                )
            })
        })
}

fn is_expression_container(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::RHS_EXPRESSION
            | SyntaxKind::VALUE_EXPRESSION
            | SyntaxKind::VALUE_LIST_ITEM_EXPRESSION
            | SyntaxKind::VALUES_BRACED_EXPRESSION
            | SyntaxKind::ELEMENTS_BRACED_EXPRESSION
            | SyntaxKind::ELEMENTS_EXPRESSION
            | SyntaxKind::BINARY_EXPRESSION
            | SyntaxKind::IDENTIFIER_EXPRESSION
            | SyntaxKind::MEMBER_ACCESS_EXPRESSION
            | SyntaxKind::LITERAL
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_syntax::parse_str;

    /// Resolves the context at the `⟨cursor⟩` marker in a fixture.
    fn context_at(source: &str) -> PositionContext {
        let offset = source
            .find("⟨cursor⟩")
            .expect("fixture has a cursor marker");
        let stripped = format!(
            "{}{}",
            &source[..offset],
            &source[offset + "⟨cursor⟩".len()..]
        );
        let tree = parse_str(&stripped, "t.nx").tree.expect("tree");
        resolve(&tree, offset)
    }

    #[test]
    fn a_cursor_in_a_property_value_resolves_to_that_property() {
        let context = context_at("<Panel\n  mode=⟨cursor⟩\n/>\n");

        assert_eq!(
            context,
            PositionContext::PropertyValue {
                tag: "Panel".to_string(),
                property: "mode".to_string(),
                span: TextRange::new(14.into(), 14.into()),
            }
        );
    }

    #[test]
    fn a_cursor_on_a_property_name_resolves_to_a_property_name() {
        let PositionContext::PropertyName { tag, .. } =
            context_at("<Panel\n  mo⟨cursor⟩de=\"a\"\n/>\n")
        else {
            panic!("expected a property-name context");
        };

        assert_eq!(tag, "Panel");
    }

    #[test]
    fn a_cursor_in_a_type_annotation_resolves_to_a_type_annotation() {
        assert!(matches!(
            context_at("let <Panel\n  title:string\n  mode:⟨cursor⟩\n/> = <div />\n"),
            PositionContext::TypeAnnotation { .. }
        ));
    }

    #[test]
    fn a_cursor_on_a_name_in_a_body_resolves_to_a_reference() {
        let PositionContext::Reference { name, .. } =
            context_at("let add(count:int) = { co⟨cursor⟩unt + 1 }\n")
        else {
            panic!("expected a reference context");
        };

        assert_eq!(name, "count");
    }

    #[test]
    fn a_cursor_on_a_declaration_name_resolves_to_a_declaration() {
        let PositionContext::Declaration { name, .. } =
            context_at("let ad⟨cursor⟩d(count:int) = 1\n")
        else {
            panic!("expected a declaration context");
        };

        assert_eq!(name, "add");
    }

    #[test]
    fn a_cursor_on_a_tag_name_resolves_to_a_component_tag() {
        let PositionContext::ComponentTag { tag, .. } = context_at("<Pa⟨cursor⟩nel />\n")
        else {
            panic!("expected a component-tag context");
        };

        assert_eq!(tag, "Panel");
    }

    /// Design D4: the empty slot is covered by no child of the element, so a resolver that stopped
    /// at the innermost node would answer `element` and offer nothing.
    #[test]
    fn an_empty_slot_on_its_own_line_resolves_to_a_property_name() {
        let PositionContext::PropertyName { tag, supplied, .. } =
            context_at("<Panel\n  mode=\"a\"\n  ⟨cursor⟩\n/>\n")
        else {
            panic!("expected a property-name context");
        };

        assert_eq!(tag, "Panel");
        assert!(supplied.contains("mode"), "got: {supplied:?}");
    }

    /// Supplied properties come from the whole opening tag, not from the cursor's own line.
    #[test]
    fn supplied_properties_are_read_across_the_whole_opening_tag() {
        let PositionContext::PropertyName { supplied, .. } =
            context_at("<Panel\n  mode=\"a\"\n  ⟨cursor⟩\n  title=\"b\"\n/>\n")
        else {
            panic!("expected a property-name context");
        };

        assert!(supplied.contains("mode"), "got: {supplied:?}");
        assert!(supplied.contains("title"), "got: {supplied:?}");
    }

    #[test]
    fn a_position_in_open_text_resolves_to_nothing() {
        assert_eq!(context_at("⟨cursor⟩\n"), PositionContext::Unresolved);
    }
}
