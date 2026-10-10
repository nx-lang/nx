//! The source tree: every piece of one document as a typed node, with the declarations the nodes
//! refer to beside them.
//!
//! <para>Hover answers what one position is. A tool that renders or compares a whole document —
//! the NX viewer, a change view, an outline — needs that answer for every piece of the source at
//! once, from the same analysis, so it does not issue a query per token or disagree with the
//! editor by running an analysis of its own. This is that answer.</para>
//!
//! <para>The walk follows the syntax tree, because coverage is a property of the source text and
//! the syntax tree is what keeps every token, comments included. Each construct is then matched to
//! the expression lowering recorded at the same span, which is where its type and the declaration
//! it reaches are read from — the same lookup hover makes, so the two agree. A construct with no
//! lowered counterpart (an import, a comment, a region that did not parse) is still a node; it
//! simply carries no type.</para>
//!
//! <para>The shape is unstable: it is published for the viewer and the change view to build on,
//! and a later change commits to it.</para>

use crate::{
    is_unresolved_type, type_ref_display, DeclarationOrigin, DocumentScope, DocumentUri,
    DocumentVersion, EditorRange, LineIndex, ModuleAnalysis, NxIdentity, SnapshotError,
    WorkspaceDeclarations, WorkspaceSnapshot,
};
use nx_hir::{ast::Expr, Doc, ExprId, Item, LocalDefinitionId, RecordField, RecordKind};
use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use text_size::TextRange as ByteTextRange;

/// The source tree of one document: its nodes in source order and the declarations they refer
/// to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTree {
    /// Client URI.
    pub uri: DocumentUri,
    /// NX identity.
    pub identity: NxIdentity,
    /// Document version used for the result.
    pub version: Option<DocumentVersion>,
    /// Every piece of the document, in order of start offset, each parent before its children.
    pub nodes: Vec<SourceNode>,
    /// Each declaration a node refers to, listed once, from this module or any other.
    pub declarations: Vec<SourceDeclaration>,
}

/// One piece of the source: a construct a reader recognizes, with the range it covers.
///
/// <para>Every token of the document belongs to exactly one node, the smallest whose range holds
/// it, and the only tokens a node owns directly are its role's punctuation and keywords and what
/// the node carries as `name`, `value` or `textType`.</para>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceNode {
    /// What the construct is.
    pub role: SourceRole,
    /// The range the construct covers, children included.
    pub range: EditorRange,
    /// The index of the enclosing node; absent for a top-level node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    /// The node's path within the document, stable under edits elsewhere: the enclosing top-level
    /// declaration's name, then the names of the attributes, members, arms and slots on the way,
    /// and a position in brackets for an item of a sequence, of element content or of a branch.
    pub key: String,
    /// The name the construct declares or names: a declaration's, a member's, an attribute's, an
    /// element's tag, a case, a referenced name, an accessed member, a loop binding, an import's
    /// alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What the construct says as written: a literal, an operator's token, a type's spelling, a
    /// text run, a comment, an unparsed region, the alias of an imported name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The text type of a typed body, as `markdown` in `<Note:markdown>`, on the element and on
    /// each run of its text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_type: Option<String>,
    /// The construct's type spelled in NX, wherever the type checker gave it one.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// The index in the declaration table of the declaration the construct refers to; for a
    /// declaration, its own entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<u32>,
    /// Variations that matter to a renderer and not to the role.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<SourceFlag>,
}

/// What a node is: a construct a reader recognizes rather than a grammar production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceRole {
    /// An import statement.
    Import,
    /// A `let`, `type`, `action`, `component` or function declaration, with its name.
    Declaration,
    /// A parameter of a function or a property of a component.
    Parameter,
    /// A `state` field of a component.
    StateField,
    /// A field of a record, an action, a union case or an inline emitted action.
    Field,
    /// A case of a union.
    UnionCase,
    /// An entry of a component's `emits`.
    Emit,
    /// A type written in the source, with the type spelled in NX.
    TypeReference,
    /// An element construction, with the declaration of its kind when it resolves.
    Element,
    /// A `name=value` attribute, or the run of element content bound to a content property.
    Attribute,
    /// A run of element text.
    Text,
    /// An `@{…}` inside text.
    Embed,
    /// A string, number or boolean, with its value as written.
    Literal,
    /// `{}`.
    Empty,
    /// A union case, written bare or qualified, with the case name and the union's declaration.
    Case,
    /// A name that refers to a declaration, a parameter, a state field or a local binding.
    Reference,
    /// A member access, with the member name and the object as its child.
    Member,
    /// A prefix, binary or postfix operator, with its token as written and its operands.
    Operator,
    /// A call, with the callee and the arguments as children.
    Call,
    /// A braced list of values, with its items as children.
    Sequence,
    /// An `if`, with its test and branches, or its arms, as children.
    Condition,
    /// An `is` expression, with the scrutinee and its arms.
    Match,
    /// One arm of an `is` expression or of a condition list, with its patterns or test and its
    /// result.
    MatchArm,
    /// A `for`, with its bindings, its iterable and its body.
    Loop,
    /// A name a `for` binds: its item or its index.
    Binding,
    /// A comment, with its text.
    Comment,
    /// A `///` doc comment, with its text.
    DocComment,
    /// A region the parser could not read.
    Unparsed,
}

/// A variation of a construct that matters to a renderer and not to its role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceFlag {
    /// The value is written in braces, as `{true}`.
    Braced,
    /// The value is written in parentheses.
    Parenthesized,
    /// The attribute is element content bound to a content property, or the member is one.
    Content,
    /// The attribute binds a handler for an emitted action.
    Handler,
    /// The element is an update of the enclosing component's state.
    StateUpdate,
    /// The property is inherited from a base.
    Inherited,
    /// The member carries the `?` mark, or the member access is `?.`.
    Optional,
    /// The element's body is raw text.
    Raw,
    /// The declaration is `abstract`.
    Abstract,
    /// The component is `external`.
    External,
    /// The declaration is `export`.
    Export,
    /// The declaration is `private`.
    Private,
}

/// One declaration a node refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDeclaration {
    /// The identity of the declaring module.
    pub module: String,
    /// The declared name.
    pub name: String,
    /// What is declared.
    pub kind: SourceDeclarationKind,
    /// The whole declaration, when this document declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<EditorRange>,
    /// The doc comment, rendered as hover renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// A record's, action's, component's or function's properties in declaration order,
    /// inherited ones first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<SourceProperty>,
    /// A component's state fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<SourceProperty>,
    /// The table indices of the declaration's bases, nearest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bases: Vec<u32>,
    /// A union's cases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cases: Vec<SourceCase>,
    /// The type an alias names, or a value's type, spelled in NX.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// The range of a value declaration's value, when this document declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_range: Option<EditorRange>,
}

/// The kinds of declaration the table describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceDeclarationKind {
    /// `type Name = { … }`.
    Record,
    /// `action Name = { … }`.
    Action,
    /// `type Name = a | b`.
    Union,
    /// `type Name = Type`.
    Alias,
    /// `component <Name … />`.
    Component,
    /// `let name(…)` or `let <Name … />`.
    Function,
    /// `let name = …`.
    Value,
}

/// One property of a declaration, or one field of a union case or of a component's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProperty {
    /// The property's name.
    pub name: String,
    /// The declared type spelled in NX.
    #[serde(rename = "type")]
    pub ty: String,
    /// The source text of the default, when the property has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// The doc comment, rendered as hover renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// `optional`, `content` and `inherited`, where they apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<SourceFlag>,
}

/// One case of a union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCase {
    /// The case's name.
    pub name: String,
    /// The doc comment, rendered as hover renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// The case's payload fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<SourceProperty>,
}

impl WorkspaceSnapshot {
    /// Returns every piece of the requested document as a typed node, and the declarations the
    /// nodes refer to.
    ///
    /// <para>The answer is read from the analysis hover and diagnostics read, so a node's type and
    /// declaration are what hover reports inside it. A document that does not fully parse still
    /// has a tree; what did not parse is an `unparsed` node.</para>
    pub fn source_tree(&self, uri: &DocumentUri) -> Result<SourceTree, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let identity = document.identity.as_str();
        let mut builder = TreeBuilder::new(
            document.source(),
            identity,
            self.document_scope(identity),
            self.module_analysis(uri),
        );
        if let Some(tree) = parse_str(document.source(), identity).tree {
            builder.module(tree.root());
        }
        let (nodes, declarations) = builder.finish();

        Ok(SourceTree {
            uri: document.uri.clone(),
            identity: document.identity.clone(),
            version: document.version,
            nodes,
            declarations,
        })
    }
}

/// Builds the node list and the declaration table of one document in one walk.
struct TreeBuilder<'a> {
    identity: &'a str,
    index: LineIndex<'a>,
    scope: DocumentScope,
    analysis: Option<ModuleAnalysis>,
    /// The first expression lowering recorded at each span, in arena order: the one hover's
    /// innermost-expression search settles on when several share a span.
    exprs: FxHashMap<ByteTextRange, ExprId>,
    /// The identifier expressions that name a parameter, a `state` field or a local binding.
    locals: FxHashSet<ExprId>,
    /// The index of the item lowered from each top-level declaration, by the declaration's span.
    items: FxHashMap<ByteTextRange, usize>,
    nodes: Vec<SourceNode>,
    /// The key segment each node takes under its parent, joined into keys by `finish`.
    segments: Vec<String>,
    table: Vec<SourceDeclaration>,
    table_index: FxHashMap<DeclarationOrigin, u32>,
    /// The component whose declaration the walk is inside, whose own updates are state updates.
    component: Option<String>,
    /// The text type of the typed body the walk is inside.
    text_type: Option<String>,
    /// Whether the walk is inside a raw body.
    raw: bool,
}

/// One child of a body or a braced list: an item, or a comment or unparsed region between items.
enum Part<'t> {
    Item(SyntaxNode<'t>),
    Trivia(SyntaxNode<'t>),
}

impl<'a> TreeBuilder<'a> {
    fn new(
        source: &'a str,
        identity: &'a str,
        scope: DocumentScope,
        analysis: Option<ModuleAnalysis>,
    ) -> Self {
        let mut exprs = FxHashMap::default();
        let mut locals = FxHashSet::default();
        let mut items = FxHashMap::default();
        if let Some(analysis) = &analysis {
            let module = analysis.lowered_module();
            for (id, _) in module.exprs() {
                if let Some(span) = module.known_expr_span(id).filter(|span| !span.is_empty()) {
                    exprs.entry(span).or_insert(id);
                }
            }
            for (item_index, item) in module.items().iter().enumerate() {
                items.entry(item.span()).or_insert(item_index);
            }
            if let Some(prepared) = analysis.artifact().prepared_module.as_deref() {
                locals = nx_hir::local_references(prepared);
            }
        }

        Self {
            identity,
            index: LineIndex::new(source),
            scope,
            analysis,
            exprs,
            locals,
            items,
            nodes: Vec::new(),
            segments: Vec::new(),
            table: Vec::new(),
            table_index: FxHashMap::default(),
            component: None,
            text_type: None,
            raw: false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Nodes and keys
    // -----------------------------------------------------------------------------------------

    fn push(
        &mut self,
        role: SourceRole,
        span: ByteTextRange,
        parent: Option<usize>,
        segment: impl Into<String>,
    ) -> usize {
        self.nodes.push(SourceNode {
            role,
            range: self.index.range_from_bytes(span),
            parent: parent.map(|parent| parent as u32),
            key: String::new(),
            name: None,
            value: None,
            text_type: None,
            ty: None,
            declaration: None,
            flags: Vec::new(),
        });
        self.segments.push(segment.into());
        self.nodes.len() - 1
    }

    /// Pushes a node for an expression, with the type and the flags its position gives it.
    fn push_expression(
        &mut self,
        role: SourceRole,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) -> usize {
        let index = self.push(role, node.span(), parent, segment);
        self.nodes[index].ty = self.type_at(node.span());
        self.nodes[index].flags.extend_from_slice(flags);
        index
    }

    /// Joins the segments into keys and returns the nodes and the table.
    ///
    /// <para>A comment and an unparsed region name nothing, so each is keyed by the sibling it
    /// precedes, which an edit elsewhere does not move. A key two nodes would share — two
    /// declarations of one name in a document that does not compile — takes a `#2`, `#3` suffix,
    /// so no two nodes share one.</para>
    fn finish(mut self) -> (Vec<SourceNode>, Vec<SourceDeclaration>) {
        let mut siblings: FxHashMap<Option<u32>, Vec<usize>> = FxHashMap::default();
        for (index, node) in self.nodes.iter().enumerate() {
            siblings.entry(node.parent).or_default().push(index);
        }
        for children in siblings.values() {
            let mut pending: Vec<usize> = Vec::new();
            for &child in children.iter().chain(std::iter::once(&usize::MAX)) {
                let anchor = match self.nodes.get(child) {
                    Some(node) if is_anonymous(node.role) => {
                        pending.push(child);
                        continue;
                    }
                    Some(_) => self.segments[child].clone(),
                    None => "end".to_string(),
                };
                let mut counts: FxHashMap<SourceRole, usize> = FxHashMap::default();
                for anonymous in pending.drain(..) {
                    let role = self.nodes[anonymous].role;
                    let count = counts.entry(role).or_default();
                    self.segments[anonymous] = format!("{anchor}:{}[{count}]", role_name(role));
                    *count += 1;
                }
            }
        }

        let mut taken = FxHashSet::default();
        for index in 0..self.nodes.len() {
            let segment = &self.segments[index];
            let key = match self.nodes[index].parent {
                None => segment.clone(),
                Some(parent) if segment.starts_with('[') => {
                    format!("{}{segment}", self.nodes[parent as usize].key)
                }
                Some(parent) => format!("{}.{segment}", self.nodes[parent as usize].key),
            };
            let mut unique = key.clone();
            let mut ordinal = 2;
            while !taken.insert(unique.clone()) {
                unique = format!("{key}#{ordinal}");
                ordinal += 1;
            }
            self.nodes[index].key = unique;
        }

        (self.nodes, self.table)
    }

    // -----------------------------------------------------------------------------------------
    // Module, imports and declarations
    // -----------------------------------------------------------------------------------------

    fn module(&mut self, root: SyntaxNode<'_>) {
        // A document nothing of which parsed as a module is an error node at the root. Its
        // children are still read, so whatever did parse inside it keeps its nodes, and each run
        // of what did not is one unparsed region.
        let mut stray: Option<ByteTextRange> = None;
        for child in root.children_with_tokens() {
            if child.raw().is_missing() || child.span().is_empty() {
                continue;
            }
            let kind = child.kind();
            let error = child.raw().is_error();
            let declaration = matches!(
                kind,
                SyntaxKind::RECORD_DEFINITION
                    | SyntaxKind::ACTION_DEFINITION
                    | SyntaxKind::UNION_DEFINITION
                    | SyntaxKind::TYPE_DEFINITION
                    | SyntaxKind::VALUE_DEFINITION
                    | SyntaxKind::FUNCTION_DEFINITION
                    | SyntaxKind::COMPONENT_DEFINITION
            );
            let part = !error
                && (declaration
                    || kind.is_comment()
                    || matches!(kind, SyntaxKind::IMPORT_STATEMENT | SyntaxKind::ELEMENT));
            if !part && !(error && root.kind() != SyntaxKind::ERROR) {
                stray = Some(match stray {
                    Some(span) => span.cover(child.span()),
                    None => child.span(),
                });
                continue;
            }
            if let Some(span) = stray.take() {
                self.unparsed_span(span, None);
            }
            match kind {
                SyntaxKind::IMPORT_STATEMENT if !error => self.import(child),
                SyntaxKind::ELEMENT if !error => self.element(child, None, "root".to_string(), &[]),
                _ if declaration && !error => self.declaration(child),
                _ => {
                    self.trivia(child, None);
                }
            }
        }
        if let Some(span) = stray {
            self.unparsed_span(span, None);
        }
    }

    fn import(&mut self, node: SyntaxNode<'_>) {
        let path = descendant(node, SyntaxKind::LIBRARY_PATH);
        let segment = format!(
            "import {}",
            path.map(|path| path.text()).unwrap_or_default()
        );
        let index = self.push(SourceRole::Import, node.span(), None, segment);
        self.import_parts(node, index);
    }

    fn import_parts(&mut self, node: SyntaxNode<'_>, import: usize) {
        for (field, child) in fields(node) {
            if self.trivia(child, Some(import)) || !child.raw().is_named() {
                continue;
            }
            match (field, child.kind()) {
                (_, SyntaxKind::LIBRARY_PATH) => {
                    let literal =
                        self.push(SourceRole::Literal, child.span(), Some(import), "path");
                    self.nodes[literal].value = Some(child.text().to_string());
                    self.nested_trivia(child, literal);
                }
                (Some("alias"), _) => self.nodes[import].name = Some(child.text().to_string()),
                (_, SyntaxKind::SELECTIVE_IMPORT) => {
                    let name = field_text(child, "name").unwrap_or_default();
                    let alias = child.child_by_field("alias").map(spell);
                    let reference =
                        self.push(SourceRole::Reference, child.span(), Some(import), &*name);
                    let visible = alias.as_deref().unwrap_or(&name);
                    self.nodes[reference].declaration = self.visible_declaration(visible);
                    self.nodes[reference].name = Some(name);
                    self.nodes[reference].value = alias;
                    self.nested_trivia(child, reference);
                }
                _ => self.import_parts(child, import),
            }
        }
    }

    fn declaration(&mut self, node: SyntaxNode<'_>) {
        let name_node = node.child_by_field("name").or_else(|| {
            node.child_by_field("signature")
                .and_then(|signature| signature.child_by_field("name"))
        });
        let name = name_node.map(spell).unwrap_or_default();
        let index = self.push(SourceRole::Declaration, node.span(), None, &*name);

        let mut flags = Vec::new();
        if let Some(visibility) = node.child_by_field("visibility") {
            flags.push(match visibility.text() {
                "private" => SourceFlag::Private,
                _ => SourceFlag::Export,
            });
        }
        if node.child_by_field("abstract").is_some() {
            flags.push(SourceFlag::Abstract);
        }
        if node.child_by_field("external").is_some() {
            flags.push(SourceFlag::External);
        }
        self.nodes[index].flags = flags;
        self.nodes[index].declaration = self.items.get(&node.span()).copied().and_then(|item| {
            let origin = (
                self.identity.to_string(),
                LocalDefinitionId::new(item as u32),
            );
            self.entry(&origin)
        });
        self.nodes[index].name = Some(name.clone());

        let member = match node.kind() {
            SyntaxKind::RECORD_DEFINITION
            | SyntaxKind::ACTION_DEFINITION
            | SyntaxKind::UNION_DEFINITION => SourceRole::Field,
            _ => SourceRole::Parameter,
        };
        let enclosing = match node.kind() {
            SyntaxKind::COMPONENT_DEFINITION => self.component.replace(name),
            _ => self.component.take(),
        };
        self.declaration_parts(node, index, member);
        self.component = enclosing;
    }

    /// The members, types and values of a declaration, read through the wrappers that group them
    /// (a component's signature and body, its `emits` and `state`, a union's case list).
    fn declaration_parts(&mut self, node: SyntaxNode<'_>, owner: usize, member: SourceRole) {
        for (field, child) in fields(node) {
            if self.trivia(child, Some(owner)) || !child.raw().is_named() {
                continue;
            }
            match (field, child.kind()) {
                (Some("name" | "visibility"), _) => {}
                (_, SyntaxKind::PROPERTY_DEFINITION) => self.member(child, owner, member),
                (_, SyntaxKind::UNION_CASE) => self.union_case(child, owner),
                (_, SyntaxKind::EMIT_DEFINITION) => self.emit_definition(child, owner),
                (_, SyntaxKind::EMIT_REFERENCE) => self.emit_reference(child, owner),
                (_, SyntaxKind::STATE_GROUP) => {
                    self.declaration_parts(child, owner, SourceRole::StateField)
                }
                (
                    _,
                    SyntaxKind::COMPONENT_SIGNATURE
                    | SyntaxKind::COMPONENT_BODY
                    | SyntaxKind::EMITS_GROUP
                    | SyntaxKind::UNION_CASE_LIST,
                ) => self.declaration_parts(child, owner, member),
                (Some("type" | "return_type"), _) => {
                    self.type_reference(child, Some(owner), "type");
                }
                (Some("base"), _) => {
                    self.type_reference(child, Some(owner), "extends");
                }
                (Some(slot @ ("value" | "body")), _) => {
                    self.expression(child, Some(owner), slot.to_string(), &[])
                }
                _ => {}
            }
        }
    }

    fn member(&mut self, node: SyntaxNode<'_>, owner: usize, role: SourceRole) {
        let name = field_text(node, "name").unwrap_or_default();
        let index = self.push(role, node.span(), Some(owner), &*name);
        self.nodes[index].name = Some(name);
        if node.child_by_field("optional").is_some() {
            self.nodes[index].flags.push(SourceFlag::Optional);
        }
        if node
            .child_by_field("modifier")
            .is_some_and(|modifier| modifier.text() == "content")
        {
            self.nodes[index].flags.push(SourceFlag::Content);
        }
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) {
                continue;
            }
            match field {
                // `type` itself, the type of a type parameter, is a type as much as `int` is.
                Some("type") => {
                    self.type_reference(child, Some(index), "type");
                }
                Some("default") => self.expression(child, Some(index), "default".to_string(), &[]),
                _ => {}
            }
        }
    }

    fn union_case(&mut self, node: SyntaxNode<'_>, owner: usize) {
        let name = field_text(node, "name").unwrap_or_default();
        let index = self.push(SourceRole::UnionCase, node.span(), Some(owner), &*name);
        self.nodes[index].name = Some(name);
        self.declaration_parts(node, index, SourceRole::Field);
    }

    fn emit_definition(&mut self, node: SyntaxNode<'_>, owner: usize) {
        let name = field_text(node, "name").unwrap_or_default();
        let index = self.push(SourceRole::Emit, node.span(), Some(owner), &*name);
        self.nodes[index].name = Some(name);
        self.declaration_parts(node, index, SourceRole::Field);
    }

    fn emit_reference(&mut self, node: SyntaxNode<'_>, owner: usize) {
        let name = node.child_by_field("name").map(spell).unwrap_or_default();
        let index = self.push(SourceRole::Emit, node.span(), Some(owner), &*name);
        self.nodes[index].declaration = self.type_declaration(&name);
        self.nodes[index].name = Some(name);
        self.nested_trivia(node, index);
    }

    /// A type as written: one node, whatever its structure, spelled the way the source spells it.
    fn type_reference(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: &str,
    ) -> usize {
        let index = self.push(SourceRole::TypeReference, node.span(), parent, segment);
        self.nodes[index].value = Some(spell(node));
        self.nodes[index].declaration =
            type_name(node).and_then(|name| self.type_declaration(&name));
        self.nested_trivia(node, index);
        index
    }

    // -----------------------------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------------------------

    fn expression(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        if node.raw().is_error() {
            self.unparsed(node, parent);
            return;
        }
        match node.kind() {
            SyntaxKind::LITERAL | SyntaxKind::SIGNED_NUMERIC_LITERAL | SyntaxKind::UNIT_LITERAL => {
                let index = self.push_expression(SourceRole::Literal, node, parent, segment, flags);
                self.nodes[index].value = Some(node.text().to_string());
            }
            SyntaxKind::IDENTIFIER_EXPRESSION => {
                let name = node.text().to_string();
                let index =
                    self.push_expression(SourceRole::Reference, node, parent, segment, flags);
                let local = self
                    .expr_at(node.span())
                    .is_some_and(|id| self.locals.contains(&id));
                if !local {
                    self.nodes[index].declaration = self.visible_declaration(&name);
                }
                self.nodes[index].name = Some(name);
            }
            SyntaxKind::CONTEXTUAL_NAME => {
                let name = field_kind_text(node, SyntaxKind::IDENTIFIER).unwrap_or_default();
                let index = self.push_expression(SourceRole::Case, node, parent, segment, flags);
                self.nodes[index].declaration = self.case_declaration(node.span());
                self.nodes[index].name = Some(name);
            }
            SyntaxKind::VALUES_BRACED_EXPRESSION | SyntaxKind::ELEMENTS_BRACED_EXPRESSION => {
                self.braced(node, parent, segment, flags)
            }
            SyntaxKind::PARENTHESIZED_EXPRESSION => {
                let mut flags = flags.to_vec();
                flags.push(SourceFlag::Parenthesized);
                self.wrapper(node, parent, segment, &flags);
            }
            // A `?` the parser invented to recover is not written, so there is no operator.
            SyntaxKind::EXISTS_EXPRESSION
                if node
                    .children_with_tokens()
                    .any(|child| child.raw().is_missing()) =>
            {
                self.wrapper(node, parent, segment, flags)
            }
            SyntaxKind::BINARY_EXPRESSION
            | SyntaxKind::PREFIX_UNARY_EXPRESSION
            | SyntaxKind::EXISTS_EXPRESSION => self.operator(node, parent, segment, flags),
            SyntaxKind::CALL_EXPRESSION => self.call(node, parent, segment, flags),
            SyntaxKind::MEMBER_ACCESS_EXPRESSION | SyntaxKind::OPTIONAL_MEMBER_EXPRESSION => {
                self.member_access(node, parent, segment, flags)
            }
            SyntaxKind::ELEMENT | SyntaxKind::TEXT_CHILD_ELEMENT => {
                self.element(node, parent, segment, flags)
            }
            SyntaxKind::VALUE_IF_SIMPLE_EXPRESSION | SyntaxKind::ELEMENTS_IF_SIMPLE_EXPRESSION => {
                self.condition(node, parent, segment, flags)
            }
            SyntaxKind::VALUE_IF_CONDITION_LIST_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_CONDITION_LIST_EXPRESSION => {
                self.condition_list(node, parent, segment, flags)
            }
            SyntaxKind::VALUE_IF_MATCH_EXPRESSION | SyntaxKind::ELEMENTS_IF_MATCH_EXPRESSION => {
                self.matching(node, parent, segment, flags)
            }
            SyntaxKind::VALUE_FOR_EXPRESSION | SyntaxKind::ELEMENTS_FOR_EXPRESSION => {
                self.for_loop(node, parent, segment, flags)
            }
            SyntaxKind::EMBED_BRACED_EXPRESSION => {
                let index = self.push_expression(SourceRole::Embed, node, parent, segment, flags);
                self.items(node, index, "");
            }
            SyntaxKind::TEXT_RUN | SyntaxKind::EMBED_TEXT_RUN | SyntaxKind::RAW_TEXT_RUN => {
                let index = self.push_expression(SourceRole::Text, node, parent, segment, flags);
                self.nodes[index].value = Some(node.text().to_string());
                self.nodes[index].text_type = self.text_type.clone();
                if self.raw {
                    self.nodes[index].flags.push(SourceFlag::Raw);
                }
            }
            _ => self.wrapper(node, parent, segment, flags),
        }
    }

    /// A node that only wraps the construct inside it: the construct takes its place.
    fn wrapper(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        for (_, child) in fields(node) {
            if self.trivia(child, parent) || !child.raw().is_named() {
                continue;
            }
            self.expression(child, parent, segment.clone(), flags);
        }
    }

    /// `{}` is the empty value, `{x}` is `x` written in braces, and two or more items are a
    /// sequence.
    fn braced(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let parts = parts(node);
        let items = parts
            .iter()
            .filter(|part| matches!(part, Part::Item(_)))
            .count();
        match items {
            0 => {
                let index = self.push_expression(SourceRole::Empty, node, parent, segment, flags);
                self.place(parts, index, "");
            }
            1 => {
                let mut flags = flags.to_vec();
                flags.push(SourceFlag::Braced);
                for part in parts {
                    match part {
                        Part::Item(item) => self.expression(item, parent, segment.clone(), &flags),
                        Part::Trivia(trivia) => {
                            self.trivia(trivia, parent);
                        }
                    }
                }
            }
            _ => {
                let index =
                    self.push_expression(SourceRole::Sequence, node, parent, segment, flags);
                self.place(parts, index, "");
            }
        }
    }

    /// The items of a list, keyed by position after `prefix`, under `parent`.
    fn items(&mut self, node: SyntaxNode<'_>, parent: usize, prefix: &str) {
        let parts = parts(node);
        self.place(parts, parent, prefix);
    }

    fn place(&mut self, parts: Vec<Part<'_>>, parent: usize, prefix: &str) {
        let mut position = 0;
        for part in parts {
            match part {
                Part::Item(item) => {
                    self.expression(item, Some(parent), format!("{prefix}[{position}]"), &[]);
                    position += 1;
                }
                Part::Trivia(trivia) => {
                    self.trivia(trivia, Some(parent));
                }
            }
        }
    }

    /// The items of one branch: the items of its braces, or the one expression it is.
    fn branch(&mut self, node: SyntaxNode<'_>, parent: usize, prefix: &str) {
        match node.kind() {
            SyntaxKind::VALUES_BRACED_EXPRESSION | SyntaxKind::ELEMENTS_BRACED_EXPRESSION => {
                self.items(node, parent, prefix)
            }
            _ => self.expression(node, Some(parent), format!("{prefix}[0]"), &[]),
        }
    }

    fn operator(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let token = node
            .child_by_field("operator")
            .map(|operator| operator.text().to_string())
            .unwrap_or_else(|| "?".to_string());
        let index = self.push_expression(SourceRole::Operator, node, parent, segment, flags);
        self.nodes[index].value = Some(token);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            if let Some(slot @ ("left" | "right" | "operand")) = field {
                self.expression(child, Some(index), slot.to_string(), &[]);
            }
        }
    }

    fn call(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let index = self.push_expression(SourceRole::Call, node, parent, segment, flags);
        let mut argument = 0;
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            if field == Some("callee") {
                self.expression(child, Some(index), "callee".to_string(), &[]);
            } else {
                self.expression(child, Some(index), format!("args[{argument}]"), &[]);
                argument += 1;
            }
        }
    }

    /// `x.m` is a member access, except where it names a case of a union: `Role.admin`.
    fn member_access(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let member = field_text(node, "member").unwrap_or_default();
        let case = node.kind() == SyntaxKind::MEMBER_ACCESS_EXPRESSION
            && self.case_declaration(node.span()).is_some();
        let (role, object) = if case {
            (SourceRole::Case, "union")
        } else {
            (SourceRole::Member, "object")
        };
        let index = self.push_expression(role, node, parent, segment, flags);
        if case {
            self.nodes[index].declaration = self.case_declaration(node.span());
        }
        if node.kind() == SyntaxKind::OPTIONAL_MEMBER_EXPRESSION {
            self.nodes[index].flags.push(SourceFlag::Optional);
        }
        self.nodes[index].name = Some(member);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            if field == Some("target") {
                self.expression(child, Some(index), object.to_string(), &[]);
            }
        }
    }

    fn condition(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let index = self.push_expression(SourceRole::Condition, node, parent, segment, flags);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match field {
                Some("condition") => self.expression(child, Some(index), "test".to_string(), &[]),
                Some(slot @ ("then" | "else")) => self.branch(child, index, slot),
                _ => {}
            }
        }
    }

    fn condition_list(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let index = self.push_expression(SourceRole::Condition, node, parent, segment, flags);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match (field, child.kind()) {
                (_, SyntaxKind::VALUE_IF_CONDITION_ARM | SyntaxKind::ELEMENTS_IF_CONDITION_ARM) => {
                    let test = child.child_by_field("condition").map(spell);
                    let arm = self.push(
                        SourceRole::MatchArm,
                        child.span(),
                        Some(index),
                        format!("when {}", test.unwrap_or_default()),
                    );
                    for (field, part) in fields(child) {
                        if self.trivia(part, Some(arm)) || !part.raw().is_named() {
                            continue;
                        }
                        match field {
                            Some("condition") => {
                                self.expression(part, Some(arm), "test".to_string(), &[])
                            }
                            Some("body") => self.branch(part, arm, ""),
                            _ => {}
                        }
                    }
                }
                (Some("else"), _) => self.branch(child, index, "else"),
                _ => {}
            }
        }
    }

    fn matching(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let index = self.push_expression(SourceRole::Match, node, parent, segment, flags);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match (field, child.kind()) {
                (Some("scrutinee"), _) => {
                    self.expression(child, Some(index), "subject".to_string(), &[])
                }
                (_, SyntaxKind::VALUE_IF_MATCH_ARM | SyntaxKind::ELEMENTS_IF_MATCH_ARM) => {
                    let arm = self.match_arm(child, index);
                    for (field, part) in fields(child) {
                        if field == Some("body") && !self.trivia(part, Some(arm)) {
                            self.branch(part, arm, "");
                        }
                    }
                }
                (Some("else"), _) => self.branch(child, index, "else"),
                _ => {}
            }
        }
    }

    /// An arm of an `is` expression with its patterns; the caller places its result. The arm is
    /// named by its patterns, which is what tells it from its siblings.
    fn match_arm(&mut self, node: SyntaxNode<'_>, parent: usize) -> usize {
        let patterns = node
            .children()
            .filter(|child| child.kind() == SyntaxKind::PATTERN)
            .collect::<Vec<_>>();
        let name = patterns
            .iter()
            .map(|pattern| spell(*pattern))
            .collect::<Vec<_>>()
            .join(", ");
        let arm = self.push(
            SourceRole::MatchArm,
            node.span(),
            Some(parent),
            format!("is {name}"),
        );
        let mut position = 0;
        for (_, child) in fields(node) {
            if self.trivia(child, Some(arm)) {
                continue;
            }
            if child.kind() == SyntaxKind::PATTERN {
                self.pattern(child, arm, format!("pattern[{position}]"));
                position += 1;
            }
        }
        arm
    }

    fn pattern(&mut self, node: SyntaxNode<'_>, parent: usize, segment: String) {
        let mut written = false;
        for (_, child) in fields(node) {
            if self.trivia(child, Some(parent)) || !child.raw().is_named() {
                continue;
            }
            written = true;
            match child.kind() {
                SyntaxKind::QUALIFIED_NAME => self.qualified_case(child, parent, segment.clone()),
                _ => self.expression(child, Some(parent), segment.clone(), &[]),
            }
        }
        if !written {
            self.push_expression(SourceRole::Empty, node, Some(parent), segment, &[]);
        }
    }

    /// A name in a pattern: a case, bare or qualified, or else a reference. A qualified case's
    /// qualifier is a reference to the union, as it is in a value.
    fn qualified_case(&mut self, node: SyntaxNode<'_>, parent: usize, segment: String) {
        let names = node
            .children()
            .filter(|child| child.kind() == SyntaxKind::IDENTIFIER)
            .collect::<Vec<_>>();
        let Some((last, qualifier)) = names.split_last() else {
            return;
        };
        let declaration = self.case_declaration(node.span());
        let role = if declaration.is_some() || qualifier.is_empty() {
            SourceRole::Case
        } else {
            SourceRole::Reference
        };
        let index = self.push_expression(role, node, Some(parent), segment, &[]);
        self.nodes[index].name = Some(match role {
            SourceRole::Case => last.text().to_string(),
            _ => spell(node),
        });
        if role == SourceRole::Reference {
            self.nodes[index].declaration = self.visible_declaration(&spell(node));
            return;
        }
        self.nodes[index].declaration = declaration;
        if let (Some(first), Some(end)) = (qualifier.first(), qualifier.last()) {
            let span = first.span().cover(end.span());
            let name = self.index_text(span);
            let reference = self.push(SourceRole::Reference, span, Some(index), "union");
            self.nodes[reference].declaration = self.visible_declaration(&name);
            self.nodes[reference].name = Some(name);
        }
        self.nested_trivia(node, index);
    }

    fn for_loop(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let index = self.push_expression(SourceRole::Loop, node, parent, segment, flags);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match field {
                Some("item" | "index") => {
                    let name = child.text().to_string();
                    let binding = self.push(SourceRole::Binding, child.span(), Some(index), &*name);
                    self.nodes[binding].name = Some(name);
                }
                Some("iterable") => self.expression(child, Some(index), "in".to_string(), &[]),
                Some("body") => self.branch(child, index, ""),
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Elements and attributes
    // -----------------------------------------------------------------------------------------

    fn element(
        &mut self,
        node: SyntaxNode<'_>,
        parent: Option<usize>,
        segment: String,
        flags: &[SourceFlag],
    ) {
        let tag = node.child_by_field("name").map(spell).unwrap_or_default();
        let index = self.push_expression(SourceRole::Element, node, parent, segment, flags);
        let text_type = field_text(node, "text_type");
        let raw = node
            .children_with_tokens()
            .any(|child| !child.raw().is_named() && child.text() == "raw");
        let origin = self.element_origin(&tag, node.span());
        let declaration = origin.as_ref().and_then(|origin| self.entry(origin));
        if self.is_state_update(origin.as_ref()) {
            self.nodes[index].flags.push(SourceFlag::StateUpdate);
        }
        if raw {
            self.nodes[index].flags.push(SourceFlag::Raw);
        }
        self.nodes[index].declaration = declaration;
        self.nodes[index].name = Some(tag);
        self.nodes[index].text_type = text_type.clone();

        let content_property = declaration.and_then(|declaration| {
            self.table[declaration as usize]
                .properties
                .iter()
                .find(|property| property.flags.contains(&SourceFlag::Content))
                .map(|property| property.name.clone())
        });
        let type_parameters = origin
            .as_ref()
            .map(|origin| self.type_parameters(origin))
            .unwrap_or_default();

        let outer_text_type = std::mem::replace(&mut self.text_type, text_type);
        let outer_raw = std::mem::replace(&mut self.raw, raw);
        let mut conditions = 0;
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match field {
                Some("properties") => {
                    self.properties(child, index, "", &type_parameters, &mut conditions)
                }
                Some("content") => self.content(child, index, content_property.as_deref()),
                // A closing tag that names another element is an error the parser lets through.
                Some("close_name")
                    if spell(child) != self.nodes[index].name.as_deref().unwrap_or("") =>
                {
                    self.unparsed(child, Some(index))
                }
                _ => {}
            }
        }
        self.text_type = outer_text_type;
        self.raw = outer_raw;
    }

    /// The body of an element: its items, under an attribute named for the content property they
    /// bind to when the element's declaration has one.
    fn content(&mut self, node: SyntaxNode<'_>, element: usize, property: Option<&str>) {
        let container = match property {
            Some(property) => {
                let attribute =
                    self.push(SourceRole::Attribute, node.span(), Some(element), property);
                self.nodes[attribute].name = Some(property.to_string());
                self.nodes[attribute].flags.push(SourceFlag::Content);
                attribute
            }
            None => element,
        };
        match node.kind() {
            SyntaxKind::MIXED_CONTENT
            | SyntaxKind::TEXT_CONTENT
            | SyntaxKind::EMBED_TEXT_CONTENT => self.items(node, container, ""),
            _ => self.expression(node, Some(container), "[0]".to_string(), &[]),
        }
    }

    /// The attributes of a property list, and the conditions and matches that choose among them.
    fn properties(
        &mut self,
        node: SyntaxNode<'_>,
        owner: usize,
        prefix: &str,
        type_parameters: &[String],
        conditions: &mut usize,
    ) {
        for (_, child) in fields(node) {
            if self.trivia(child, Some(owner)) || !child.raw().is_named() {
                continue;
            }
            match child.kind() {
                SyntaxKind::PROPERTY_VALUE => self.attribute(child, owner, prefix, type_parameters),
                SyntaxKind::PROPERTY_LIST_IF_EXPRESSION => {
                    let segment = format!("{prefix}if[{conditions}]");
                    *conditions += 1;
                    for inner in child.children() {
                        self.property_condition(inner, owner, segment.clone(), type_parameters);
                    }
                    for (_, trivia) in fields(child) {
                        self.trivia(trivia, Some(owner));
                    }
                }
                _ => self.properties(child, owner, prefix, type_parameters, conditions),
            }
        }
    }

    fn attribute(
        &mut self,
        node: SyntaxNode<'_>,
        owner: usize,
        prefix: &str,
        type_parameters: &[String],
    ) {
        let name = node.child_by_field("name").map(spell).unwrap_or_default();
        let index = self.push(
            SourceRole::Attribute,
            node.span(),
            Some(owner),
            format!("{prefix}{name}"),
        );
        if self
            .expr_at(node.span())
            .zip(self.analysis.as_ref())
            .is_some_and(|(id, analysis)| {
                matches!(
                    analysis.lowered_module().expr(id),
                    Expr::ActionHandler { .. }
                )
            })
        {
            self.nodes[index].flags.push(SourceFlag::Handler);
        }
        let type_argument = type_parameters.contains(&name);
        self.nodes[index].name = Some(name);
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            if field != Some("value") {
                continue;
            }
            // A bare name bound to a type parameter is a type argument, `T=int`, not a case.
            let written = child.children().next();
            match written {
                Some(name) if type_argument && name.kind() == SyntaxKind::CONTEXTUAL_NAME => {
                    self.type_reference(name, Some(index), "value");
                }
                _ => self.expression(child, Some(index), "value".to_string(), &[]),
            }
        }
    }

    fn property_condition(
        &mut self,
        node: SyntaxNode<'_>,
        owner: usize,
        segment: String,
        type_parameters: &[String],
    ) {
        let role = match node.kind() {
            SyntaxKind::PROPERTY_LIST_IF_MATCH_EXPRESSION => SourceRole::Match,
            _ => SourceRole::Condition,
        };
        let index = self.push(role, node.span(), Some(owner), segment);
        let mut conditions = 0;
        for (field, child) in fields(node) {
            if self.trivia(child, Some(index)) || !child.raw().is_named() {
                continue;
            }
            match (field, child.kind()) {
                (Some("condition"), _) => {
                    self.expression(child, Some(index), "test".to_string(), &[])
                }
                (Some("scrutinee"), _) => {
                    self.expression(child, Some(index), "subject".to_string(), &[])
                }
                (Some(slot @ ("then" | "else")), _) => self.properties(
                    child,
                    index,
                    &format!("{slot}."),
                    type_parameters,
                    &mut conditions,
                ),
                (_, SyntaxKind::PROPERTY_LIST_IF_CONDITION_ARM) => {
                    let test = child.child_by_field("condition").map(spell);
                    let arm = self.push(
                        SourceRole::MatchArm,
                        child.span(),
                        Some(index),
                        format!("when {}", test.unwrap_or_default()),
                    );
                    let mut arm_conditions = 0;
                    for (field, part) in fields(child) {
                        if self.trivia(part, Some(arm)) || !part.raw().is_named() {
                            continue;
                        }
                        match field {
                            Some("condition") => {
                                self.expression(part, Some(arm), "test".to_string(), &[])
                            }
                            _ => {
                                self.properties(part, arm, "", type_parameters, &mut arm_conditions)
                            }
                        }
                    }
                }
                (_, SyntaxKind::PROPERTY_LIST_IF_MATCH_ARM) => {
                    let arm = self.match_arm(child, index);
                    let mut arm_conditions = 0;
                    for (_, part) in fields(child) {
                        if part.kind() == SyntaxKind::PROPERTY_LIST && !self.trivia(part, Some(arm))
                        {
                            self.properties(part, arm, "", type_parameters, &mut arm_conditions);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Comments and regions that did not parse
    // -----------------------------------------------------------------------------------------

    /// Places a child every construct can hold — a comment, a region that did not parse, a token
    /// the parser invented — and returns whether `child` was one.
    fn trivia(&mut self, child: SyntaxNode<'_>, parent: Option<usize>) -> bool {
        if child.raw().is_missing() {
            return true;
        }
        if child.raw().is_error() {
            self.unparsed(child, parent);
            return true;
        }
        let role = match child.kind() {
            SyntaxKind::DOC_COMMENT => SourceRole::DocComment,
            SyntaxKind::LINE_COMMENT
            | SyntaxKind::BLOCK_COMMENT
            | SyntaxKind::HTML_BLOCK_COMMENT => SourceRole::Comment,
            _ => return false,
        };
        let index = self.push(role, child.span(), parent, "");
        self.nodes[index].value = Some(child.text().to_string());
        true
    }

    /// The comments and unparsed regions anywhere inside a construct that is one node.
    fn nested_trivia(&mut self, node: SyntaxNode<'_>, parent: usize) {
        for child in node.children_with_tokens() {
            if !self.trivia(child, Some(parent)) {
                self.nested_trivia(child, parent);
            }
        }
    }

    fn unparsed(&mut self, node: SyntaxNode<'_>, parent: Option<usize>) {
        if !node.span().is_empty() {
            self.unparsed_span(node.span(), parent);
        }
    }

    fn unparsed_span(&mut self, span: ByteTextRange, parent: Option<usize>) {
        let index = self.push(SourceRole::Unparsed, span, parent, "");
        self.nodes[index].value = Some(self.index_text(span));
    }

    fn index_text(&self, span: ByteTextRange) -> String {
        self.index.text[usize::from(span.start())..usize::from(span.end())].to_string()
    }

    // -----------------------------------------------------------------------------------------
    // What the analysis knows
    // -----------------------------------------------------------------------------------------

    fn expr_at(&self, span: ByteTextRange) -> Option<ExprId> {
        self.exprs.get(&span).copied()
    }

    /// The type of the expression lowered at exactly `span`, as hover reports it there.
    fn type_at(&self, span: ByteTextRange) -> Option<String> {
        let analysis = self.analysis.as_ref()?;
        let ty = analysis.type_env().get_expr_type(self.expr_at(span)?)?;
        (!is_unresolved_type(ty)).then(|| ty.to_string())
    }

    /// The union a case written at exactly `span` belongs to.
    fn case_declaration(&mut self, span: ByteTextRange) -> Option<u32> {
        let analysis = self.analysis.as_ref()?;
        let id = self.expr_at(span)?;
        let origin = match analysis.type_env().get_expr_type(id) {
            Some(nx_types::Type::UnionCase(case)) => case
                .origin()
                .map(|origin| (origin.module_identity().to_string(), origin.definition_id())),
            _ => match analysis.lowered_module().expr(id) {
                Expr::ResolvedUnionCase {
                    module_identity,
                    definition_id,
                    ..
                } => Some((module_identity.clone(), *definition_id)),
                _ => None,
            },
        }?;
        self.entry(&origin)
    }

    /// The declaration a name the document can see denotes.
    fn visible_declaration(&mut self, name: &str) -> Option<u32> {
        let origin = self.scope.visible.get(name)?.origin.clone();
        self.entry(&origin)
    }

    /// The declaration a type name written in this document denotes.
    fn type_declaration(&mut self, name: &str) -> Option<u32> {
        let origin = self
            .scope
            .workspace
            .type_namespaces
            .get(self.identity)?
            .get(name)?
            .clone();
        self.entry(&origin)
    }

    /// The declaration an element's tag names: the visible declaration of that name, or else the
    /// one the element's type names, which is how a component's own `<Update>` resolves.
    fn element_origin(&self, tag: &str, span: ByteTextRange) -> Option<DeclarationOrigin> {
        if let Some(declaration) = self.scope.visible.get(tag) {
            return Some(declaration.origin.clone());
        }
        let analysis = self.analysis.as_ref()?;
        match analysis.type_env().get_expr_type(self.expr_at(span)?)? {
            nx_types::Type::Named(named) => named
                .origin()
                .map(|origin| (origin.module_identity().to_string(), origin.definition_id())),
            _ => None,
        }
    }

    /// Whether `origin` is the update record of the component the walk is inside.
    fn is_state_update(&self, origin: Option<&DeclarationOrigin>) -> bool {
        let (Some(origin), Some(component)) = (origin, self.component.as_deref()) else {
            return false;
        };
        matches!(
            item_at(&self.scope.workspace, origin),
            Some(Item::Record(record))
                if matches!(&record.kind, RecordKind::Update { target } if target.as_str() == component)
        )
    }

    /// The type parameters of the declaration an element constructs, which its attributes bind
    /// to types rather than values.
    fn type_parameters(&self, origin: &DeclarationOrigin) -> Vec<String> {
        let names = match item_at(&self.scope.workspace, origin) {
            Some(Item::Component(component)) => &component.type_params,
            Some(Item::Record(record)) => &record.type_params,
            _ => return Vec::new(),
        };
        names
            .iter()
            .map(|param| param.name.as_str().to_string())
            .collect()
    }

    // -----------------------------------------------------------------------------------------
    // The declaration table
    // -----------------------------------------------------------------------------------------

    /// The table index of the declaration at `origin`, adding it, and its bases, on first use.
    fn entry(&mut self, origin: &DeclarationOrigin) -> Option<u32> {
        if let Some(index) = self.table_index.get(origin) {
            return Some(*index);
        }
        let workspace = Arc::clone(&self.scope.workspace);
        let item = item_at(&workspace, origin)?;
        let index = self.table.len() as u32;
        self.table_index.insert(origin.clone(), index);
        self.table.push(SourceDeclaration {
            module: origin.0.clone(),
            name: item.name().as_str().to_string(),
            kind: declaration_kind(item),
            range: None,
            doc: None,
            properties: Vec::new(),
            state: Vec::new(),
            bases: Vec::new(),
            cases: Vec::new(),
            ty: None,
            value_range: None,
        });

        let chain = base_chain(&workspace, origin);
        let bases = chain
            .iter()
            .filter_map(|base| self.entry(base))
            .collect::<Vec<_>>();

        // Inherited properties come first, furthest base first; a property a nearer declaration
        // writes again replaces the inherited one where it stands.
        let mut properties: Vec<SourceProperty> = Vec::new();
        for declaring in chain.iter().rev().chain(std::iter::once(origin)) {
            let Some(declaring_item) = item_at(&workspace, declaring) else {
                continue;
            };
            let inherited = declaring != origin;
            for mut property in self.own_properties(&workspace, declaring, declaring_item) {
                if inherited {
                    property.flags.push(SourceFlag::Inherited);
                }
                match properties
                    .iter_mut()
                    .find(|existing| existing.name == property.name)
                {
                    Some(existing) => *existing = property,
                    None => properties.push(property),
                }
            }
        }

        let module = origin.0.as_str();
        let here = module == self.identity;
        let entry = &mut self.table[index as usize];
        entry.properties = properties;
        entry.bases = bases;
        entry.doc = self.scope.render_doc(module, item.doc());
        entry.range = here.then(|| self.index.range_from_bytes(item.span()));
        match item {
            Item::Component(component) => {
                entry.state = component
                    .state
                    .iter()
                    .map(|field| field_property(&workspace, &self.scope, module, field))
                    .collect();
            }
            Item::Union(union_def) => {
                entry.cases = union_def
                    .cases
                    .iter()
                    .map(|case| SourceCase {
                        name: case.name.as_str().to_string(),
                        doc: self.scope.render_doc(module, case.doc.as_ref()),
                        properties: case
                            .fields
                            .iter()
                            .map(|field| {
                                property(
                                    &workspace,
                                    &self.scope,
                                    module,
                                    PropertyParts {
                                        name: field.name.as_str(),
                                        ty: &field.ty,
                                        optional: field.optional,
                                        content: field.is_content,
                                        default: field.default,
                                        doc: field.doc.as_ref(),
                                    },
                                )
                            })
                            .collect(),
                    })
                    .collect();
            }
            Item::TypeAlias(alias) => entry.ty = Some(type_ref_display(&alias.ty)),
            Item::Value(value) => {
                entry.ty = match &value.ty {
                    Some(ty) => Some(type_ref_display(ty)),
                    None => workspace
                        .artifact(module)
                        .and_then(|artifact| artifact.type_env.get_expr_type(value.value))
                        .filter(|ty| !is_unresolved_type(ty))
                        .map(|ty| ty.to_string()),
                };
                entry.value_range = here
                    .then(|| {
                        workspace
                            .artifact(module)?
                            .lowered_module
                            .as_deref()?
                            .known_expr_span(value.value)
                    })
                    .flatten()
                    .map(|span| self.index.range_from_bytes(span));
            }
            Item::Function(_) | Item::Record(_) => {}
        }
        Some(index)
    }

    /// The properties a declaration writes itself.
    fn own_properties(
        &self,
        workspace: &WorkspaceDeclarations,
        origin: &DeclarationOrigin,
        item: &Item,
    ) -> Vec<SourceProperty> {
        let module = origin.0.as_str();
        match item {
            Item::Record(record) => record
                .properties
                .iter()
                .map(|field| field_property(workspace, &self.scope, module, field))
                .collect(),
            Item::Component(component) => component
                .props
                .iter()
                .map(|field| field_property(workspace, &self.scope, module, field))
                .collect(),
            Item::Function(function) => function
                .params
                .iter()
                .map(|param| {
                    property(
                        workspace,
                        &self.scope,
                        module,
                        PropertyParts {
                            name: param.name.as_str(),
                            ty: &param.ty,
                            optional: param.optional,
                            content: param.is_content,
                            default: param.default,
                            doc: param.doc.as_ref(),
                        },
                    )
                })
                .collect(),
            Item::Value(_) | Item::TypeAlias(_) | Item::Union(_) => Vec::new(),
        }
    }
}

/// What a property is made of, whichever kind of declaration wrote it.
struct PropertyParts<'i> {
    name: &'i str,
    ty: &'i nx_hir::ast::TypeRef,
    optional: bool,
    content: bool,
    default: Option<ExprId>,
    doc: Option<&'i Doc>,
}

fn field_property(
    workspace: &WorkspaceDeclarations,
    scope: &DocumentScope,
    module: &str,
    field: &RecordField,
) -> SourceProperty {
    property(
        workspace,
        scope,
        module,
        PropertyParts {
            name: field.name.as_str(),
            ty: &field.ty,
            optional: field.optional,
            content: field.is_content,
            default: field.default,
            doc: field.doc.as_ref(),
        },
    )
}

fn property(
    workspace: &WorkspaceDeclarations,
    scope: &DocumentScope,
    module: &str,
    parts: PropertyParts<'_>,
) -> SourceProperty {
    let mut flags = Vec::new();
    if parts.optional {
        flags.push(SourceFlag::Optional);
    }
    if parts.content {
        flags.push(SourceFlag::Content);
    }
    SourceProperty {
        name: parts.name.to_string(),
        ty: type_ref_display(parts.ty),
        default: parts
            .default
            .and_then(|default| default_text(workspace, module, default)),
        doc: scope.render_doc(module, parts.doc),
        flags,
    }
}

/// A default as its declaring module wrote it.
fn default_text(
    workspace: &WorkspaceDeclarations,
    module: &str,
    default: ExprId,
) -> Option<String> {
    let span = workspace
        .artifact(module)?
        .lowered_module
        .as_deref()?
        .known_expr_span(default)?;
    let source = workspace.sources.get(module)?;
    source
        .get(usize::from(span.start())..usize::from(span.end()))
        .map(str::to_string)
}

fn item_at<'w>(
    workspace: &'w WorkspaceDeclarations,
    origin: &DeclarationOrigin,
) -> Option<&'w Item> {
    workspace
        .artifact(&origin.0)?
        .lowered_module
        .as_deref()?
        .item_by_definition(origin.1)
}

/// The bases of the declaration at `origin`, nearest first, each resolved in the module of the
/// declaration that names it.
fn base_chain(
    workspace: &WorkspaceDeclarations,
    origin: &DeclarationOrigin,
) -> Vec<DeclarationOrigin> {
    let mut chain: Vec<DeclarationOrigin> = Vec::new();
    let mut current = origin.clone();
    loop {
        let base = match item_at(workspace, &current) {
            Some(Item::Record(record)) => record.base.as_ref(),
            Some(Item::Component(component)) => component.base.as_ref(),
            Some(Item::Union(union_def)) => union_def.base.as_ref(),
            _ => None,
        };
        let Some(base) = base else {
            break;
        };
        let kind = workspace
            .by_origin
            .get(&current)
            .map(|declaration| declaration.kind);
        let resolved = kind
            .and_then(|kind| workspace.base_origin(&current, base.as_str(), kind))
            .or_else(|| {
                workspace
                    .type_namespaces
                    .get(&current.0)?
                    .get(base.as_str())
                    .cloned()
            });
        // A cycle is a declaration error the compiler reports; here it ends the chain.
        match resolved {
            Some(next) if next != *origin && !chain.contains(&next) => {
                chain.push(next.clone());
                current = next;
            }
            _ => break,
        }
    }
    chain
}

fn declaration_kind(item: &Item) -> SourceDeclarationKind {
    match item {
        Item::Function(_) => SourceDeclarationKind::Function,
        Item::Value(_) => SourceDeclarationKind::Value,
        Item::Component(_) => SourceDeclarationKind::Component,
        Item::TypeAlias(_) => SourceDeclarationKind::Alias,
        Item::Union(_) => SourceDeclarationKind::Union,
        Item::Record(record) if record.kind == RecordKind::Action => SourceDeclarationKind::Action,
        Item::Record(_) => SourceDeclarationKind::Record,
    }
}

/// Whether a node names nothing, so its key is taken from the sibling it precedes.
fn is_anonymous(role: SourceRole) -> bool {
    matches!(
        role,
        SourceRole::Comment | SourceRole::DocComment | SourceRole::Unparsed
    )
}

fn role_name(role: SourceRole) -> &'static str {
    match role {
        SourceRole::DocComment => "docComment",
        SourceRole::Unparsed => "unparsed",
        _ => "comment",
    }
}

/// Each child of `node`, tokens included, with the field it fills.
fn fields(node: SyntaxNode<'_>) -> Vec<(Option<&'static str>, SyntaxNode<'_>)> {
    let raw = node.raw();
    node.children_with_tokens()
        .enumerate()
        .map(|(index, child)| (raw.field_name_for_child(index as u32), child))
        .collect()
}

/// The items and the trivia of a list, through the hidden list nodes that group items.
fn parts(node: SyntaxNode<'_>) -> Vec<Part<'_>> {
    let mut parts = Vec::new();
    for child in node.children_with_tokens() {
        if child.raw().is_error() || child.raw().is_missing() || child.kind().is_comment() {
            parts.push(Part::Trivia(child));
        } else if !child.raw().is_named() {
            continue;
        } else if child.kind() == SyntaxKind::ELEMENTS_EXPRESSION {
            parts.extend(self::parts(child));
        } else {
            parts.push(Part::Item(child));
        }
    }
    parts
}

fn field_text(node: SyntaxNode<'_>, field: &str) -> Option<String> {
    node.child_by_field(field).map(spell)
}

fn field_kind_text(node: SyntaxNode<'_>, kind: SyntaxKind) -> Option<String> {
    node.children()
        .find(|child| child.kind() == kind)
        .map(|child| child.text().to_string())
}

/// The first descendant of `node` of `kind`, `node` included.
fn descendant(node: SyntaxNode<'_>, kind: SyntaxKind) -> Option<SyntaxNode<'_>> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| descendant(child, kind))
}

/// The name a written type is declared under: the named type at its root, under any occurrence
/// suffix or parentheses.
fn type_name(node: SyntaxNode<'_>) -> Option<String> {
    match node.kind() {
        SyntaxKind::TYPE => node.children().next().and_then(type_name),
        SyntaxKind::PARENTHESIZED_TYPE => node.child_by_field("type").and_then(type_name),
        SyntaxKind::USER_DEFINED_TYPE | SyntaxKind::QUALIFIED_NAME => Some(spell(node)),
        SyntaxKind::APPLIED_TYPE => node.child_by_field("name").map(spell),
        SyntaxKind::CONTEXTUAL_NAME => field_kind_text(node, SyntaxKind::IDENTIFIER),
        _ => None,
    }
}

/// The text of `node` without its comments, with each run of whitespace or comments between two
/// tokens written as one space.
fn spell(node: SyntaxNode<'_>) -> String {
    fn tokens<'t>(node: SyntaxNode<'t>, into: &mut Vec<SyntaxNode<'t>>) {
        if node.kind().is_comment() || node.raw().is_missing() {
            return;
        }
        if node.raw().child_count() == 0 {
            into.push(node);
            return;
        }
        for child in node.children_with_tokens() {
            tokens(child, into);
        }
    }
    let mut leaves = Vec::new();
    tokens(node, &mut leaves);
    let mut spelled = String::new();
    let mut end = None;
    for leaf in leaves {
        if end.is_some_and(|end| end < leaf.start_byte()) {
            spelled.push(' ');
        }
        spelled.push_str(leaf.text());
        end = Some(leaf.end_byte());
    }
    spelled
}
