//! CST → HIR lowering.
//!
//! This module converts the tree-sitter Concrete Syntax Tree (CST) into
//! our typed High-level Intermediate Representation (HIR).

use crate::ast::{
    BinOp, Expr, Literal, MatchArm, OrderedFloat, RecordLiteralProperty, Stmt, TypeRef, UnOp,
};
use crate::{
    property_union_name, update_record_name, Component, ComponentEmit, ComponentEmitKind, Element,
    ExprId, Function, Import, ImportKind, Item, LoweredModule, LoweringDiagnostic, Name, Param,
    Property, PropertyConditionArm, PropertyEntry, PropertyMatchArm, RecordDef, RecordField,
    RecordKind, SelectiveImport, SourceId, TypeAlias, UnionCaseDef, UnionCaseField, UnionDef,
    ValueDef, Visibility, PROPERTY_UNION_SUFFIX, UPDATE_RECORD_SUFFIX,
};
use nx_diagnostics::{TextSize, TextSpan};
use nx_syntax::{SyntaxKind, SyntaxNode};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Context for lowering operations.
///
/// Maintains the module being built and provides helper methods for
/// allocating expressions and handling errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeTag {
    Int,
    Int32,
    Int64,
    Float32,
    Float64,
    Boolean,
    String,
    Null,
    Unknown,
}

impl TypeTag {
    fn from_type_ref(ty: &TypeRef) -> Self {
        match ty {
            TypeRef::Name(name) => match name.as_str() {
                "string" => TypeTag::String,
                "int" => TypeTag::Int,
                "int32" => TypeTag::Int32,
                "int64" => TypeTag::Int64,
                "float32" => TypeTag::Float32,
                "float64" => TypeTag::Float64,
                "boolean" => TypeTag::Boolean,
                _ => TypeTag::Unknown,
            },
            TypeRef::Nullable(inner) => TypeTag::from_type_ref(inner),
            _ => TypeTag::Unknown,
        }
    }

    fn combine_numeric(lhs: TypeTag, rhs: TypeTag) -> TypeTag {
        match (lhs, rhs) {
            // Same-category integer promotion, by the rank order int32 < int < int64
            (TypeTag::Int64, TypeTag::Int32 | TypeTag::Int | TypeTag::Int64)
            | (TypeTag::Int32 | TypeTag::Int, TypeTag::Int64) => TypeTag::Int64,
            (TypeTag::Int, TypeTag::Int32 | TypeTag::Int) | (TypeTag::Int32, TypeTag::Int) => {
                TypeTag::Int
            }
            (TypeTag::Int32, TypeTag::Int32) => TypeTag::Int32,
            // Same-category float promotion
            (TypeTag::Float32, TypeTag::Float32) => TypeTag::Float32,
            (TypeTag::Float32, TypeTag::Float64) | (TypeTag::Float64, TypeTag::Float32) => {
                TypeTag::Float64
            }
            (TypeTag::Float64, TypeTag::Float64) => TypeTag::Float64,
            _ => TypeTag::Unknown,
        }
    }

    fn is_string(self) -> bool {
        matches!(self, TypeTag::String)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HandlerPropResolution {
    Emit(ComponentEmit),
    Collision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PredeclaredComponent {
    name: Name,
    visibility: Visibility,
    is_abstract: bool,
    is_external: bool,
    base: Option<Name>,
    declared_props: Vec<RecordField>,
    declared_emits: Vec<ComponentEmit>,
    effective_props: Vec<RecordField>,
    effective_emits: Vec<ComponentEmit>,
    span: TextSpan,
    handler_props: FxHashMap<Name, HandlerPropResolution>,
    finalized: bool,
}

pub struct LoweringContext {
    module: LoweredModule,
    expr_types: FxHashMap<ExprId, TypeTag>,
    scope_stack: Vec<FxHashMap<Name, TypeTag>>,
    predeclared_components: FxHashMap<Name, PredeclaredComponent>,
    component_emit_records: FxHashMap<Name, Vec<RecordDef>>,
    /// Records element tags may name before their declaration is lowered: inline emitted actions
    /// and every derived `<Target>.Update` record.
    predeclared_records: FxHashMap<Name, RecordDef>,
    /// Derived update records keyed by the name of the declaration they patch.
    update_records: FxHashMap<Name, RecordDef>,
    /// Update records to add once every declared item has been added, in declaration order.
    pending_update_items: Vec<RecordDef>,
    /// Derived property unions keyed by the name of the declaration whose fields they name.
    property_unions: FxHashMap<Name, UnionDef>,
    /// Property unions to add after the update records, in declaration order.
    pending_property_items: Vec<UnionDef>,
    /// The component whose declaration is being lowered, if any.
    ///
    /// <para>Inside a component a bare `Update` tag names that component's update record, a bare
    /// `Property` names its property union, and a handler bound there is owned by it.</para>
    current_component: Option<Name>,
}

impl LoweringContext {
    /// Creates a new lowering context for the given source file.
    pub fn new(source_id: SourceId) -> Self {
        Self {
            module: LoweredModule::new(source_id),
            expr_types: FxHashMap::default(),
            scope_stack: vec![FxHashMap::default()],
            predeclared_components: FxHashMap::default(),
            component_emit_records: FxHashMap::default(),
            predeclared_records: FxHashMap::default(),
            update_records: FxHashMap::default(),
            pending_update_items: Vec::new(),
            property_unions: FxHashMap::default(),
            pending_property_items: Vec::new(),
            current_component: None,
        }
    }

    /// Consumes the context and returns the completed module.
    pub fn finish(self) -> LoweredModule {
        self.module
    }

    /// Allocates an expression in the module arena.
    fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        let id = self.module.alloc_expr(expr);
        self.expr_types.insert(id, TypeTag::Unknown);
        id
    }

    /// Creates an error expression for malformed CST nodes.
    fn error_expr(&mut self, span: TextSpan) -> ExprId {
        self.alloc_expr(Expr::Error(span))
    }

    fn lower_qualified_name_expr(&mut self, node: SyntaxNode) -> ExprId {
        let mut parts = node
            .text()
            .split('.')
            .filter(|part| !part.is_empty())
            .map(Name::new)
            .collect::<Vec<_>>();
        self.resolve_bare_property_base(&mut parts);
        let mut parts = parts.into_iter();

        let Some(first) = parts.next() else {
            return self.error_expr(node.span());
        };

        let mut expr = self.alloc_expr(Expr::Ident(first.clone()));
        let first_span = node
            .children()
            .find(|child| child.kind() == SyntaxKind::IDENTIFIER)
            .map(|child| child.span())
            .unwrap_or_else(|| node.span());
        self.module.set_expr_span(expr, first_span);
        let ty = self.lookup_name(&first);
        self.set_expr_type(expr, ty);

        for member in parts {
            expr = self.alloc_expr(Expr::Member {
                base: expr,
                member,
                span: node.span(),
            });
        }

        expr
    }

    fn lower_pattern_expr(&mut self, node: SyntaxNode) -> ExprId {
        let pattern_node = node.children().next().unwrap_or(node);
        if pattern_node.kind() == SyntaxKind::QUALIFIED_NAME {
            // A single-segment pattern is a contextual name resolved against the scrutinee's type,
            // not a lexical identifier. Qualified patterns (`LoadState.idle`) keep their existing
            // member-access lowering.
            let text = pattern_node.text();
            if !text.contains('.') {
                let name = Name::new(text.trim());
                if !name.as_str().is_empty() {
                    return self.alloc_expr(Expr::ContextualName {
                        name,
                        span: pattern_node.span(),
                    });
                }
            }
            self.lower_qualified_name_expr(pattern_node)
        } else {
            self.lower_expr(pattern_node)
        }
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(FxHashMap::default());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn add_diagnostic(&mut self, message: impl Into<String>, span: TextSpan) {
        self.module.add_diagnostic(LoweringDiagnostic {
            message: message.into(),
            span,
        });
    }

    fn define_name(&mut self, name: &Name, ty: TypeTag) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name.clone(), ty);
        }
    }

    fn lookup_name(&self, name: &Name) -> TypeTag {
        for scope in self.scope_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return *ty;
            }
        }
        TypeTag::Unknown
    }

    /// Folds `-` applied directly to a numeric literal into a negative literal.
    ///
    /// Returns `None` for any other operand, so negation of a non-literal stays a unary operation.
    /// Folding everywhere — unbraced, braced, and inside a larger expression — keeps a single
    /// lowered representation for `-1.0`, `{-1.0}`, and the `-90` in `{-90 + rotation}`.
    ///
    /// <para>The operand is rewritten in place rather than replaced by a second expression. The
    /// two spell one literal, and leaving the unnegated one in the arena would leave an expression
    /// nothing references but a lookup by offset still finds — where it would shadow the folded
    /// literal, being the narrower of the two.</para>
    fn fold_negated_literal(&mut self, operand: ExprId, span: TextSpan) -> Option<ExprId> {
        let folded = match self.module.expr(operand) {
            Expr::Literal(Literal::Int(value)) => Literal::Int(value.wrapping_neg()),
            Expr::Literal(Literal::Float(value)) => Literal::Float(OrderedFloat(-value.0)),
            _ => return None,
        };
        *self.module.expr_mut(operand) = Expr::Literal(folded);
        // The span is the whole written form, `-` included: that is what the reader points at.
        self.module.set_expr_span(operand, span);
        Some(operand)
    }

    /// Allocates a literal with its type and the span it was written at.
    ///
    /// <para>`Expr::Literal` carries no span of its own, so a literal is located through the
    /// module's span map like an identifier is. Without an entry there a literal cannot be found
    /// by offset at all, which is what kept an editor from reporting the type of one.</para>
    fn literal_expr(&mut self, literal: Literal, ty: TypeTag, span: TextSpan) -> ExprId {
        let expr = self.alloc_expr(Expr::Literal(literal));
        self.set_expr_type(expr, ty);
        self.module.set_expr_span(expr, span);
        expr
    }

    fn set_expr_type(&mut self, expr: ExprId, ty: TypeTag) {
        self.expr_types.insert(expr, ty);
    }

    fn expr_type(&self, expr: ExprId) -> TypeTag {
        self.expr_types
            .get(&expr)
            .copied()
            .unwrap_or(TypeTag::Unknown)
    }

    fn unquote_string_literal(text: &str) -> String {
        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
            text[1..text.len() - 1].to_string()
        } else {
            text.to_string()
        }
    }

    fn lower_visibility(node: SyntaxNode) -> Visibility {
        match node.child_by_field("visibility").map(|child| child.text()) {
            Some("private") => Visibility::Private,
            Some("export") => Visibility::Export,
            _ => Visibility::Internal,
        }
    }

    fn find_library_path_node<'tree>(
        node: SyntaxNode<'tree>,
        preferred_field: &str,
    ) -> Option<SyntaxNode<'tree>> {
        node.child_by_field(preferred_field).or_else(|| {
            node.children().find(|child| {
                child.kind() == SyntaxKind::LIBRARY_PATH
                    || child.kind() == SyntaxKind::STRING_LITERAL
            })
        })
    }

    fn lower_library_path(node: SyntaxNode) -> Option<String> {
        match node.kind() {
            SyntaxKind::LIBRARY_PATH => {
                Self::find_library_path_node(node, "value").and_then(Self::lower_library_path)
            }
            SyntaxKind::STRING_LITERAL => Some(Self::unquote_string_literal(node.text())),
            _ => Self::find_library_path_node(node, "path").and_then(Self::lower_library_path),
        }
    }

    fn lower_selective_import(&mut self, node: SyntaxNode) -> Option<SelectiveImport> {
        if node.kind() != SyntaxKind::SELECTIVE_IMPORT {
            return None;
        }

        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .or_else(|| {
                node.children()
                    .find(|child| child.kind() == SyntaxKind::IDENTIFIER)
                    .map(|n| Name::new(n.text()))
            })?;

        let qualifier = node.child_by_field("alias").and_then(|alias_node| {
            let alias_text = alias_node.text();
            let mut parts = alias_text.split('.');
            let Some(prefix) = parts.next() else {
                return None;
            };
            let Some(imported_suffix) = parts.next() else {
                self.add_diagnostic(
                    format!(
                        "Selective import alias '{}' must be a qualified name of the form Prefix.{}",
                        alias_text,
                        name.as_str()
                    ),
                    alias_node.span(),
                );
                return None;
            };

            if parts.next().is_some() {
                self.add_diagnostic(
                    format!(
                        "Selective import alias '{}' must contain exactly one dot",
                        alias_text
                    ),
                    alias_node.span(),
                );
                return None;
            }

            if imported_suffix != name.as_str() {
                self.add_diagnostic(
                    format!(
                        "Selective import alias '{}' must end with '{}'",
                        alias_text,
                        name.as_str()
                    ),
                    alias_node.span(),
                );
                return None;
            }

            Some(Name::new(prefix))
        });

        Some(SelectiveImport {
            name,
            qualifier,
            span: node.span(),
        })
    }

    fn lower_import_statement(&mut self, node: SyntaxNode) -> Option<Import> {
        if node.kind() != SyntaxKind::IMPORT_STATEMENT {
            return None;
        }

        let kind_node = node.child_by_field("kind").or_else(|| {
            node.children().find(|child| {
                matches!(
                    child.kind(),
                    SyntaxKind::WILDCARD_IMPORT | SyntaxKind::SELECTIVE_IMPORT_LIST
                )
            })
        })?;

        match kind_node.kind() {
            SyntaxKind::WILDCARD_IMPORT => {
                let path = Self::find_library_path_node(kind_node, "path")
                    .and_then(Self::lower_library_path)?;

                let alias = kind_node
                    .child_by_field("alias")
                    .map(|alias_node| Name::new(alias_node.text()));

                Some(Import {
                    library_path: path,
                    kind: ImportKind::Wildcard { alias },
                    span: node.span(),
                })
            }
            SyntaxKind::SELECTIVE_IMPORT_LIST => {
                let path = Self::find_library_path_node(node, "path")
                    .and_then(Self::lower_library_path)?;

                let selective_entries = kind_node
                    .children()
                    .filter_map(|entry| self.lower_selective_import(entry))
                    .collect();

                Some(Import {
                    library_path: path,
                    kind: ImportKind::Selective {
                        entries: selective_entries,
                    },
                    span: node.span(),
                })
            }
            _ => None,
        }
    }

    fn lower_sequence_expr_from_items(&mut self, node: SyntaxNode) -> ExprId {
        let items: Vec<_> = node.children().collect();
        match items.len() {
            // `{}` is the empty list. Only the values brace admits zero items grammatically; a
            // zero-item elements or embed brace can only be the product of error recovery, and
            // stays an error expression so the recovered tree does not read as a valid empty list.
            0 if node.kind() == SyntaxKind::VALUES_BRACED_EXPRESSION => {
                self.alloc_expr(Expr::Array {
                    elements: Vec::new(),
                    span: node.span(),
                })
            }
            0 => self.error_expr(node.span()),
            1 => self.lower_expr(items[0]),
            _ => {
                let elements = items
                    .into_iter()
                    .map(|item| self.lower_expr(item))
                    .collect();
                self.alloc_expr(Expr::Array {
                    elements,
                    span: node.span(),
                })
            }
        }
    }

    fn property_value_node<'tree>(node: SyntaxNode<'tree>) -> Option<SyntaxNode<'tree>> {
        node.child_by_field("value").or_else(|| {
            node.children().find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::STRING_LITERAL
                        | SyntaxKind::VALUES_BRACED_EXPRESSION
                        | SyntaxKind::ELEMENT
                        | SyntaxKind::VALUE_EXPRESSION
                        | SyntaxKind::RHS_EXPRESSION
                )
            })
        })
    }

    fn lower_value_or_error<'tree>(
        &mut self,
        value_node: Option<SyntaxNode<'tree>>,
        span: TextSpan,
    ) -> ExprId {
        if let Some(value_node) = value_node {
            self.lower_expr(value_node)
        } else {
            self.error_expr(span)
        }
    }

    fn handler_prop_name(emit_name: &str) -> String {
        format!("on{}", emit_name)
    }

    fn local_emit_name(action_name: &str) -> &str {
        action_name.rsplit('.').next().unwrap_or(action_name)
    }

    fn is_handler_binding_candidate(prop_name: &str) -> bool {
        if !prop_name.starts_with("on") || prop_name.len() <= 2 {
            return false;
        }

        prop_name
            .as_bytes()
            .get(2)
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false)
    }

    fn find_predeclared_component(&self, name: &str) -> Option<&PredeclaredComponent> {
        self.predeclared_components.get(&Name::new(name))
    }

    fn finalize_predeclared_component(&mut self, name: &Name, stack: &mut Vec<Name>) {
        if self
            .predeclared_components
            .get(name)
            .map(|component| component.finalized)
            .unwrap_or(false)
        {
            return;
        }

        if stack.contains(name) {
            return;
        }

        stack.push(name.clone());

        let base = self
            .predeclared_components
            .get(name)
            .and_then(|component| component.base.clone());
        if let Some(base_name) = base.clone() {
            let should_inherit = self
                .predeclared_components
                .get(&base_name)
                .map(|component| component.is_abstract)
                .unwrap_or(false);
            if should_inherit {
                self.finalize_predeclared_component(&base_name, stack);
            }
        }

        let mut effective_props = Vec::new();
        let mut effective_emits = Vec::new();

        if let Some(base_name) = base {
            if let Some(base_component) = self.predeclared_components.get(&base_name) {
                if base_component.is_abstract {
                    effective_props.extend(base_component.effective_props.clone());
                    effective_emits.extend(base_component.effective_emits.clone());
                }
            }
        }

        let (declared_props, declared_emits, component_name) = self
            .predeclared_components
            .get(name)
            .map(|component| {
                (
                    component.declared_props.clone(),
                    component.declared_emits.clone(),
                    component.name.clone(),
                )
            })
            .unwrap_or_else(|| (Vec::new(), Vec::new(), name.clone()));

        effective_props.extend(declared_props.clone());
        effective_emits.extend(declared_emits.clone());

        let mut handler_props = FxHashMap::default();
        let mut seen_handler_props: FxHashMap<Name, TextSpan> = FxHashMap::default();
        for emit in &effective_emits {
            let handler_name = Name::new(&Self::handler_prop_name(emit.name.as_str()));
            let mut has_collision = false;

            if effective_props.iter().any(|prop| prop.name == handler_name) {
                self.add_diagnostic(
                    format!(
                        "Component '{}' declares prop '{}' which collides with emitted action handler '{}'",
                        component_name.as_str(),
                        handler_name.as_str(),
                        handler_name.as_str()
                    ),
                    emit.span,
                );
                has_collision = true;
            }

            if let Some(previous_span) = seen_handler_props.insert(handler_name.clone(), emit.span)
            {
                self.add_diagnostic(
                    format!(
                        "Component '{}' emits multiple actions that map to handler '{}'",
                        component_name.as_str(),
                        handler_name.as_str()
                    ),
                    previous_span,
                );
                has_collision = true;
            }

            handler_props.insert(
                handler_name,
                if has_collision {
                    HandlerPropResolution::Collision
                } else {
                    HandlerPropResolution::Emit(emit.clone())
                },
            );
        }

        if let Some(component) = self.predeclared_components.get_mut(name) {
            component.effective_props = effective_props;
            component.effective_emits = effective_emits;
            component.handler_props = handler_props;
            component.finalized = true;
        }

        stack.pop();
    }

    fn find_predeclared_record(&self, name: &str) -> Option<&RecordDef> {
        self.predeclared_records.get(&Name::new(name))
    }

    /// Reads the name, type, and content modifier of each field declared under `node`, without
    /// lowering default expressions.
    ///
    /// <para>An update record never carries a default, and lowering one here would allocate an
    /// expression a second time for a default the declaration itself lowers.</para>
    fn lower_field_signatures(&self, node: SyntaxNode) -> Vec<RecordField> {
        node.children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
            .map(|prop| {
                let ty_node = prop.child_by_field("type").unwrap_or(prop);
                RecordField::with_content(
                    Self::property_definition_name(prop),
                    self.lower_type(ty_node),
                    Self::property_definition_is_content(prop),
                    None,
                    prop.span(),
                )
            })
            .collect()
    }

    /// Synthesizes and registers the derived update record and property union for one
    /// record-shaped declaration.
    ///
    /// <para>The record mirrors the fields `target` declares, with defaults stripped, and the union
    /// has one constant case per field. Inherited fields are added later: to the record when its
    /// effective shape is resolved, and to the union by `complete_property_unions` once the module
    /// is prepared, since a base may live in another module.</para>
    fn predeclare_update_record(
        &mut self,
        target: &Name,
        visibility: Visibility,
        fields: &[RecordField],
        span: TextSpan,
    ) {
        let record = RecordDef {
            name: update_record_name(target.as_str()),
            visibility,
            kind: RecordKind::Update {
                target: target.clone(),
            },
            is_abstract: false,
            base: None,
            properties: fields
                .iter()
                .map(|field| RecordField {
                    default: None,
                    ..field.clone()
                })
                .collect(),
            span,
        };
        self.predeclared_records
            .insert(record.name.clone(), record.clone());
        self.update_records.insert(target.clone(), record);

        let union = UnionDef {
            name: property_union_name(target.as_str()),
            visibility,
            base: None,
            cases: fields
                .iter()
                .map(|field| UnionCaseDef {
                    name: field.name.clone(),
                    fields: Vec::new(),
                    span: field.span,
                })
                .collect(),
            property_target: Some(target.clone()),
            span,
        };
        self.property_unions.insert(target.clone(), union);
    }

    /// Predeclares the update record of every top-level `type` record and `action`.
    fn predeclare_record_update_records(&mut self, root: SyntaxNode) {
        for child in root.children() {
            if !matches!(
                child.kind(),
                SyntaxKind::RECORD_DEFINITION | SyntaxKind::ACTION_DEFINITION
            ) {
                continue;
            }
            let Some(name) = child.child_by_field("name").map(|n| Name::new(n.text())) else {
                continue;
            };
            let fields = self.lower_field_signatures(child);
            self.predeclare_update_record(
                &name,
                Self::lower_visibility(child),
                &fields,
                child.span(),
            );
        }
    }

    /// Queues the update record and property union derived from `target`, if they were
    /// predeclared.
    ///
    /// <para>Derived items are added after every declared item, so they never shift the index —
    /// and so the definition identity — of anything the author wrote. Every update record precedes
    /// every property union for the same reason: adding the unions did not move the records.</para>
    fn add_derived_items(&mut self, target: &Name) {
        if let Some(record) = self.update_records.get(target).cloned() {
            self.pending_update_items.push(record);
        }
        if let Some(union) = self.property_unions.get(target).cloned() {
            self.pending_property_items.push(union);
        }
    }

    fn property_definition_name(prop: SyntaxNode) -> Name {
        prop.child_by_field("name")
            .map(|node| Name::new(node.text()))
            .unwrap_or_else(|| Name::new("_"))
    }

    fn property_definition_is_content(prop: SyntaxNode) -> bool {
        prop.child_by_field("modifier")
            .map(|node| node.text() == "content")
            .unwrap_or(false)
    }

    fn lower_property_definition(
        &mut self,
        prop: SyntaxNode,
    ) -> (Name, TypeRef, Option<ExprId>, bool) {
        let field_name = Self::property_definition_name(prop);
        let ty_node = prop.child_by_field("type").unwrap_or(prop);
        let ty = self.lower_type(ty_node);
        let default = prop
            .child_by_field("default")
            .map(|default_node| self.lower_expr(default_node));
        let is_content = Self::property_definition_is_content(prop);

        if let Some(modifier) = prop.child_by_field("modifier") {
            if modifier.text() != "content" {
                self.add_diagnostic(
                    format!(
                        "Unsupported property modifier '{}'; only 'content' is allowed here",
                        modifier.text()
                    ),
                    modifier.span(),
                );
            }
        }

        (field_name, ty, default, is_content)
    }

    fn lower_record_fields_from_node(
        &mut self,
        node: SyntaxNode,
        define_names: bool,
    ) -> Vec<RecordField> {
        let mut properties = Vec::new();
        let mut content_field_name: Option<Name> = None;
        for prop in node
            .children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        {
            let (field_name, ty, default, is_content) = self.lower_property_definition(prop);

            if is_content {
                if let Some(existing_name) = content_field_name.as_ref() {
                    self.add_diagnostic(
                        format!(
                            "Only one content property is allowed in this declaration; '{}' conflicts with '{}'",
                            field_name.as_str(),
                            existing_name.as_str()
                        ),
                        prop.span(),
                    );
                } else {
                    content_field_name = Some(field_name.clone());
                }
            }

            if define_names {
                self.define_name(&field_name, TypeTag::from_type_ref(&ty));
            }

            properties.push(RecordField::with_content(
                field_name,
                ty,
                is_content,
                default,
                prop.span(),
            ));
        }

        properties
    }

    fn predeclare_component(&mut self, node: SyntaxNode) {
        let Some(signature) = node.child_by_field("signature") else {
            return;
        };

        let name = signature
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let visibility = Self::lower_visibility(node);
        let is_abstract = node.child_by_field("abstract").is_some();
        let is_external = node.child_by_field("external").is_some();
        let base = signature
            .child_by_field("base")
            .map(|base| Name::new(base.text()));

        let enclosing_component = self.current_component.replace(name.clone());

        // State is lowered with the component body; only the update record and property union
        // derived from it are needed now, so element tags and type references in any body can
        // name them.
        let state_fields = node
            .child_by_field("body")
            .and_then(|body| body.child_by_field("state"))
            .map(|state| self.lower_field_signatures(state))
            .unwrap_or_default();
        if !state_fields.is_empty() {
            self.predeclare_update_record(&name, visibility, &state_fields, node.span());
        }

        let props = self.lower_record_fields_from_node(signature, false);

        let mut emits = Vec::new();
        let mut inline_records = Vec::new();
        if let Some(emits_group) = signature.child_by_field("emits") {
            for emit_node in emits_group.children() {
                match emit_node.kind() {
                    SyntaxKind::EMIT_DEFINITION => {
                        let emit_name = emit_node
                            .child_by_field("name")
                            .map(|n| Name::new(n.text()))
                            .unwrap_or_else(|| Name::new("unknown"));
                        if emit_name.as_str() == UPDATE_RECORD_SUFFIX {
                            self.add_diagnostic(
                                format!(
                                    "Component '{}' cannot declare an emitted action named 'Update': '{}.Update' is reserved for the component's derived update record",
                                    name.as_str(),
                                    name.as_str()
                                ),
                                emit_node.span(),
                            );
                            continue;
                        }
                        if emit_name.as_str() == PROPERTY_UNION_SUFFIX {
                            self.add_diagnostic(
                                format!(
                                    "Component '{}' cannot declare an emitted action named 'Property': '{}.Property' is reserved for the component's derived property union",
                                    name.as_str(),
                                    name.as_str()
                                ),
                                emit_node.span(),
                            );
                            continue;
                        }
                        let action_name =
                            Name::new(&format!("{}.{}", name.as_str(), emit_name.as_str()));
                        let record = RecordDef {
                            name: action_name.clone(),
                            visibility,
                            kind: RecordKind::Action,
                            is_abstract: false,
                            base: emit_node
                                .child_by_field("base")
                                .map(|base| Name::new(base.text())),
                            properties: self.lower_record_fields_from_node(emit_node, false),
                            span: emit_node.span(),
                        };
                        self.predeclared_records
                            .insert(action_name.clone(), record.clone());
                        self.predeclare_update_record(
                            &action_name,
                            visibility,
                            &record.properties,
                            record.span,
                        );
                        inline_records.push(record);
                        emits.push(ComponentEmit {
                            name: emit_name,
                            action_name,
                            kind: ComponentEmitKind::Inline,
                            span: emit_node.span(),
                        });
                    }
                    SyntaxKind::EMIT_REFERENCE => {
                        let action_name = emit_node
                            .child_by_field("name")
                            .map(|n| Name::new(n.text()))
                            .unwrap_or_else(|| Name::new("unknown"));
                        let local_name = Name::new(Self::local_emit_name(action_name.as_str()));
                        if local_name.as_str() == UPDATE_RECORD_SUFFIX {
                            self.add_diagnostic(
                                format!(
                                    "Component '{}' cannot emit '{}': the handler 'onUpdate' and the name '{}.Update' would collide with the component's derived update record",
                                    name.as_str(),
                                    action_name.as_str(),
                                    name.as_str()
                                ),
                                emit_node.span(),
                            );
                            continue;
                        }
                        if local_name.as_str() == PROPERTY_UNION_SUFFIX {
                            self.add_diagnostic(
                                format!(
                                    "Component '{}' cannot emit '{}': the name '{}.Property' would collide with the component's derived property union",
                                    name.as_str(),
                                    action_name.as_str(),
                                    name.as_str()
                                ),
                                emit_node.span(),
                            );
                            continue;
                        }
                        emits.push(ComponentEmit {
                            name: local_name,
                            action_name,
                            kind: ComponentEmitKind::Shared,
                            span: emit_node.span(),
                        });
                    }
                    _ => {}
                }
            }
        }

        self.current_component = enclosing_component;

        self.component_emit_records
            .insert(name.clone(), inline_records);
        self.predeclared_components.insert(
            name.clone(),
            PredeclaredComponent {
                name,
                visibility,
                is_abstract,
                is_external,
                base,
                declared_props: props.clone(),
                declared_emits: emits.clone(),
                effective_props: props,
                effective_emits: emits,
                span: node.span(),
                handler_props: FxHashMap::default(),
                finalized: false,
            },
        );
    }

    fn predeclare_components(&mut self, root: SyntaxNode) {
        for child in root.children() {
            if child.kind() == SyntaxKind::COMPONENT_DEFINITION {
                self.predeclare_component(child);
            }
        }

        let component_names = self
            .predeclared_components
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut stack = Vec::new();
        for name in component_names {
            self.finalize_predeclared_component(&name, &mut stack);
        }
    }

    fn lower_component_definition(&mut self, node: SyntaxNode) -> Component {
        let name = node
            .child_by_field("signature")
            .and_then(|signature| signature.child_by_field("name"))
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));

        let body_node = node.child_by_field("body");
        let predeclared = self.predeclared_components.get(&name).cloned();
        let (visibility, is_abstract, is_external, base, props, emits, span) = predeclared
            .map(|component| {
                (
                    component.visibility,
                    component.is_abstract,
                    component.is_external,
                    component.base,
                    component.declared_props,
                    component.declared_emits,
                    component.span,
                )
            })
            .unwrap_or_else(|| {
                (
                    Self::lower_visibility(node),
                    node.child_by_field("abstract").is_some(),
                    node.child_by_field("external").is_some(),
                    node.child_by_field("signature")
                        .and_then(|signature| signature.child_by_field("base"))
                        .map(|base| Name::new(base.text())),
                    Vec::new(),
                    Vec::new(),
                    node.span(),
                )
            });

        self.push_scope();
        for prop in &props {
            self.define_name(&prop.name, TypeTag::from_type_ref(&prop.ty));
        }

        let enclosing_component = self.current_component.replace(name.clone());
        let state = body_node
            .and_then(|body| body.child_by_field("state"))
            .map(|state_node| self.lower_record_fields_from_node(state_node, true))
            .unwrap_or_default();
        let body = body_node
            .and_then(|body| body.child_by_field("body"))
            .map(|body_expr| self.lower_expr(body_expr));
        self.current_component = enclosing_component;
        self.pop_scope();

        Component {
            name,
            visibility,
            is_abstract,
            is_external,
            base,
            props,
            emits,
            state,
            body,
            span,
        }
    }
    /// Lowers a SyntaxNode to an expression.
    pub fn lower_expr(&mut self, node: SyntaxNode) -> ExprId {
        if node.is_error() {
            return self.error_expr(node.span());
        }

        match node.kind() {
            // Literals
            SyntaxKind::STRING_LITERAL | SyntaxKind::STRING_EXPRESSION => {
                let s = Self::unquote_string_literal(node.text());
                self.literal_expr(
                    Literal::String(SmolStr::new(s)),
                    TypeTag::String,
                    node.span(),
                )
            }

            SyntaxKind::INT_LITERAL => {
                let text = node.text();
                match text.parse::<i64>() {
                    Ok(value) => self.literal_expr(Literal::Int(value), TypeTag::Int, node.span()),
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::HEX_LITERAL => {
                let text = node.text();
                let digits = text.trim_start_matches("0x").trim_start_matches("0X");
                match i64::from_str_radix(digits, 16) {
                    Ok(value) => self.literal_expr(Literal::Int(value), TypeTag::Int, node.span()),
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::NUMBER_LITERAL
            | SyntaxKind::NUMBER_EXPRESSION
            | SyntaxKind::REAL_LITERAL => {
                let text = node.text();
                if let Ok(value) = text.parse::<i64>() {
                    self.literal_expr(Literal::Int(value), TypeTag::Int, node.span())
                } else if let Ok(value) = text.parse::<f64>() {
                    self.literal_expr(
                        Literal::Float(OrderedFloat(value)),
                        TypeTag::Float64,
                        node.span(),
                    )
                } else {
                    self.error_expr(node.span())
                }
            }

            SyntaxKind::BOOLEAN_LITERAL
            | SyntaxKind::BOOL_LITERAL
            | SyntaxKind::BOOLEAN_EXPRESSION => {
                let text = node.text();
                let value = text == "true";
                self.literal_expr(Literal::Boolean(value), TypeTag::Boolean, node.span())
            }

            SyntaxKind::NULL_LITERAL | SyntaxKind::NULL_EXPRESSION => {
                self.literal_expr(Literal::Null, TypeTag::Null, node.span())
            }

            // Identifier
            SyntaxKind::QUALIFIED_NAME => self.lower_qualified_name_expr(node),

            SyntaxKind::IDENTIFIER | SyntaxKind::IDENTIFIER_EXPRESSION => {
                // For identifier expressions, get the actual identifier child
                if let Some(id_node) = node
                    .child_by_field("name")
                    .or_else(|| node.children().find(|n| n.kind() == SyntaxKind::IDENTIFIER))
                {
                    let name = Name::new(id_node.text());
                    let expr = self.alloc_expr(Expr::Ident(name.clone()));
                    self.module.set_expr_span(expr, id_node.span());
                    let ty = self.lookup_name(&name);
                    self.set_expr_type(expr, ty);
                    expr
                } else {
                    let name = Name::new(node.text());
                    let expr = self.alloc_expr(Expr::Ident(name.clone()));
                    self.module.set_expr_span(expr, node.span());
                    let ty = self.lookup_name(&name);
                    self.set_expr_type(expr, ty);
                    expr
                }
            }

            // Binary operations
            SyntaxKind::BINARY_EXPRESSION => {
                let lhs = node
                    .child_by_field("left")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let rhs = node
                    .child_by_field("right")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                // Find operator
                let op = node.children_with_tokens().find_map(|n| match n.kind() {
                    SyntaxKind::PLUS => Some(BinOp::Add),
                    SyntaxKind::MINUS => Some(BinOp::Sub),
                    SyntaxKind::STAR => Some(BinOp::Mul),
                    SyntaxKind::SLASH => Some(BinOp::Div),
                    SyntaxKind::PERCENT => Some(BinOp::Mod),
                    SyntaxKind::EQ_EQ => Some(BinOp::Eq),
                    SyntaxKind::BANG_EQ => Some(BinOp::Ne),
                    SyntaxKind::LT => Some(BinOp::Lt),
                    SyntaxKind::GT => Some(BinOp::Gt),
                    SyntaxKind::LT_EQ => Some(BinOp::Le),
                    SyntaxKind::GT_EQ => Some(BinOp::Ge),
                    SyntaxKind::AMP_AMP => Some(BinOp::And),
                    SyntaxKind::PIPE_PIPE => Some(BinOp::Or),
                    _ => None,
                });

                if let Some(mut op) = op {
                    if matches!(op, BinOp::Add) {
                        let lhs_ty = self.expr_type(lhs);
                        let rhs_ty = self.expr_type(rhs);
                        if lhs_ty.is_string() && rhs_ty.is_string() {
                            op = BinOp::Concat;
                        }
                    }

                    let result_ty = match op {
                        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                            TypeTag::combine_numeric(self.expr_type(lhs), self.expr_type(rhs))
                        }
                        BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Gt
                        | BinOp::Le
                        | BinOp::Ge
                        | BinOp::And
                        | BinOp::Or => TypeTag::Boolean,
                        BinOp::Concat => TypeTag::String,
                    };

                    let expr = self.alloc_expr(Expr::BinaryOp {
                        lhs,
                        op,
                        rhs,
                        span: node.span(),
                    });
                    self.set_expr_type(expr, result_ty);
                    expr
                } else {
                    self.error_expr(node.span())
                }
            }

            // Unary operations
            SyntaxKind::UNARY_EXPRESSION | SyntaxKind::PREFIX_UNARY_EXPRESSION => {
                let expr_node = node
                    .child_by_field("operand")
                    .or_else(|| node.children().last())
                    .unwrap();
                let expr = self.lower_expr(expr_node);

                let op = node
                    .child_by_field("operator")
                    .map(|n| match n.kind() {
                        SyntaxKind::BANG => UnOp::Not,
                        SyntaxKind::MINUS => UnOp::Neg,
                        _ => {
                            if n.text() == "!" {
                                UnOp::Not
                            } else {
                                UnOp::Neg
                            }
                        }
                    })
                    .or_else(|| {
                        node.children_with_tokens().find_map(|n| match n.kind() {
                            SyntaxKind::BANG => Some(UnOp::Not),
                            SyntaxKind::MINUS => Some(UnOp::Neg),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| {
                        let text = node.text().trim_start();
                        if text.starts_with('!') {
                            UnOp::Not
                        } else {
                            UnOp::Neg
                        }
                    });

                if op == UnOp::Neg {
                    if let Some(folded) = self.fold_negated_literal(expr, node.span()) {
                        return folded;
                    }
                }

                let expr_id = self.alloc_expr(Expr::UnaryOp {
                    op,
                    expr,
                    span: node.span(),
                });

                let operand_ty = self.expr_type(expr);
                let result_ty = match op {
                    UnOp::Not => TypeTag::Boolean,
                    UnOp::Neg => match operand_ty {
                        TypeTag::Int
                        | TypeTag::Int32
                        | TypeTag::Int64
                        | TypeTag::Float32
                        | TypeTag::Float64 => operand_ty,
                        _ => TypeTag::Unknown,
                    },
                };

                self.set_expr_type(expr_id, result_ty);
                expr_id
            }

            // Call expression
            SyntaxKind::CALL_EXPRESSION => {
                let func = node
                    .child_by_field("function")
                    .or_else(|| node.children().next())
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let args = node
                    .children()
                    .skip(1)
                    .filter(|n| {
                        !matches!(
                            n.kind(),
                            SyntaxKind::LPAREN | SyntaxKind::RPAREN | SyntaxKind::COMMA
                        )
                    })
                    .map(|n| self.lower_expr(n))
                    .collect();

                self.alloc_expr(Expr::Call {
                    func,
                    args,
                    span: node.span(),
                })
            }

            // Member access
            SyntaxKind::MEMBER_EXPRESSION | SyntaxKind::MEMBER_ACCESS_EXPRESSION => {
                let object = node
                    .child_by_field("object")
                    .or_else(|| node.children().next());
                let base = match object {
                    Some(object) if self.is_bare_property_base(object) => {
                        self.lower_bare_property_base(object)
                    }
                    Some(object) => self.lower_expr(object),
                    None => self.error_expr(node.span()),
                };

                let member = node
                    .child_by_field("property")
                    .or_else(|| node.children().nth(1))
                    .map(|n| Name::new(n.text()))
                    .unwrap_or_else(|| Name::new(""));

                self.alloc_expr(Expr::Member {
                    base,
                    member,
                    span: node.span(),
                })
            }

            SyntaxKind::ELEMENT | SyntaxKind::TEXT_CHILD_ELEMENT => {
                let span = node.span();
                let element = self.lower_element(node);

                if element.content.is_empty() {
                    if let Some(record_def) = self
                        .module
                        .find_item(element.tag.as_str())
                        .and_then(|item| match item {
                            Item::Record(record_def) => Some(record_def),
                            _ => None,
                        })
                        .or_else(|| self.find_predeclared_record(element.tag.as_str()))
                    {
                        let props = element
                            .properties
                            .iter()
                            .map(|p| RecordLiteralProperty {
                                name: p.key.clone(),
                                value: p.value,
                                span: p.span,
                            })
                            .collect();
                        return self.alloc_expr(Expr::RecordLiteral {
                            record: record_def.name.clone(),
                            properties: props,
                            span,
                        });
                    }
                }

                let element_id = self.module.alloc_element(element);
                self.alloc_expr(Expr::Element {
                    element: element_id,
                    span,
                })
            }

            // Sequence (array) expression
            SyntaxKind::SEQUENCE_EXPRESSION => {
                let elements = node
                    .children()
                    .filter(|n| {
                        !matches!(
                            n.kind(),
                            SyntaxKind::LBRACKET | SyntaxKind::RBRACKET | SyntaxKind::COMMA
                        )
                    })
                    .map(|n| self.lower_expr(n))
                    .collect();

                self.alloc_expr(Expr::Array {
                    elements,
                    span: node.span(),
                })
            }

            // Parenthesized expression - unwrap
            SyntaxKind::PARENTHESIZED_EXPRESSION => node
                .children()
                .find(|n| !matches!(n.kind(), SyntaxKind::LPAREN | SyntaxKind::RPAREN))
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // Value expression wrappers - unwrap
            SyntaxKind::LITERAL => node
                // Tree-sitter wraps actual literal nodes (string/int/etc.) in a
                // `literal` parent. Unwrap so downstream code keeps seeing the
                // concrete literal expression.
                .children()
                .next()
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // A bare name written where a literal is required. Deliberately not `Expr::Ident`:
            // it resolves against the expected type at its binding site, not against lexical scope.
            SyntaxKind::CONTEXTUAL_NAME => {
                let name = Name::new(node.text().trim());
                self.alloc_expr(Expr::ContextualName {
                    name,
                    span: node.span(),
                })
            }

            // `-` directly before a numeric literal, folded to a negative literal.
            SyntaxKind::SIGNED_NUMERIC_LITERAL => node
                .children()
                .find(|n| !matches!(n.kind(), SyntaxKind::MINUS))
                .map(|n| {
                    let operand = self.lower_expr(n);
                    self.fold_negated_literal(operand, node.span())
                        .unwrap_or_else(|| self.error_expr(node.span()))
                })
                .unwrap_or_else(|| self.error_expr(node.span())),

            SyntaxKind::VALUE_EXPRESSION | SyntaxKind::VALUE_EXPR | SyntaxKind::RHS_EXPRESSION => {
                node.children()
                    .next()
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()))
            }

            SyntaxKind::VALUES_BRACED_EXPRESSION | SyntaxKind::EMBED_BRACED_EXPRESSION => {
                self.lower_sequence_expr_from_items(node)
            }
            SyntaxKind::ELEMENTS_BRACED_EXPRESSION => node
                .children()
                .next()
                .map(|child| self.lower_sequence_expr_from_items(child))
                .unwrap_or_else(|| self.error_expr(node.span())),

            SyntaxKind::VALUE_LIST_ITEM_EXPRESSION => node
                .children()
                .next()
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // For loop expression
            SyntaxKind::VALUE_FOR_EXPRESSION | SyntaxKind::ELEMENTS_FOR_EXPRESSION => {
                // Get item identifier
                let item = node
                    .child_by_field("item")
                    .map(|n| Name::new(n.text()))
                    .unwrap_or_else(|| Name::new("_"));

                // Get optional index identifier
                let index = node.child_by_field("index").map(|n| Name::new(n.text()));

                // Get iterable expression
                let iterable = node
                    .child_by_field("iterable")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                // Get body expression
                let body = node
                    .child_by_field("body")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                self.alloc_expr(Expr::For {
                    item,
                    index,
                    iterable,
                    body,
                    span: node.span(),
                })
            }

            // Ternary expression: condition ? consequent : alternative
            SyntaxKind::CONDITIONAL_EXPRESSION => {
                let condition = node
                    .child_by_field("condition")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let then_branch = node
                    .child_by_field("consequent")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let else_branch = node
                    .child_by_field("alternative")
                    .map(|n| self.lower_expr(n));

                self.alloc_expr(Expr::If {
                    condition,
                    then_branch,
                    else_branch,
                    span: node.span(),
                })
            }

            // Value if expression wrapper
            SyntaxKind::VALUE_IF_EXPRESSION | SyntaxKind::ELEMENTS_IF_EXPRESSION => node
                .children()
                .next()
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // If-else expression: if condition { then } else { else }
            SyntaxKind::VALUE_IF_SIMPLE_EXPRESSION | SyntaxKind::ELEMENTS_IF_SIMPLE_EXPRESSION => {
                let condition = node
                    .child_by_field("condition")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let then_branch = node
                    .child_by_field("then")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let else_branch = node.child_by_field("else").map(|n| self.lower_expr(n));

                self.alloc_expr(Expr::If {
                    condition,
                    then_branch,
                    else_branch,
                    span: node.span(),
                })
            }

            // Condition list expression: if { cond1 => expr1, cond2 => expr2, else => default }
            // We lower this to nested if-else expressions
            SyntaxKind::VALUE_IF_CONDITION_LIST_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_CONDITION_LIST_EXPRESSION => {
                // Collect all condition arms
                let mut arms: Vec<(ExprId, ExprId)> = Vec::new();
                let mut else_expr: Option<ExprId> = None;

                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::VALUE_IF_CONDITION_ARM
                        | SyntaxKind::ELEMENTS_IF_CONDITION_ARM => {
                            let condition = child
                                .child_by_field("condition")
                                .map(|n| self.lower_expr(n))
                                .unwrap_or_else(|| self.error_expr(child.span()));
                            let body = child
                                .child_by_field("body")
                                .map(|n| self.lower_expr(n))
                                .unwrap_or_else(|| self.error_expr(child.span()));
                            arms.push((condition, body));
                        }
                        _ => {
                            // Check for else branch by looking for the field
                            if let Some(else_node) = node.child_by_field("else") {
                                else_expr = Some(self.lower_expr(else_node));
                            }
                        }
                    }
                }

                // Build nested if-else from the arms (in reverse order)
                // if { a => x, b => y, else => z } becomes: if a { x } else { if b { y } else { z } }
                let mut result = else_expr;
                for (condition, body) in arms.into_iter().rev() {
                    result = Some(self.alloc_expr(Expr::If {
                        condition,
                        then_branch: body,
                        else_branch: result,
                        span: node.span(),
                    }));
                }

                result.unwrap_or_else(|| self.error_expr(node.span()))
            }

            // Match expression: if scrutinee is { pattern => expr, ... }
            SyntaxKind::VALUE_IF_MATCH_EXPRESSION | SyntaxKind::ELEMENTS_IF_MATCH_EXPRESSION => {
                let scrutinee_expr = node
                    .child_by_field("scrutinee")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let mut arms: Vec<MatchArm> = Vec::new();

                for child in node.children() {
                    if matches!(
                        child.kind(),
                        SyntaxKind::VALUE_IF_MATCH_ARM | SyntaxKind::ELEMENTS_IF_MATCH_ARM
                    ) {
                        // Each arm can have multiple patterns (comma-separated)
                        let mut patterns: Vec<ExprId> = Vec::new();

                        for arm_child in child.children() {
                            if arm_child.kind() == SyntaxKind::PATTERN {
                                patterns.push(self.lower_pattern_expr(arm_child));
                            }
                        }

                        if !patterns.is_empty() {
                            arms.push(MatchArm {
                                patterns,
                                body: child
                                    .child_by_field("body")
                                    .map(|n| self.lower_expr(n))
                                    .unwrap_or_else(|| self.error_expr(child.span())),
                            });
                        }
                    }
                }

                let else_branch = node.child_by_field("else").map(|n| self.lower_expr(n));

                self.alloc_expr(Expr::Match {
                    scrutinee: scrutinee_expr,
                    arms,
                    else_branch,
                    span: node.span(),
                })
            }

            // Default: create error
            _ => self.error_expr(node.span()),
        }
    }

    /// Lowers a SyntaxNode to a statement.
    pub fn lower_stmt(&mut self, _node: SyntaxNode) -> Stmt {
        // TODO: Implement statement lowering
        // For now, return a placeholder
        Stmt::Expr(
            self.error_expr(TextSpan::new(TextSize::from(0), TextSize::from(0))),
            TextSpan::new(TextSize::from(0), TextSize::from(0)),
        )
    }

    /// Lowers a type reference.
    pub fn lower_type(&self, node: SyntaxNode) -> TypeRef {
        if node.is_error() {
            return TypeRef::name("error");
        }

        match node.kind() {
            SyntaxKind::TYPE => {
                let mut children = node.children_with_tokens();
                let Some(base_node) = children.next() else {
                    return TypeRef::name("unknown");
                };

                let mut ty = self.lower_type(base_node);
                for child in children {
                    match child.kind() {
                        SyntaxKind::QUESTION => {
                            ty = TypeRef::nullable(ty);
                        }
                        SyntaxKind::LBRACKET => {
                            // Type suffixes compose in source order, so each `[` token from a
                            // `[]` pair wraps the current type in one more array layer.
                            ty = TypeRef::array(ty);
                        }
                        _ => {}
                    }
                }

                ty
            }
            SyntaxKind::PRIMITIVE_TYPE => TypeRef::name(node.text()),
            SyntaxKind::IDENTIFIER => self.resolve_bare_property_type(node.text()),
            SyntaxKind::USER_DEFINED_TYPE => node
                .children()
                .next()
                .map(|child| self.lower_type(child))
                .unwrap_or_else(|| self.resolve_bare_property_type(node.text())),
            SyntaxKind::QUALIFIED_NAME => self.resolve_bare_property_type(node.text()),
            _ => TypeRef::name("unknown"),
        }
    }

    /// Lowers a type alias definition node.
    pub fn lower_type_alias(&self, node: SyntaxNode) -> TypeAlias {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let type_node = node.child_by_field("type").unwrap_or(node);
        let ty = self.lower_type(type_node);

        TypeAlias {
            name,
            visibility: Self::lower_visibility(node),
            ty,
            span: node.span(),
        }
    }

    /// Lowers a top-level value definition.
    pub fn lower_value_definition(&mut self, node: SyntaxNode) -> ValueDef {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let ty = node
            .child_by_field("type")
            .map(|type_node| self.lower_type(type_node));
        let value = node
            .child_by_field("value")
            .map(|value_node| self.lower_expr(value_node))
            .unwrap_or_else(|| self.error_expr(node.span()));
        let visibility = Self::lower_visibility(node);

        let value_ty = ty
            .as_ref()
            .map(TypeTag::from_type_ref)
            .unwrap_or_else(|| self.expr_type(value));
        self.define_name(&name, value_ty);

        ValueDef {
            name,
            visibility,
            ty,
            value,
            span: node.span(),
        }
    }

    /// Lowers a record definition node.
    pub fn lower_record_definition(&mut self, node: SyntaxNode) -> RecordDef {
        self.lower_record_like_definition(node, RecordKind::Plain)
    }

    /// Lowers an action definition node into a record-compatible definition.
    fn lower_action_definition(&mut self, node: SyntaxNode) -> RecordDef {
        self.lower_record_like_definition(node, RecordKind::Action)
    }

    fn lower_record_like_definition(&mut self, node: SyntaxNode, kind: RecordKind) -> RecordDef {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));

        RecordDef {
            name,
            visibility: Self::lower_visibility(node),
            kind,
            is_abstract: node.child_by_field("abstract").is_some(),
            base: node
                .child_by_field("base")
                .map(|base| Name::new(base.text())),
            properties: self.lower_record_fields_from_node(node, false),
            span: node.span(),
        }
    }

    /// Lowers a discriminated union definition node.
    pub fn lower_union_definition(&mut self, node: SyntaxNode) -> UnionDef {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let cases = node
            .child_by_field("cases")
            .map(|cases| {
                cases
                    .children()
                    .filter(|child| child.kind() == SyntaxKind::UNION_CASE)
                    .map(|case| self.lower_union_case(case))
                    .collect()
            })
            .unwrap_or_default();

        UnionDef {
            name,
            visibility: Self::lower_visibility(node),
            base: node
                .child_by_field("base")
                .map(|base| Name::new(base.text())),
            cases,
            property_target: None,
            span: node.span(),
        }
    }

    fn lower_union_case(&mut self, node: SyntaxNode) -> UnionCaseDef {
        let name = node
            .child_by_field("name")
            .map(|name| Name::new(name.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let fields = self
            .lower_record_fields_from_node(node, false)
            .into_iter()
            .map(UnionCaseField::from_record_field)
            .collect();

        UnionCaseDef {
            name,
            fields,
            span: node.span(),
        }
    }

    /// Lowers a function definition.
    ///
    /// Supports both element-style (`let <Name props... />`) and paren-style (`let name(params)`)
    /// declarations with an optional `: Type` return annotation.
    pub fn lower_function(&mut self, node: SyntaxNode) -> Function {
        let span = node.span();

        // Extract function name (`element_name` for markup functions, `identifier` for paren forms)
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("anonymous"));

        // Parse parameters from property_definition nodes
        let mut params = Vec::new();
        let mut content_param_name: Option<Name> = None;
        for child in node.children() {
            if child.kind() == SyntaxKind::PROPERTY_DEFINITION {
                let (param_name, param_type, _default, is_content) =
                    self.lower_property_definition(child);
                let param_span = child.span();

                if is_content {
                    if let Some(existing_name) = content_param_name.as_ref() {
                        self.add_diagnostic(
                            format!(
                                "Only one content property is allowed in this declaration; '{}' conflicts with '{}'",
                                param_name.as_str(),
                                existing_name.as_str()
                            ),
                            param_span,
                        );
                    } else {
                        content_param_name = Some(param_name.clone());
                    }
                }

                params.push(Param::with_content(
                    param_name, param_type, is_content, param_span,
                ));

                // Note: Default values are part of property_definition grammar
                // but we don't store them in Param yet (future enhancement)
            }
        }

        // Track parameter types in a new scope so expression lowering can infer operand kinds.
        self.push_scope();
        for param in &params {
            let ty = TypeTag::from_type_ref(&param.ty);
            self.define_name(&param.name, ty);
        }

        // Lower the optional return type annotation if present
        let return_type = node
            .child_by_field("return_type")
            .map(|n| self.lower_type(n));

        // Lower the body expression
        let body = node
            .child_by_field("body")
            .map(|n| self.lower_expr(n))
            .unwrap_or_else(|| self.error_expr(span));

        self.pop_scope();

        Function {
            name,
            visibility: Self::lower_visibility(node),
            params,
            return_type,
            body,
            span,
        }
    }

    /// Recursively extracts element body-content expressions from parsed content nodes.
    ///
    /// Content can be wrapped in element/text containers. This preserves expression-producing
    /// body content instead of only literal nested elements.
    fn lower_element_content(&mut self, node: SyntaxNode, content: &mut Vec<ExprId>) {
        match node.kind() {
            SyntaxKind::ELEMENT
            | SyntaxKind::TEXT_CHILD_ELEMENT
            | SyntaxKind::VALUES_BRACED_EXPRESSION
            | SyntaxKind::ELEMENTS_BRACED_EXPRESSION
            | SyntaxKind::EMBED_BRACED_EXPRESSION
            | SyntaxKind::VALUE_IF_EXPRESSION
            | SyntaxKind::VALUE_IF_SIMPLE_EXPRESSION
            | SyntaxKind::VALUE_IF_MATCH_EXPRESSION
            | SyntaxKind::VALUE_IF_CONDITION_LIST_EXPRESSION
            | SyntaxKind::VALUE_FOR_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_SIMPLE_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_MATCH_EXPRESSION
            | SyntaxKind::ELEMENTS_IF_CONDITION_LIST_EXPRESSION
            | SyntaxKind::ELEMENTS_FOR_EXPRESSION => {
                content.push(self.lower_expr(node));
            }
            // These containers just group body content, so recurse into their syntax children.
            SyntaxKind::MIXED_CONTENT
            | SyntaxKind::ELEMENTS_EXPRESSION
            | SyntaxKind::CONTENT
            | SyntaxKind::TEXT_CONTENT
            | SyntaxKind::EMBED_TEXT_CONTENT => {
                for child in node.children() {
                    self.lower_element_content(child, content);
                }
            }
            SyntaxKind::TEXT_RUN | SyntaxKind::RAW_TEXT_RUN => {
                // Text in a body is a string literal like any other, so it is allocated the same
                // way — one place records a literal's span, and a literal cannot arrive without
                // one. Nothing reads this span yet: an offset in element content resolves to no
                // expression today (`specs/future.md`, unchecked element content). The type is
                // lowering-local and inert here, since a text run is never an operand.
                let text = node.text();
                if !text.trim().is_empty() {
                    content.push(self.literal_expr(
                        Literal::String(SmolStr::new(text)),
                        TypeTag::String,
                        node.span(),
                    ));
                }
            }
            _ => {}
        }
    }

    fn lower_property_value(
        &mut self,
        child: SyntaxNode,
        component: Option<&PredeclaredComponent>,
    ) -> Property {
        let key = child
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("_"));

        let value_node = Self::property_value_node(child);
        let value = if let Some(component) = component {
            let prop_name = key.as_str();
            let is_declared_prop = component
                .effective_props
                .iter()
                .any(|prop| prop.name == key);

            if !is_declared_prop && Self::is_handler_binding_candidate(prop_name) {
                if let Some(HandlerPropResolution::Emit(emit)) = component.handler_props.get(&key) {
                    let body = if let Some(value_node) = value_node {
                        self.push_scope();
                        let action_name = Name::new("action");
                        self.define_name(&action_name, TypeTag::Unknown);
                        let body = self.lower_expr(value_node);
                        self.pop_scope();
                        body
                    } else {
                        self.error_expr(child.span())
                    };

                    self.alloc_expr(Expr::ActionHandler {
                        component: component.name.clone(),
                        emit: emit.name.clone(),
                        action_name: emit.action_name.clone(),
                        // A binding lowered from source reaches a component declared in this same
                        // module, so the emit it names is declared here too.
                        action_module_identity: None,
                        owner: self.current_component.clone(),
                        body,
                        span: child.span(),
                    })
                } else if matches!(
                    component.handler_props.get(&key),
                    Some(HandlerPropResolution::Collision)
                ) {
                    self.add_diagnostic(
                        format!(
                            "Component '{}' cannot use '{}' because it collides with a declared prop or duplicate emitted action",
                            component.name.as_str(),
                            prop_name
                        ),
                        child.span(),
                    );
                    self.lower_value_or_error(value_node, child.span())
                } else {
                    self.add_diagnostic(
                        format!(
                            "Component '{}' does not emit '{}' required by handler '{}'",
                            component.name.as_str(),
                            &prop_name[2..],
                            prop_name
                        ),
                        child.span(),
                    );
                    self.lower_value_or_error(value_node, child.span())
                }
            } else {
                self.lower_value_or_error(value_node, child.span())
            }
        } else {
            self.lower_value_or_error(value_node, child.span())
        };

        Property {
            key,
            value,
            span: child.span(),
        }
    }

    fn lower_property_entries(
        &mut self,
        node: SyntaxNode,
        component: Option<&PredeclaredComponent>,
    ) -> Vec<PropertyEntry> {
        node.children()
            .filter_map(|child| self.lower_property_entry(child, component))
            .collect()
    }

    fn lower_property_entry(
        &mut self,
        child: SyntaxNode,
        component: Option<&PredeclaredComponent>,
    ) -> Option<PropertyEntry> {
        match child.kind() {
            SyntaxKind::PROPERTY_VALUE => Some(PropertyEntry::Value(
                self.lower_property_value(child, component),
            )),
            SyntaxKind::PROPERTY_LIST_IF_EXPRESSION => child
                .children()
                .find_map(|nested| self.lower_property_entry(nested, component)),
            SyntaxKind::PROPERTY_LIST_IF_SIMPLE_EXPRESSION => {
                let condition = child
                    .child_by_field("condition")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(child.span()));
                let then_entries = child
                    .child_by_field("then")
                    .map(|n| self.lower_property_entries(n, component))
                    .unwrap_or_default();
                let else_entries = child
                    .child_by_field("else")
                    .map(|n| self.lower_property_entries(n, component))
                    .unwrap_or_default();

                Some(PropertyEntry::If {
                    condition,
                    then_entries,
                    else_entries,
                    span: child.span(),
                })
            }
            SyntaxKind::PROPERTY_LIST_IF_CONDITION_LIST_EXPRESSION => {
                let mut arms = Vec::new();
                for arm_node in child.children() {
                    if arm_node.kind() != SyntaxKind::PROPERTY_LIST_IF_CONDITION_ARM {
                        continue;
                    }

                    let condition = arm_node
                        .child_by_field("condition")
                        .map(|n| self.lower_expr(n))
                        .unwrap_or_else(|| self.error_expr(arm_node.span()));
                    let entries = arm_node
                        .child_by_field("body")
                        .map(|n| self.lower_property_entries(n, component))
                        .unwrap_or_default();
                    arms.push(PropertyConditionArm {
                        condition,
                        entries,
                        span: arm_node.span(),
                    });
                }

                let else_entries = child
                    .child_by_field("else")
                    .map(|n| self.lower_property_entries(n, component))
                    .unwrap_or_default();

                Some(PropertyEntry::ConditionList {
                    arms,
                    else_entries,
                    span: child.span(),
                })
            }
            SyntaxKind::PROPERTY_LIST_IF_MATCH_EXPRESSION => {
                let scrutinee = child
                    .child_by_field("scrutinee")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(child.span()));
                let mut arms = Vec::new();

                for arm_node in child.children() {
                    if arm_node.kind() != SyntaxKind::PROPERTY_LIST_IF_MATCH_ARM {
                        continue;
                    }

                    let mut patterns = Vec::new();
                    for pattern_node in arm_node.children() {
                        if pattern_node.kind() == SyntaxKind::PATTERN {
                            patterns.push(self.lower_pattern_expr(pattern_node));
                        }
                    }

                    let entries = arm_node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::PROPERTY_LIST)
                        .map(|n| self.lower_property_entries(n, component))
                        .unwrap_or_default();

                    if !patterns.is_empty() {
                        arms.push(PropertyMatchArm {
                            patterns,
                            entries,
                            span: arm_node.span(),
                        });
                    }
                }

                let else_entries = child
                    .child_by_field("else")
                    .map(|n| self.lower_property_entries(n, component))
                    .unwrap_or_default();

                Some(PropertyEntry::Match {
                    scrutinee,
                    arms,
                    else_entries,
                    span: child.span(),
                })
            }
            _ => None,
        }
    }

    /// Lowers an element.
    ///
    /// Parses: `<tag prop1=val1 prop2={expr}>...body content...</tag>`
    /// Or self-closing: `<tag prop1=val1 />`
    pub fn lower_element(&mut self, node: SyntaxNode) -> Element {
        let span = node.span();

        // Extract tag name
        let tag = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let tag = self.resolve_bare_update_tag(tag);
        let component = self.find_predeclared_component(tag.as_str()).cloned();

        // Parse properties from property_list.
        let mut property_entries = Vec::new();
        if let Some(prop_list) = node.child_by_field("properties") {
            property_entries = self.lower_property_entries(prop_list, component.as_ref());
        }
        let properties = property_entries
            .iter()
            .filter_map(|entry| match entry {
                PropertyEntry::Value(property) => Some(property.clone()),
                _ => None,
            })
            .collect();

        // Parse body content expressions.
        let mut content = Vec::new();
        if let Some(content_node) = node.child_by_field("content") {
            self.lower_element_content(content_node, &mut content);
        }

        // Extract closing tag name for validation
        let close_name = node
            .child_by_field("close_name")
            .map(|n| Name::new(n.text()));

        Element {
            tag,
            properties,
            property_entries,
            content,
            close_name,
            span,
        }
    }

    /// Rewrites a bare `Property` at the head of a member access to the enclosing component's
    /// `<Component>.Property`, so `Property.name` lowers exactly as `Counter.Property.name` does.
    ///
    /// <para>Outside a component the name is left as written, so a declaration named `Property`
    /// still resolves and type checking reports the bare form when nothing does. A bare `Property`
    /// that is not the head of a member access is left alone: a union's name is not a value.</para>
    fn resolve_bare_property_base(&self, parts: &mut Vec<Name>) {
        if parts.len() < 2 || parts[0].as_str() != PROPERTY_UNION_SUFFIX {
            return;
        }
        if let Some(component) = &self.current_component {
            parts.insert(0, component.clone());
        }
    }

    /// Returns true when `node` is the identifier `Property` inside a component, which as the
    /// object of a member access names the component's property union.
    fn is_bare_property_base(&self, node: SyntaxNode) -> bool {
        self.current_component.is_some()
            && Self::bare_identifier_text(node).as_deref() == Some(PROPERTY_UNION_SUFFIX)
    }

    /// The identifier a value expression is, when it is nothing more than one.
    ///
    /// <para>The parser wraps an identifier written as a value in expression nodes; this looks
    /// through those wrappers and returns `None` for anything with structure of its own.</para>
    fn bare_identifier_text(node: SyntaxNode) -> Option<String> {
        match node.kind() {
            SyntaxKind::IDENTIFIER => Some(node.text().trim().to_string()),
            SyntaxKind::VALUE_EXPRESSION
            | SyntaxKind::VALUE_LIST_ITEM_EXPRESSION
            | SyntaxKind::IDENTIFIER_EXPRESSION => {
                let mut children = node.children();
                let child = children.next()?;
                if children.next().is_some() {
                    return None;
                }
                Self::bare_identifier_text(child)
            }
            _ => None,
        }
    }

    /// Lowers the bare `Property` object of a member access to `<Component>.Property`, as the
    /// qualified spelling lowers.
    fn lower_bare_property_base(&mut self, object: SyntaxNode) -> ExprId {
        let component = self
            .current_component
            .clone()
            .expect("a bare Property base is only recognized inside a component");
        let base = self.alloc_expr(Expr::Ident(component.clone()));
        self.module.set_expr_span(base, object.span());
        let ty = self.lookup_name(&component);
        self.set_expr_type(base, ty);
        self.alloc_expr(Expr::Member {
            base,
            member: Name::new(PROPERTY_UNION_SUFFIX),
            span: object.span(),
        })
    }

    /// Resolves a bare `Property` type reference to the enclosing component's property union.
    fn resolve_bare_property_type(&self, text: &str) -> TypeRef {
        match &self.current_component {
            Some(component) if text == PROPERTY_UNION_SUFFIX => {
                TypeRef::name(property_union_name(component.as_str()))
            }
            _ => TypeRef::name(text),
        }
    }

    /// Resolves a bare `Update` tag to the enclosing component's `<Component>.Update` record.
    ///
    /// <para>Inside a component the bare name always means that component's update record, even
    /// when the module declares something else called `Update`: a handler's meaning should not
    /// depend on what else happens to be declared. Resolving it here makes the bare form
    /// indistinguishable from the qualified one in every later phase. Outside a component the tag
    /// is left as written, so a declaration named `Update` still resolves, and type checking
    /// reports the bare form when nothing does.</para>
    ///
    /// <para>A component without state has no update record, so the qualified name resolves to
    /// nothing; the checker reports that once, as it does for any `X.Update` with no record.</para>
    fn resolve_bare_update_tag(&self, tag: Name) -> Name {
        if tag.as_str() != UPDATE_RECORD_SUFFIX {
            return tag;
        }
        match &self.current_component {
            Some(component) => update_record_name(component.as_str()),
            None => tag,
        }
    }

    /// Lowers a module (source file).
    pub fn lower_module(&mut self, root: SyntaxNode) {
        self.predeclare_record_update_records(root);
        self.predeclare_components(root);

        // Process all top-level items
        for child in root.children() {
            match child.kind() {
                SyntaxKind::IMPORT_STATEMENT => {
                    if let Some(import) = self.lower_import_statement(child) {
                        self.module.imports.push(import);
                    }
                }
                SyntaxKind::FUNCTION_DEFINITION => {
                    let func = self.lower_function(child);
                    self.module.add_item(Item::Function(func));
                }
                SyntaxKind::VALUE_DEFINITION => {
                    let value = self.lower_value_definition(child);
                    self.module.add_item(Item::Value(value));
                }
                SyntaxKind::COMPONENT_DEFINITION => {
                    let component = self.lower_component_definition(child);
                    let inline_emit_records = self
                        .component_emit_records
                        .get(&component.name)
                        .cloned()
                        .unwrap_or_default();
                    let component_name = component.name.clone();
                    self.module.add_item(Item::Component(component));
                    for record in inline_emit_records {
                        let action_name = record.name.clone();
                        self.module.add_item(Item::Record(record));
                        self.add_derived_items(&action_name);
                    }
                    self.add_derived_items(&component_name);
                }
                SyntaxKind::TYPE_DEFINITION => {
                    let alias = self.lower_type_alias(child);
                    self.module.add_item(Item::TypeAlias(alias));
                }
                SyntaxKind::RECORD_DEFINITION => {
                    let record = self.lower_record_definition(child);
                    let name = record.name.clone();
                    self.module.add_item(Item::Record(record));
                    self.add_derived_items(&name);
                }
                SyntaxKind::ACTION_DEFINITION => {
                    let action = self.lower_action_definition(child);
                    let name = action.name.clone();
                    self.module.add_item(Item::Record(action));
                    self.add_derived_items(&name);
                }
                SyntaxKind::UNION_DEFINITION => {
                    let union_def = self.lower_union_definition(child);
                    self.module.add_item(Item::Union(union_def));
                }
                SyntaxKind::ELEMENT => {
                    // Top-level element becomes an implicit 'root' function
                    let span = child.span();
                    let element = self.lower_element(child);
                    let element_id = self.module.alloc_element(element);

                    // Create an Expr::Element that references the element
                    let body = self.alloc_expr(Expr::Element {
                        element: element_id,
                        span,
                    });

                    // Create the implicit 'root' function. This entry point must remain
                    // discoverable by the runtime even though omitted source visibility
                    // now lowers to internal visibility.
                    let root_func = Function {
                        name: Name::new("root"),
                        visibility: Visibility::Export,
                        params: vec![],
                        return_type: None,
                        body,
                        span,
                    };

                    self.module.add_item(Item::Function(root_func));
                }
                _ => {
                    // Skip other node types for now
                }
            }
        }

        for record in std::mem::take(&mut self.pending_update_items) {
            self.module.add_item(Item::Record(record));
        }
        for union in std::mem::take(&mut self.pending_property_items) {
            self.module.add_item(Item::Union(union));
        }
    }
}

/// Lower a CST root node to a HIR LoweredModule.
pub fn lower(root: SyntaxNode, source_id: SourceId) -> LoweredModule {
    let mut ctx = LoweringContext::new(source_id);
    ctx.lower_module(root);
    ctx.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        effective_record_shape, validate_record_definitions, validate_union_definitions,
        PreparedItemKind, PreparedModule, PreparedNamespace,
    };
    use nx_syntax::parse_str;
    mod tree_helpers {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../nx-syntax/tests/tree_helpers.rs"
        ));
    }
    use tree_helpers::{collect_kinds, find_first_kind};

    fn prepared_record_validation_messages(module: &LoweredModule) -> Vec<String> {
        let prepared = PreparedModule::standalone("record-validation.nx", module.clone());
        validate_record_definitions(&prepared)
            .into_iter()
            .map(|error| error.message())
            .collect()
    }

    fn prepared_union_validation_messages(module: &LoweredModule) -> Vec<String> {
        let prepared = PreparedModule::standalone("union-validation.nx", module.clone());
        validate_union_definitions(&prepared)
            .into_iter()
            .map(|error| error.message())
            .collect()
    }

    #[test]
    fn test_lowering_context_creation() {
        let ctx = LoweringContext::new(SourceId::new(0));
        let module = ctx.finish();
        assert_eq!(module.items().len(), 0);
        assert!(module.imports.is_empty());
    }

    /// A literal is located by the span map like every other expression. Without an entry there
    /// it cannot be found by offset at all, which is what kept an editor from reporting its type.
    #[test]
    fn literals_record_the_span_they_were_written_at() {
        let source =
            "let s = \"hi\"\nlet i = 42\nlet f = 1.5\nlet b = true\nlet n = null\nlet m = -42\n";
        let tree = parse_str(source, "literals.nx")
            .tree
            .expect("Should parse literals");
        let module = lower(tree.root(), SourceId::new(0));

        let located = module
            .exprs()
            .filter(|(_, expr)| matches!(expr, Expr::Literal(_)))
            .map(|(id, expr)| (expr.clone(), module.expr_span(id)))
            .collect::<Vec<_>>();

        assert_eq!(located.len(), 6, "got: {located:?}");
        for (expr, span) in &located {
            assert!(
                !span.is_empty(),
                "{expr:?} carries no span, so no offset can reach it"
            );
            assert_eq!(
                &source[usize::from(span.start())..usize::from(span.end())],
                match expr {
                    Expr::Literal(Literal::String(value)) => format!("\"{value}\""),
                    Expr::Literal(Literal::Int(value)) => value.to_string(),
                    Expr::Literal(Literal::Float(value)) => format!("{}", value.0),
                    Expr::Literal(Literal::Boolean(value)) => value.to_string(),
                    Expr::Literal(Literal::Null) => "null".to_string(),
                    other => panic!("unexpected literal {other:?}"),
                },
                "the span does not cover what was written"
            );
        }
    }

    /// Folding `-` into the literal rewrites the operand rather than leaving it behind. A discarded
    /// operand would still be findable by offset, and being the narrower of the two it would shadow
    /// the folded literal — which is the one the checker typed.
    #[test]
    fn a_folded_negative_literal_leaves_no_unnegated_expression_behind() {
        let source = "let m = -42\n";
        let tree = parse_str(source, "negative.nx")
            .tree
            .expect("Should parse a negative literal");
        let module = lower(tree.root(), SourceId::new(0));

        let literals = module
            .exprs()
            .filter(|(_, expr)| matches!(expr, Expr::Literal(_)))
            .map(|(id, expr)| (expr.clone(), module.expr_span(id)))
            .collect::<Vec<_>>();

        assert_eq!(literals.len(), 1, "got: {literals:?}");
        assert!(matches!(literals[0].0, Expr::Literal(Literal::Int(-42))));
        assert_eq!(&source[8..11], "-42");
        assert_eq!(literals[0].1, TextSpan::new(8.into(), 11.into()));
    }

    #[test]
    fn test_lower_wildcard_imports() {
        let source = r#"import "./ui"
import "./controls" as UI
<UI.Root />"#;
        let parse_result = parse_str(source, "imports.nx");

        let tree = parse_result.tree.expect("Should parse wildcard imports");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.imports.len(), 2);

        let first = &module.imports[0];
        assert_eq!(first.library_path, "./ui");
        match &first.kind {
            ImportKind::Wildcard { alias } => {
                assert!(alias.is_none());
            }
            other => panic!("Expected wildcard import, got {:?}", other),
        }

        let second = &module.imports[1];
        assert_eq!(second.library_path, "./controls");
        match &second.kind {
            ImportKind::Wildcard { alias } => {
                assert_eq!(
                    alias.as_ref().map(Name::as_str),
                    Some("UI"),
                    "Namespace alias should lower from wildcard import"
                );
            }
            other => panic!("Expected wildcard import, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_selective_imports() {
        let source = r#"import { Button, Stack as Layout.Stack } from "https://example.com/ui.zip"
<Button />"#;
        let parse_result = parse_str(source, "selective-imports.nx");

        let tree = parse_result.tree.expect("Should parse selective imports");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.imports.len(), 1);
        let import = &module.imports[0];
        assert_eq!(import.library_path, "https://example.com/ui.zip");
        match &import.kind {
            ImportKind::Selective { entries } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].name.as_str(), "Button");
                assert!(entries[0].qualifier.is_none());
                assert_eq!(entries[1].name.as_str(), "Stack");
                assert_eq!(
                    entries[1].qualifier.as_ref().map(Name::as_str),
                    Some("Layout")
                );
            }
            other => panic!("Expected selective import, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_invalid_selective_import_alias_reports_diagnostic() {
        let source = r#"import { Stack as Layout.Panel } from "../layout"
<Stack />"#;
        let parse_result = parse_str(source, "invalid-selective-import.nx");

        let tree = parse_result
            .tree
            .expect("Invalid selective alias should still parse");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.imports.len(), 1);
        let import = &module.imports[0];
        match &import.kind {
            ImportKind::Selective { entries } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].name.as_str(), "Stack");
                assert!(entries[0].qualifier.is_none());
            }
            other => panic!("Expected selective import, got {:?}", other),
        }

        assert!(
            module
                .diagnostics()
                .iter()
                .any(|diag| diag.message.contains("must end with 'Stack'")),
            "Expected invalid selective alias diagnostic, got {:?}",
            module.diagnostics()
        );
    }

    #[test]
    fn test_lower_visibility_modifiers() {
        let source = r#"private let footerText: string = "Built with NX"
private action SaveRequested = {}
export component <SearchBox /> = { <input /> }
type Theme = string
private let <Render /> = <div />
type Mode = light | dark"#;
        let parse_result = parse_str(source, "visibility.nx");

        let tree = parse_result.tree.expect("Visibility source should parse");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let save_requested = module
            .find_item("SaveRequested")
            .expect("Expected lowered action record");
        assert_eq!(save_requested.visibility(), Visibility::Private);

        let footer_text = module
            .find_item("footerText")
            .expect("Expected lowered top-level value");
        assert_eq!(footer_text.visibility(), Visibility::Private);

        let search_box = module
            .find_item("SearchBox")
            .expect("Expected lowered component");
        assert_eq!(search_box.visibility(), Visibility::Export);

        let theme = module
            .find_item("Theme")
            .expect("Expected lowered type alias");
        assert_eq!(theme.visibility(), Visibility::Internal);

        let render = module
            .find_item("Render")
            .expect("Expected lowered function");
        assert_eq!(render.visibility(), Visibility::Private);

        let mode = module.find_item("Mode").expect("Expected lowered enum");
        assert_eq!(mode.visibility(), Visibility::Internal);
    }

    #[test]
    fn test_lower_top_level_value_definition() {
        let source = r#"let title: string = "NX""#;
        let parse_result = parse_str(source, "value.nx");
        let tree = parse_result.tree.expect("Value source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let value = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Value(value) => Some(value),
                _ => None,
            })
            .expect("Expected lowered top-level value");

        assert_eq!(value.name.as_str(), "title");
        assert_eq!(value.visibility, Visibility::Internal);
        assert!(
            matches!(value.ty.as_ref(), Some(TypeRef::Name(name)) if name.as_str() == "string")
        );
    }

    #[test]
    fn test_lower_exported_top_level_value_definition() {
        let source = r#"export let title: string = "NX""#;
        let parse_result = parse_str(source, "value.nx");
        let tree = parse_result
            .tree
            .expect("Exported value source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let value = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Value(value) => Some(value),
                _ => None,
            })
            .expect("Expected lowered exported top-level value");

        assert_eq!(value.name.as_str(), "title");
        assert_eq!(value.visibility, Visibility::Export);
    }

    #[test]
    fn test_lower_simple_function() {
        // Parse a simple function definition
        let source = "let <Button text:string /> = <button>{text}</button>";
        let parse_result = parse_str(source, "test.nx");

        assert!(parse_result.tree.is_some());
        let tree = parse_result.tree.unwrap();
        let root = tree.root();

        // Lower to HIR
        let module = lower(root, SourceId::new(0));

        // Should have one function item
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 1);
                assert_eq!(func.params[0].name.as_str(), "text");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_function_with_multiple_params() {
        let source = "let <Button text:string disabled:boolean /> = <button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.params[0].name.as_str(), "text");
                assert_eq!(func.params[1].name.as_str(), "disabled");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_paren_function_with_return_type() {
        let source = "let add(a:int, b:int): int = { a + b }";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "add");
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.params[0].name.as_str(), "a");
                assert_eq!(func.params[1].name.as_str(), "b");

                let ret = func
                    .return_type
                    .as_ref()
                    .expect("Function should capture return type annotation");
                match ret {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
                    _ => panic!("Expected simple return type"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_element_function_with_return_type() {
        let source = r#"let <Button text:string />: Element = <button>{text}</button>"#;
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 1);

                let ret = func
                    .return_type
                    .as_ref()
                    .expect("Element-style function should retain return type");
                match ret {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "Element"),
                    _ => panic!("Expected simple return type"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_content_metadata_across_function_record_and_component_surfaces() {
        let source = r#"
            type Note = { title:string content body:string }
            let Wrap(title:string, content body:Element) = <section>{body}</section>
            component <Panel title:string content body:Element emits { Submitted { content payload:string } } /> = {
                state { content current:Element }
                <section>{body}</section>
            }
        "#;
        let parse_result = parse_str(source, "content-metadata.nx");
        let tree = parse_result
            .tree
            .expect("Should parse content metadata source");
        let module = lower(tree.root(), SourceId::new(0));

        let record = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(record) if record.name.as_str() == "Note" => Some(record),
                _ => None,
            })
            .expect("Expected record definition");
        assert_eq!(
            record.content_property().map(|field| field.name.as_str()),
            Some("body")
        );

        let function = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "Wrap" => Some(function),
                _ => None,
            })
            .expect("Expected function definition");
        assert_eq!(
            function.content_param().map(|param| param.name.as_str()),
            Some("body")
        );

        let component = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "Panel" => Some(component),
                _ => None,
            })
            .expect("Expected component definition");
        assert_eq!(
            component.content_prop().map(|field| field.name.as_str()),
            Some("body")
        );
        assert_eq!(
            component
                .state
                .iter()
                .find(|field| field.is_content)
                .map(|field| field.name.as_str()),
            Some("current")
        );

        let emitted_action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(record) if record.name.as_str() == "Panel.Submitted" => Some(record),
                _ => None,
            })
            .expect("Expected inline emitted action record");
        assert_eq!(
            emitted_action
                .content_property()
                .map(|field| field.name.as_str()),
            Some("payload")
        );
    }

    #[test]
    fn test_lower_duplicate_content_property_in_single_record_diagnostic() {
        let source = r#"
            type Foo = {
              content title:string
              content body:string
            }
        "#;
        let parse_result = parse_str(source, "duplicate-record-content.nx");
        let tree = parse_result
            .tree
            .expect("Should parse duplicate record content source");
        let module = lower(tree.root(), SourceId::new(0));

        let messages: Vec<_> = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("Only one content property is allowed")),
            "Expected duplicate content property diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_type_alias_and_union() {
        let source = r#"
            type UserId = string
            type Direction = north | south | east | west
        "#;
        let parse_result = parse_str(source, "types.nx");
        let tree = parse_result.tree.expect("Should parse enum/type defs");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 2);

        match &module.items()[0] {
            Item::TypeAlias(alias) => {
                assert_eq!(alias.name.as_str(), "UserId");
            }
            other => panic!("Expected type alias, got {:?}", other),
        }

        match &module.items()[1] {
            Item::Union(union_def) => {
                assert_eq!(union_def.name.as_str(), "Direction");
                assert!(
                    union_def.is_constant_union(),
                    "an enum lowers to a constant union"
                );
                let names: Vec<_> = union_def
                    .cases
                    .iter()
                    .map(|case| case.name.as_str())
                    .collect();
                assert!(names.contains(&"north"));
                assert!(names.contains(&"west"));
            }
            other => panic!("Expected union definition, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_union_definition() {
        let source = r#"
            abstract type EventBase = { source:string = "ui" }
            export type UiEvent extends EventBase =
              | clicked {
                  x:int
                  y:int
                  retryable:boolean = true
                }
              | closed
        "#;
        let parse_result = parse_str(source, "union-hir.nx");
        assert!(
            parse_result.is_ok(),
            "Union source should parse: {:?}",
            parse_result.errors
        );
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));

        let union = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Union(union) => Some(union),
                _ => None,
            })
            .expect("Expected union item");

        assert_eq!(union.name.as_str(), "UiEvent");
        assert_eq!(union.visibility, Visibility::Export);
        assert_eq!(
            union.base.as_ref().map(|name| name.as_str()),
            Some("EventBase")
        );
        assert_eq!(union.cases.len(), 2);
        assert_eq!(union.cases[0].name.as_str(), "clicked");
        assert_eq!(union.cases[0].fields.len(), 3);
        assert_eq!(union.cases[0].fields[0].name.as_str(), "x");
        assert!(
            union.cases[0].fields[2].default.is_some(),
            "Case field default should be preserved"
        );
        assert_eq!(union.cases[1].name.as_str(), "closed");
        assert!(union.cases[1].is_fieldless());
    }

    #[test]
    fn test_prepared_union_binds_only_union_name_in_type_namespace() {
        let source = "export type LoadState = idle | failed { message:string }";
        let parse_result = parse_str(source, "union-binding.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let prepared = PreparedModule::standalone("union-binding.nx", module);
        let union_name = Name::new("LoadState");
        let case_name = Name::new("LoadState.idle");

        let binding = prepared
            .resolve_binding(PreparedNamespace::Type, &union_name)
            .expect("Union name should bind in type namespace");
        assert_eq!(binding.kind, PreparedItemKind::Union);
        assert!(
            prepared
                .resolve_binding(PreparedNamespace::Element, &union_name)
                .is_none(),
            "Union name should not bind as an element constructor"
        );
        assert!(
            prepared
                .resolve_binding(PreparedNamespace::Type, &case_name)
                .is_none(),
            "Cases should not be exposed as top-level type bindings"
        );
        assert!(
            prepared
                .resolve_binding(PreparedNamespace::Element, &case_name)
                .is_none(),
            "Cases should not be exposed as top-level element bindings"
        );
    }

    #[test]
    fn test_validate_union_base_must_be_abstract_record() {
        let source = r#"
            type Concrete = { source:string }
            type BadUnion extends Concrete = | failed { message:string }
        "#;
        let parse_result = parse_str(source, "bad-union-base.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let messages = prepared_union_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("only abstract records may be extended")),
            "Expected invalid union base diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_validate_union_base_cannot_be_union() {
        let source = r#"
            type LoadState = | idle
            type MoreLoadState extends LoadState = | failed { message:string }
        "#;
        let parse_result = parse_str(source, "union-extends-union.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let messages = prepared_union_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("does not resolve to an abstract record")),
            "Expected union-base diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_validate_union_rejects_inherited_case_field_collision() {
        let source = r#"
            abstract type EventBase = { source:string }
            type UiEvent extends EventBase = | clicked { source:string x:int }
        "#;
        let parse_result = parse_str(source, "union-inherited-collision.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let messages = prepared_union_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("redeclares inherited field 'source'")),
            "Expected inherited-field collision diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_validate_union_inherited_content_field_collision_reports_once() {
        let source = r#"
            abstract type EventBase = { content body:string }
            type UiEvent extends EventBase = | clicked { content body:string }
        "#;
        let parse_result = parse_str(source, "union-inherited-content-collision.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let messages = prepared_union_validation_messages(&module);

        let inherited_field_count = messages
            .iter()
            .filter(|message| message.contains("redeclares inherited field 'body'"))
            .count();
        let content_property_count = messages
            .iter()
            .filter(|message| message.contains("already the content property"))
            .count();

        assert_eq!(
            inherited_field_count, 1,
            "Expected one inherited-field diagnostic, got {:?}",
            messages
        );
        assert_eq!(
            content_property_count, 0,
            "Inherited content field collision should not also report a content-property diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_validate_union_rejects_duplicate_case_content_fields() {
        let source = r#"
            type LoadState = | failed {
              content message:string
              content details:string
            }
        "#;
        let parse_result = parse_str(source, "union-content-collision.nx");
        assert!(parse_result.is_ok(), "Union source should parse");
        let tree = parse_result.tree.expect("Should parse union source");
        let module = lower(tree.root(), SourceId::new(0));
        let messages = prepared_union_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("already the content property")),
            "Expected content-field collision diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_record_definition() {
        let source = r#"
            type User = {
              name: string
              age: int = 0
            }
        "#;
        let parse_result = parse_str(source, "record.nx");
        let tree = parse_result.tree.expect("Should parse record");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let record = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) => Some(def),
                _ => None,
            })
            .expect("Should lower record definition");

        assert_eq!(record.name.as_str(), "User");
        assert_eq!(record.kind, RecordKind::Plain);
        assert_eq!(record.properties.len(), 2);
        let defaults = record
            .properties
            .iter()
            .filter(|f| f.default.is_some())
            .count();
        assert_eq!(defaults, 1, "One field should carry a default value");
    }

    #[test]
    fn test_lower_record_inheritance_metadata() {
        let source = r#"
            abstract type Entity = {
              id: int
            }

            abstract type UserBase extends Entity = {
              name: string
            }

            type User extends UserBase = {
              email: string?
            }
        "#;
        let parse_result = parse_str(source, "record-inheritance.nx");
        let tree = parse_result.tree.expect("Should parse record inheritance");
        let module = lower(tree.root(), SourceId::new(0));

        let records: Vec<_> = module
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Record(def) if def.update_target().is_none() => Some(def),
                _ => None,
            })
            .collect();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].name.as_str(), "Entity");
        assert!(records[0].is_abstract);
        assert!(records[0].base.is_none());

        assert_eq!(records[1].name.as_str(), "UserBase");
        assert!(records[1].is_abstract);
        assert_eq!(
            records[1].base.as_ref().map(|name| name.as_str()),
            Some("Entity")
        );

        assert_eq!(records[2].name.as_str(), "User");
        assert!(!records[2].is_abstract);
        assert_eq!(
            records[2].base.as_ref().map(|name| name.as_str()),
            Some("UserBase")
        );
    }

    #[test]
    fn test_lower_action_inheritance_metadata() {
        let source = r#"
            abstract action InputAction = {
              source: string
            }

            abstract action SearchAction extends InputAction = {
              query: string
            }

            action SearchSubmitted extends SearchAction = {
              submittedAt: string
            }
        "#;
        let parse_result = parse_str(source, "action-inheritance.nx");
        let tree = parse_result.tree.expect("Should parse action inheritance");
        let module = lower(tree.root(), SourceId::new(0));

        let actions: Vec<_> = module
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Record(def) if def.kind == RecordKind::Action => Some(def),
                _ => None,
            })
            .collect();

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].name.as_str(), "InputAction");
        assert!(actions[0].is_abstract);
        assert!(actions[0].base.is_none());

        assert_eq!(actions[1].name.as_str(), "SearchAction");
        assert!(actions[1].is_abstract);
        assert_eq!(
            actions[1].base.as_ref().map(|name| name.as_str()),
            Some("InputAction")
        );

        assert_eq!(actions[2].name.as_str(), "SearchSubmitted");
        assert!(!actions[2].is_abstract);
        assert_eq!(
            actions[2].base.as_ref().map(|name| name.as_str()),
            Some("SearchAction")
        );
    }

    #[test]
    fn test_lower_action_inheritance_ancestry() {
        let source = r#"
            abstract action InputAction = {
              source: string
            }

            type InputActionBase = InputAction

            abstract action SearchAction extends InputActionBase = {
              query: string
            }

            action SearchSubmitted extends SearchAction = {
              submittedAt: string
            }
        "#;
        let parse_result = parse_str(source, "action-ancestry.nx");
        let tree = parse_result.tree.expect("Should parse action inheritance");
        let module = lower(tree.root(), SourceId::new(0));
        let prepared = PreparedModule::standalone("action-ancestry.nx", module.clone());

        let action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) if def.name.as_str() == "SearchSubmitted" => Some(def),
                _ => None,
            })
            .expect("Should lower SearchSubmitted action");

        let shape = effective_record_shape(&prepared, action).expect("Action shape should resolve");
        let ancestors: Vec<_> = shape
            .ancestors
            .iter()
            .map(|ancestor| ancestor.name.as_str())
            .collect();
        assert_eq!(ancestors, vec!["SearchAction", "InputAction"]);
    }

    #[test]
    fn test_lower_record_inheritance_validation_diagnostics() {
        let source = r#"
            abstract type Entity = {
              id: int
            }

            type User extends Entity = {
              name: string
            }

            type Admin extends User = {
              level: int
            }

            abstract type DuplicateBase = {
              name: string
            }

            type DuplicateUser extends DuplicateBase = {
              name: string
            }
        "#;
        let parse_result = parse_str(source, "record-inheritance-errors.nx");
        let tree = parse_result
            .tree
            .expect("Should parse record inheritance error source");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer record-inheritance diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("only abstract records may be extended")),
            "Expected concrete-base diagnostic, got {:?}",
            messages
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("redeclares inherited field 'name'")),
            "Expected duplicate inherited field diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_action_inheritance_validation_diagnostics() {
        let source = r#"
            abstract action InputAction = {
              source: string
            }

            action DuplicateAction extends InputAction = {
              source: string
            }

            action ConcreteBase = {
              query: string
            }

            action DerivedAction extends ConcreteBase = {
              submittedAt: string
            }
        "#;
        let parse_result = parse_str(source, "action-inheritance-errors.nx");
        let tree = parse_result
            .tree
            .expect("Should parse action inheritance error source");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer action-inheritance diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("only abstract actions may be extended")),
            "Expected concrete-base diagnostic, got {:?}",
            messages
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("redeclares inherited field 'source'")),
            "Expected duplicate inherited field diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_action_inheritance_rejects_mixed_kinds() {
        let source = r#"
            abstract type EventBase = {
              source: string
            }

            abstract action ActionBase = {
              query: string
            }

            action ActionDerived extends EventBase = {
              value: string
            }

            type RecordDerived extends ActionBase = {
              extra: string
            }
        "#;
        let parse_result = parse_str(source, "action-kind-mismatch.nx");
        let tree = parse_result
            .tree
            .expect("Should parse action inheritance kind mismatch");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer action kind mismatch diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("actions cannot be used as base records")),
            "Expected record/action mismatch diagnostic, got {:?}",
            messages
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("records cannot be used as base actions")),
            "Expected action/record mismatch diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_record_inheritance_duplicate_content_property_diagnostic() {
        let source = r#"
            abstract type Base = {
              content body:string
            }

            type Card extends Base = {
              content footer:string
            }
        "#;
        let parse_result = parse_str(source, "record-content-conflict.nx");
        let tree = parse_result
            .tree
            .expect("Should parse content inheritance conflict");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer record-inheritance diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("already inherits content property")),
            "Expected duplicate content property diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_record_inheritance_cycle_diagnostic() {
        let source = r#"
            abstract type Entity extends UserBase = {
              id: int
            }

            abstract type UserBase extends Entity = {
              name: string
            }
        "#;
        let parse_result = parse_str(source, "record-cycle.nx");
        let tree = parse_result
            .tree
            .expect("Should parse record inheritance cycle");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer record-inheritance diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("Record inheritance cycle detected")),
            "Expected inheritance cycle diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_action_inheritance_cycle_diagnostic() {
        let source = r#"
            abstract action InputAction extends SearchAction = {
              source: string
            }

            abstract action SearchAction extends InputAction = {
              query: string
            }
        "#;
        let parse_result = parse_str(source, "action-cycle.nx");
        let tree = parse_result
            .tree
            .expect("Should parse action inheritance cycle");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer action cycle diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);

        assert!(
            messages
                .iter()
                .any(|message| message.contains("inheritance cycle detected")),
            "Expected inheritance cycle diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_record_inheritance_reports_invalid_mid_chain_base_once() {
        let source = r#"
            abstract type BrokenBase extends MissingBase = {
              id: int
            }

            type User extends BrokenBase = {
              name: string
            }

            type Admin extends BrokenBase = {
              level: int
            }
        "#;
        let parse_result = parse_str(source, "record-mid-chain-invalid-base.nx");
        let tree = parse_result
            .tree
            .expect("Should parse record inheritance error source");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Raw lowering should defer record-inheritance diagnostics to prepared validation"
        );
        let messages = prepared_record_validation_messages(&module);
        let missing_base_messages = messages
            .iter()
            .filter(|message| message.contains("BrokenBase") && message.contains("MissingBase"))
            .count();

        assert_eq!(
            missing_base_messages, 1,
            "Expected invalid mid-chain base to be reported once, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_action_definition_as_action_record() {
        let source = r#"
            action SaveRequested = {
              value: string
            }
        "#;
        let parse_result = parse_str(source, "action.nx");
        let tree = parse_result.tree.expect("Should parse action");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) => Some(def),
                _ => None,
            })
            .expect("Should lower action definition");

        assert_eq!(action.name.as_str(), "SaveRequested");
        assert_eq!(action.kind, RecordKind::Action);
        assert!(action.is_action());
        assert_eq!(action.properties.len(), 1);
    }

    #[test]
    fn test_lower_record_field_nullable_and_array_types() {
        let source = r#"
            type User = {
              tags: string[]
              age: int?
              aliases: string?[]
              grouped: string[][]
              backupTags: string[]?
              maybeAliases: string?[]?
            }
        "#;
        let parse_result = parse_str(source, "record-types.nx");
        let tree = parse_result.tree.expect("Should parse record");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let record = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) => Some(def),
                _ => None,
            })
            .expect("Should lower record definition");

        let tags = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "tags")
            .expect("Expected tags field");
        match &tags.ty {
            TypeRef::Array(inner) => match inner.as_ref() {
                TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                other => panic!("Expected string element type, got {:?}", other),
            },
            other => panic!("Expected array type, got {:?}", other),
        }

        let age = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "age")
            .expect("Expected age field");
        match &age.ty {
            TypeRef::Nullable(inner) => match inner.as_ref() {
                TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
                other => panic!("Expected int inner type, got {:?}", other),
            },
            other => panic!("Expected nullable type, got {:?}", other),
        }

        let aliases = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "aliases")
            .expect("Expected aliases field");
        match &aliases.ty {
            TypeRef::Array(inner) => match inner.as_ref() {
                TypeRef::Nullable(nullable_inner) => match nullable_inner.as_ref() {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                    other => panic!("Expected string alias element type, got {:?}", other),
                },
                other => panic!("Expected nullable alias element type, got {:?}", other),
            },
            other => panic!("Expected array alias type, got {:?}", other),
        }

        let grouped = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "grouped")
            .expect("Expected grouped field");
        match &grouped.ty {
            TypeRef::Array(outer) => match outer.as_ref() {
                TypeRef::Array(inner) => match inner.as_ref() {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                    other => panic!("Expected grouped inner string type, got {:?}", other),
                },
                other => panic!("Expected grouped inner array type, got {:?}", other),
            },
            other => panic!("Expected grouped outer array type, got {:?}", other),
        }

        let backup_tags = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "backupTags")
            .expect("Expected backupTags field");
        match &backup_tags.ty {
            TypeRef::Nullable(inner) => match inner.as_ref() {
                TypeRef::Array(array_inner) => match array_inner.as_ref() {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                    other => panic!("Expected backupTags array string type, got {:?}", other),
                },
                other => panic!("Expected backupTags inner array type, got {:?}", other),
            },
            other => panic!("Expected nullable backupTags type, got {:?}", other),
        }

        let maybe_aliases = record
            .properties
            .iter()
            .find(|field| field.name.as_str() == "maybeAliases")
            .expect("Expected maybeAliases field");
        match &maybe_aliases.ty {
            TypeRef::Nullable(inner) => match inner.as_ref() {
                TypeRef::Array(array_inner) => match array_inner.as_ref() {
                    TypeRef::Nullable(nullable_inner) => match nullable_inner.as_ref() {
                        TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                        other => panic!("Expected maybeAliases inner string type, got {:?}", other),
                    },
                    other => panic!(
                        "Expected maybeAliases nullable array element type, got {:?}",
                        other
                    ),
                },
                other => panic!("Expected maybeAliases inner array type, got {:?}", other),
            },
            other => panic!("Expected nullable maybeAliases type, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_action_literal_expression() {
        let source = r#"
            action SaveRequested = { value: string = "ready" }
            let build(): SaveRequested = { <SaveRequested /> }
        "#;
        let parse_result = parse_str(source, "action-literal.nx");
        let tree = parse_result.tree.expect("Should parse action literal");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) if def.name.as_str() == "SaveRequested" => Some(def),
                _ => None,
            })
            .expect("Should lower action definition");
        assert_eq!(action.kind, RecordKind::Action);

        let build = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(func) if func.name.as_str() == "build" => Some(func),
                _ => None,
            })
            .expect("Should lower build function");

        let Expr::RecordLiteral { record, .. } = module.expr(build.body) else {
            panic!("Expected action constructor to lower as record literal");
        };
        assert_eq!(record.as_str(), "SaveRequested");
    }

    #[test]
    fn test_lower_inline_emit_action_inheritance_metadata() {
        let source = r#"
            abstract action InputAction = {
              source: string
            }

            type InputActionBase = InputAction

            component <SearchBox emits { ValueChanged extends InputActionBase { value: string } } /> = {
              <TextInput />
            }
        "#;
        let parse_result = parse_str(source, "inline-action-inheritance.nx");
        let tree = parse_result
            .tree
            .expect("Inline emitted action inheritance should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let inline_action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(record) if record.name.as_str() == "SearchBox.ValueChanged" => {
                    Some(record)
                }
                _ => None,
            })
            .expect("Expected synthesized inline emitted action record");

        assert_eq!(inline_action.kind, RecordKind::Action);
        assert!(!inline_action.is_abstract);
        assert_eq!(
            inline_action.base.as_ref().map(|name| name.as_str()),
            Some("InputActionBase")
        );

        let messages = prepared_record_validation_messages(&module);
        assert!(
            messages.is_empty(),
            "Inline emitted action inheritance should validate, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_record_literal_expression() {
        let source = r#"
            type User = { name: string age: int = 30 }
            let make(): User = { <User name="Bob" /> }
        "#;

        let parse_result = parse_str(source, "record-literal.nx");
        let tree = parse_result.tree.expect("Should parse record literal");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let func = match module.find_item("make") {
            Some(Item::Function(f)) => f,
            _ => panic!("expected function item"),
        };

        match module.expr(func.body) {
            Expr::RecordLiteral {
                record, properties, ..
            } => {
                assert_eq!(record.as_str(), "User");
                assert_eq!(properties.len(), 1);
                assert_eq!(properties[0].name.as_str(), "name");
            }
            other => panic!("expected record literal expr, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_union_with_leading_pipe() {
        let source = r#"
            type Orientation = horizontal | vertical
        "#;
        let parse_result = parse_str(source, "enum-lead.nx");
        let tree = parse_result.tree.expect("Should parse enum");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let unions: Vec<_> = module
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Union(def) => Some(def),
                _ => None,
            })
            .collect();
        assert_eq!(unions.len(), 1);
        let union_def = unions[0];
        assert_eq!(union_def.name.as_str(), "Orientation");
        assert!(union_def.is_constant_union());
        assert_eq!(union_def.cases.len(), 2);
        assert_eq!(union_def.cases[0].name.as_str(), "horizontal");
        assert_eq!(union_def.cases[1].name.as_str(), "vertical");
    }

    #[test]
    fn test_lower_simple_element() {
        let source = "<button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        // Should have one function item named 'root' (implicit from top-level element)
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");
                assert!(func.params.is_empty());

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "button");
                        assert_eq!(elem.properties.len(), 0);
                        assert_eq!(elem.content.len(), 0);
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_embed_interpolation_inside_text_element() {
        let source = r#"
            <Root>
              <markdown:text>@{user}</markdown:text>
            </Root>
        "#;
        let parse_result = parse_str(source, "text.nx");
        let tree = parse_result.tree.expect("Should parse typed text element");
        let root = tree.root();

        let embed_interp = find_first_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION)
            .expect("expected embed brace node");

        let mut ctx = LoweringContext::new(SourceId::new(0));
        let expr_id = ctx.lower_expr(embed_interp);
        let expr = ctx.module.expr(expr_id);

        match expr {
            Expr::Ident(name) => assert_eq!(name.as_str(), "user"),
            other => panic!("Expected identifier from embed brace, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_multi_item_embed_braced_expression_to_array() {
        let source = r#"
            <Root>
              <markdown:text>@{user title}</markdown:text>
            </Root>
        "#;
        let parse_result = parse_str(source, "text-list.nx");
        let tree = parse_result.tree.expect("Should parse typed text element");
        let root = tree.root();

        let embed_braced = find_first_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION)
            .expect("expected embed brace node");

        let mut ctx = LoweringContext::new(SourceId::new(0));
        let expr_id = ctx.lower_expr(embed_braced);
        let expr = ctx.module.expr(expr_id);

        match expr {
            Expr::Array { elements, .. } => assert_eq!(elements.len(), 2),
            other => panic!(
                "Expected array from multi-item embed brace, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_multi_item_embed_braced_expression_with_elements_to_array() {
        let source = r#"
            <Root>
              <markdown:text>@{<A/> <B/>}</markdown>
            </Root>
        "#;
        let parse_result = parse_str(source, "text-elements.nx");
        assert!(
            parse_result.errors.is_empty(),
            "Expected element-valued embed brace to parse, got {:?}",
            parse_result.errors
        );
        let tree = parse_result.tree.expect("Should parse typed text element");
        let root = tree.root();

        let embed_braced = find_first_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION)
            .expect("expected embed brace node");

        let mut ctx = LoweringContext::new(SourceId::new(0));
        let expr_id = ctx.lower_expr(embed_braced);
        let expr = ctx.module.expr(expr_id);

        match expr {
            Expr::Array { elements, .. } => {
                assert_eq!(elements.len(), 2);
                for element in elements {
                    assert!(
                        matches!(ctx.module.expr(*element), Expr::Element { .. }),
                        "Expected embed brace element item, got {:?}",
                        ctx.module.expr(*element)
                    );
                }
            }
            other => panic!(
                "Expected array from element-valued embed brace, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_values_braced_expression_preserves_source_arity() {
        let source = r#"
            let single = {item}
            let many = {first second}
        "#;
        let parse_result = parse_str(source, "values-braces.nx");
        let tree = parse_result.tree.expect("Should parse braced values");
        let root = tree.root();

        let mut braced_nodes = Vec::new();
        collect_kinds(
            &root,
            SyntaxKind::VALUES_BRACED_EXPRESSION,
            &mut braced_nodes,
        );
        assert_eq!(
            braced_nodes.len(),
            2,
            "Expected singleton and multi-item brace nodes"
        );

        let mut ctx = LoweringContext::new(SourceId::new(0));
        let single_expr = ctx.lower_expr(braced_nodes[0]);
        let many_expr = ctx.lower_expr(braced_nodes[1]);

        match ctx.module.expr(single_expr) {
            Expr::Ident(name) => assert_eq!(name.as_str(), "item"),
            other => panic!(
                "Expected singleton brace to lower to inner expression, got {:?}",
                other
            ),
        }

        match ctx.module.expr(many_expr) {
            Expr::Array { elements, .. } => assert_eq!(elements.len(), 2),
            other => panic!(
                "Expected multi-item brace to lower to array, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_element_with_properties() {
        let source = r#"<button class="btn" disabled="true" />"#;
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "button");
                        assert_eq!(elem.properties.len(), 2);
                        assert_eq!(elem.properties[0].key.as_str(), "class");
                        assert_eq!(elem.properties[1].key.as_str(), "disabled");
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_element_with_simple_property_fragment() {
        let source = r#"<Button if showLabel { label="Save" } />"#;
        let parse_result = parse_str(source, "property-fragment-simple.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));
        let Item::Function(func) = &module.items()[0] else {
            panic!("Expected Function item for implicit root");
        };
        let Expr::Element { element, .. } = module.expr(func.body) else {
            panic!("Expected Element expression as body");
        };
        let elem = module.element(*element);

        assert_eq!(elem.properties.len(), 0);
        assert_eq!(elem.property_entries.len(), 1);
        let PropertyEntry::If {
            then_entries,
            else_entries,
            ..
        } = &elem.property_entries[0]
        else {
            panic!("Expected simple property-list if fragment");
        };

        assert_eq!(then_entries.len(), 1);
        assert_eq!(else_entries.len(), 0);
        let PropertyEntry::Value(property) = &then_entries[0] else {
            panic!("Expected direct property in then branch");
        };
        assert_eq!(property.key.as_str(), "label");
    }

    #[test]
    fn test_lower_element_with_condition_list_property_fragment() {
        let source = r#"<Badge if { isError => tone="danger" isWarning => tone="warning" else => tone="neutral" } />"#;
        let parse_result = parse_str(source, "property-fragment-condition-list.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));
        let Item::Function(func) = &module.items()[0] else {
            panic!("Expected Function item for implicit root");
        };
        let Expr::Element { element, .. } = module.expr(func.body) else {
            panic!("Expected Element expression as body");
        };
        let elem = module.element(*element);

        assert_eq!(elem.property_entries.len(), 1);
        let PropertyEntry::ConditionList {
            arms, else_entries, ..
        } = &elem.property_entries[0]
        else {
            panic!("Expected condition-list property fragment");
        };

        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].entries.len(), 1);
        assert_eq!(arms[1].entries.len(), 1);
        assert_eq!(else_entries.len(), 1);
    }

    #[test]
    fn test_lower_element_with_match_property_fragment() {
        let source = r#"<View if state is { LoadState.failed => message={state.message} else => message="" } />"#;
        let parse_result = parse_str(source, "property-fragment-match.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));
        let Item::Function(func) = &module.items()[0] else {
            panic!("Expected Function item for implicit root");
        };
        let Expr::Element { element, .. } = module.expr(func.body) else {
            panic!("Expected Element expression as body");
        };
        let elem = module.element(*element);

        assert_eq!(elem.property_entries.len(), 1);
        let PropertyEntry::Match {
            arms, else_entries, ..
        } = &elem.property_entries[0]
        else {
            panic!("Expected match property fragment");
        };

        assert_eq!(arms.len(), 1);
        assert_eq!(arms[0].patterns.len(), 1);
        assert_eq!(arms[0].entries.len(), 1);
        assert_eq!(else_entries.len(), 1);
    }

    #[test]
    fn test_lower_nested_elements() {
        let source = "<div><button /></div>";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "div");
                        assert_eq!(elem.content.len(), 1);

                        // Check nested button element
                        match module.expr(elem.content[0]) {
                            Expr::Element { element, .. } => {
                                let child = module.element(*element);
                                assert_eq!(child.tag.as_str(), "button");
                            }
                            other => {
                                panic!("Expected nested element child expression, got {:?}", other)
                            }
                        }
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_dynamic_element_content_is_preserved() {
        let source = r#"
            let <Panel flag:boolean items:object /> = <div>
              {<A /> <B />}
              if flag { <Shown /> } else { <Hidden /> }
              for item in items { <Row /> }
            </div>
        "#;
        let parse_result = parse_str(source, "dynamic-content.nx");
        let tree = parse_result.tree.expect("Should parse dynamic content");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let func = match &module.items()[0] {
            Item::Function(func) => func,
            other => panic!("Expected function item, got {:?}", other),
        };

        let body_expr = module.expr(func.body);
        let Expr::Element { element, .. } = body_expr else {
            panic!("Expected element body, got {:?}", body_expr);
        };

        let element = module.element(*element);
        assert_eq!(element.content.len(), 3);

        match module.expr(element.content[0]) {
            Expr::Array { elements, .. } => {
                assert_eq!(elements.len(), 2);
                for child in elements {
                    assert!(
                        matches!(module.expr(*child), Expr::Element { .. }),
                        "Expected element in braced content list, got {:?}",
                        module.expr(*child)
                    );
                }
            }
            other => panic!(
                "Expected braced content list to lower to array, got {:?}",
                other
            ),
        }

        match module.expr(element.content[1]) {
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                assert!(matches!(module.expr(*then_branch), Expr::Element { .. }));
                let else_branch = else_branch.as_ref().expect("Expected else branch");
                assert!(matches!(module.expr(*else_branch), Expr::Element { .. }));
            }
            other => panic!("Expected conditional content expression, got {:?}", other),
        }

        match module.expr(element.content[2]) {
            Expr::For { body, .. } => {
                assert!(matches!(module.expr(*body), Expr::Element { .. }));
            }
            other => panic!("Expected for content expression, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_scalar_braced_content_expression_is_preserved() {
        let source = r#"
            let <Panel count:int /> = <div>{count}</div>
        "#;
        let parse_result = parse_str(source, "scalar-content.nx");
        let tree = parse_result.tree.expect("Should parse scalar content");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let func = match &module.items()[0] {
            Item::Function(func) => func,
            other => panic!("Expected function item, got {:?}", other),
        };

        let body_expr = module.expr(func.body);
        let Expr::Element { element, .. } = body_expr else {
            panic!("Expected element body, got {:?}", body_expr);
        };

        let element = module.element(*element);
        assert_eq!(element.content.len(), 1);

        match module.expr(element.content[0]) {
            Expr::Ident(name) => assert_eq!(name.as_str(), "count"),
            other => panic!(
                "Expected scalar content expression to be preserved, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_plain_text_element_content_is_preserved() {
        let source = r#"
            let root() = <Panel>label text<Badge /></Panel>
        "#;
        let parse_result = parse_str(source, "text-content.nx");
        let tree = parse_result
            .tree
            .expect("Should parse regular element text content");
        let module = lower(tree.root(), SourceId::new(0));

        let func = match &module.items()[0] {
            Item::Function(func) => func,
            other => panic!("Expected function item, got {:?}", other),
        };

        let body_expr = module.expr(func.body);
        let Expr::Element { element, .. } = body_expr else {
            panic!("Expected element body, got {:?}", body_expr);
        };

        let element = module.element(*element);
        assert_eq!(element.content.len(), 2);

        match module.expr(element.content[0]) {
            Expr::Literal(Literal::String(value)) => {
                assert_eq!(value.as_str(), "label text");
            }
            other => panic!(
                "Expected text content to lower to string literal, got {:?}",
                other
            ),
        }

        assert!(
            matches!(module.expr(element.content[1]), Expr::Element { .. }),
            "Expected nested element content item, got {:?}",
            module.expr(element.content[1])
        );
    }

    #[test]
    fn test_lower_function_body_element_expression() {
        use crate::ast::Expr;

        let source = "let <Button text:string /> = <button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        match &module.items()[0] {
            Item::Function(func) => {
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let element_ref = module.element(*element);
                        assert_eq!(element_ref.tag.as_str(), "button");
                    }
                    _ => panic!("Expected element expression in function body"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_string_addition_lowers_to_concat() {
        let source = r#"let <concat a:string b:string /> = { a + b }"#;
        let parse_result = parse_str(source, "test.nx");
        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );
        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);
        let func = match &module.items()[0] {
            Item::Function(f) => f,
            _ => panic!("expected function item"),
        };

        let assert_concat = |expr_id: ExprId| match module.expr(expr_id) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Concat),
            Expr::Block {
                expr: Some(final_expr),
                ..
            } => match module.expr(*final_expr) {
                Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Concat),
                other => panic!("expected binary op in block, found {:?}", other),
            },
            other => panic!("expected binary op, found {:?}", other),
        };

        assert_concat(func.body);
    }

    #[test]
    fn test_lower_for_loop_simple() {
        let source = "let <ForSimple items:object /> = {for item in items { item * 2 }}";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        // Should have one function item
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "ForSimple");

                // Function body should be a block containing a for loop
                let body_expr = module.expr(func.body);

                // Navigate through potential block wrapper
                let for_expr = match body_expr {
                    Expr::For { .. } => body_expr,
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected For or Block expression, got {:?}", other),
                };

                // Verify it's a for loop
                match for_expr {
                    Expr::For {
                        item,
                        index,
                        iterable,
                        body,
                        ..
                    } => {
                        assert_eq!(item.as_str(), "item");
                        assert!(index.is_none());

                        // Verify iterable is an identifier
                        match module.expr(*iterable) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "items"),
                            other => panic!("Expected Ident for iterable, got {:?}", other),
                        }

                        // Verify body is a binary operation
                        match module.expr(*body) {
                            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Mul),
                            other => panic!("Expected BinaryOp for body, got {:?}", other),
                        }
                    }
                    other => panic!("Expected For expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_for_loop_with_index() {
        let source =
            "let <ForWithIndex items:object /> = {for item, index in items { item + index }}";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                let body_expr = module.expr(func.body);

                let for_expr = match body_expr {
                    Expr::For { .. } => body_expr,
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected For or Block expression, got {:?}", other),
                };

                match for_expr {
                    Expr::For { item, index, .. } => {
                        assert_eq!(item.as_str(), "item");
                        assert!(index.is_some());
                        assert_eq!(index.as_ref().unwrap().as_str(), "index");
                    }
                    other => panic!("Expected For expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_ternary_expression() {
        // Ternary: condition ? consequent : alternative
        let source = "let choose(cond:boolean): int = { cond ? 1 : 0 }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "choose");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If {
                        condition,
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        // Verify condition is an identifier
                        match module.expr(*condition) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "cond"),
                            other => panic!("Expected Ident for condition, got {:?}", other),
                        }

                        // Verify then branch is literal 1
                        match module.expr(*then_branch) {
                            Expr::Literal(Literal::Int(1)) => (),
                            other => {
                                panic!("Expected Literal(Int(1)) for then_branch, got {:?}", other)
                            }
                        }

                        // Verify else branch is literal 0
                        assert!(else_branch.is_some());
                        match module.expr(else_branch.unwrap()) {
                            Expr::Literal(Literal::Int(0)) => (),
                            other => {
                                panic!("Expected Literal(Int(0)) for else_branch, got {:?}", other)
                            }
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_if_else_expression() {
        // If-else: if condition { then } else { else }
        let source = "let max(a:int, b:int): int = { if a > b { a } else { b } }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "max");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If {
                        condition,
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        // Verify condition is a > b
                        match module.expr(*condition) {
                            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Gt),
                            other => panic!("Expected BinaryOp for condition, got {:?}", other),
                        }

                        // Verify then branch is identifier 'a'
                        match module.expr(*then_branch) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "a"),
                            other => panic!("Expected Ident for then_branch, got {:?}", other),
                        }

                        // Verify else branch is identifier 'b'
                        assert!(else_branch.is_some());
                        match module.expr(else_branch.unwrap()) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "b"),
                            other => panic!("Expected Ident for else_branch, got {:?}", other),
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_if_without_else() {
        // If without else: if condition { then }
        let source = "let maybe(x:int): int = { if x > 0 { x } }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "maybe");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If { else_branch, .. } => {
                        // Verify no else branch
                        assert!(else_branch.is_none(), "Expected no else branch");
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_match_expression_preserves_arms() {
        let source = r#"
            let describe(x:int): string = {
                if x is {
                    0 => "zero"
                    1, 2 => "small"
                    else => "many"
                }
            }
        "#;
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                let match_expr = match module.expr(func.body) {
                    Expr::Match { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected Match or Block expression, got {:?}", other),
                };

                match match_expr {
                    Expr::Match {
                        scrutinee,
                        arms,
                        else_branch,
                        ..
                    } => {
                        assert!(
                            matches!(module.expr(*scrutinee), Expr::Ident(name) if name.as_str() == "x")
                        );
                        assert_eq!(arms.len(), 2);
                        assert_eq!(arms[0].patterns.len(), 1);
                        assert_eq!(arms[1].patterns.len(), 2);
                        assert!(else_branch.is_some());
                    }
                    other => panic!("Expected Match expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_nested_ternary() {
        // Nested ternary: x > 0 ? "positive" : x < 0 ? "negative" : "zero"
        let source =
            r#"let classify(x:int): string = { x > 0 ? "positive" : x < 0 ? "negative" : "zero" }"#;
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "classify");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                // Verify it's a nested if expression
                match if_expr {
                    Expr::If { else_branch, .. } => {
                        assert!(else_branch.is_some());
                        // The else branch should be another if expression
                        match module.expr(else_branch.unwrap()) {
                            Expr::If {
                                else_branch: inner_else,
                                ..
                            } => {
                                assert!(inner_else.is_some());
                            }
                            other => panic!("Expected nested If expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_component_inline_emit_public_name_and_handler_forward_references() {
        let source = r#"
            action TrackSearch = { value: string }

            let makeChange(value:string): SearchBox.ValueChanged = { <SearchBox.ValueChanged value={value} /> }
            let render() = <SearchBox onValueChanged=<TrackSearch value={action.value} /> />

            component <SearchBox emits { ValueChanged { value:string } } /> = {
              <TextInput />
            }
        "#;

        let parse_result = parse_str(source, "component-actions.nx");
        let tree = parse_result
            .tree
            .expect("Component action handler source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let component = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) => Some(component),
                _ => None,
            })
            .expect("Expected lowered component item");
        assert_eq!(component.name.as_str(), "SearchBox");
        assert_eq!(component.emits.len(), 1);
        assert_eq!(component.emits[0].name.as_str(), "ValueChanged");
        assert_eq!(
            component.emits[0].action_name.as_str(),
            "SearchBox.ValueChanged"
        );
        assert_eq!(component.emits[0].kind, ComponentEmitKind::Inline);

        let inline_action = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(record) if record.name.as_str() == "SearchBox.ValueChanged" => {
                    Some(record)
                }
                _ => None,
            })
            .expect("Expected synthesized inline emitted action record");
        assert_eq!(inline_action.kind, RecordKind::Action);

        let make_change = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "makeChange" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("Expected makeChange function");

        match make_change
            .return_type
            .as_ref()
            .expect("Expected explicit return type")
        {
            TypeRef::Name(name) => assert_eq!(name.as_str(), "SearchBox.ValueChanged"),
            other => panic!("Expected named return type, got {:?}", other),
        }

        match module.expr(make_change.body) {
            Expr::RecordLiteral { record, .. } => {
                assert_eq!(record.as_str(), "SearchBox.ValueChanged");
            }
            other => panic!("Expected record literal in makeChange, got {:?}", other),
        }

        let render = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "render" => Some(function),
                _ => None,
            })
            .expect("Expected render function");

        let render_element = match module.expr(render.body) {
            Expr::Element { element, .. } => *element,
            other => panic!("Expected component invocation element, got {:?}", other),
        };
        let property = &module.element(render_element).properties[0];
        match module.expr(property.value) {
            Expr::ActionHandler {
                component,
                emit,
                action_name,
                ..
            } => {
                assert_eq!(component.as_str(), "SearchBox");
                assert_eq!(emit.as_str(), "ValueChanged");
                assert_eq!(action_name.as_str(), "SearchBox.ValueChanged");
            }
            other => panic!(
                "Expected lowered action handler expression, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_inherited_component_emit_handler_uses_ancestor_action_name() {
        let source = r#"
            action TrackSearch = { value: string }

            abstract component <SearchBase emits { ValueChanged { value:string } } />
            component <SearchBox extends SearchBase /> = {
              <TextInput />
            }

            let render() = <SearchBox onValueChanged=<TrackSearch value={action.value} /> />
        "#;

        let parse_result = parse_str(source, "component-inherited-actions.nx");
        let tree = parse_result
            .tree
            .expect("Inherited component action handler source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let render = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "render" => Some(function),
                _ => None,
            })
            .expect("Expected render function");

        let render_element = match module.expr(render.body) {
            Expr::Element { element, .. } => *element,
            other => panic!("Expected component invocation element, got {:?}", other),
        };
        let property = &module.element(render_element).properties[0];
        match module.expr(property.value) {
            Expr::ActionHandler {
                component,
                emit,
                action_name,
                ..
            } => {
                assert_eq!(component.as_str(), "SearchBox");
                assert_eq!(emit.as_str(), "ValueChanged");
                assert_eq!(action_name.as_str(), "SearchBase.ValueChanged");
            }
            other => panic!(
                "Expected lowered inherited action handler expression, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_component_preserves_prop_defaults_state_and_body() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" /> = {
              state {
                query:string = {placeholder}
              }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#;

        let parse_result = parse_str(source, "component-runtime.nx");
        let tree = parse_result
            .tree
            .expect("Component runtime source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let component = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "SearchBox" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected lowered component item");

        assert_eq!(component.props.len(), 1);
        assert_eq!(component.props[0].name.as_str(), "placeholder");
        assert!(
            component.props[0].default.is_some(),
            "Expected component prop default expression"
        );

        assert_eq!(component.state.len(), 1);
        assert_eq!(component.state[0].name.as_str(), "query");
        assert!(
            component.state.iter().all(|field| field.default.is_some()),
            "Expected state defaults to be preserved"
        );
        assert!(!component.is_abstract);
        assert!(!component.is_external);
        assert!(component.base.is_none());

        let Expr::Element { element, .. } = module.expr(
            component
                .body
                .expect("Expected lowered component body expression"),
        ) else {
            panic!("Expected lowered component body expression");
        };
        let element = module.element(*element);
        assert_eq!(element.tag.as_str(), "TextInput");
        assert_eq!(element.properties.len(), 2);
    }

    #[test]
    fn test_lower_component_preserves_modifier_base_and_optional_body_metadata() {
        let source = r#"
            abstract component <SearchBase placeholder:string emits { ValueChanged { value:string } } />
            external component <SearchBox extends SearchBase showSearchIcon:boolean = true />
            abstract external component <RemoteSearchBase />
        "#;

        let parse_result = parse_str(source, "component-metadata.nx");
        let tree = parse_result
            .tree
            .expect("Component metadata source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let search_base = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "SearchBase" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected abstract base component");
        assert!(search_base.is_abstract);
        assert!(!search_base.is_external);
        assert!(search_base.base.is_none());
        assert!(search_base.body.is_none());
        assert_eq!(
            search_base.emits[0].action_name.as_str(),
            "SearchBase.ValueChanged"
        );

        let search_box = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "SearchBox" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected external derived component");
        assert!(!search_box.is_abstract);
        assert!(search_box.is_external);
        assert_eq!(
            search_box.base.as_ref().map(|name| name.as_str()),
            Some("SearchBase")
        );
        assert!(search_box.body.is_none());
        assert_eq!(search_box.props.len(), 1);
        assert_eq!(search_box.props[0].name.as_str(), "showSearchIcon");

        let remote_base = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "RemoteSearchBase" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected abstract external component");
        assert!(remote_base.is_abstract);
        assert!(remote_base.is_external);
        assert!(remote_base.base.is_none());
        assert!(remote_base.body.is_none());
    }

    #[test]
    fn test_lower_external_component_state_only_body_preserves_state_without_body() {
        let source = r#"
            external component <SearchBox placeholder:string /> = {
              state {
                query:string
              }
            }
        "#;

        let parse_result = parse_str(source, "component-external-state.nx");
        let tree = parse_result
            .tree
            .expect("External component state source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let component = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "SearchBox" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected lowered external component item");

        assert!(component.is_external);
        assert!(!component.is_abstract);
        assert_eq!(component.props.len(), 1);
        assert_eq!(component.props[0].name.as_str(), "placeholder");
        assert_eq!(component.state.len(), 1);
        assert_eq!(component.state[0].name.as_str(), "query");
        assert!(component.body.is_none());
    }

    #[test]
    fn test_lower_component_handler_diagnostics_for_collision_and_unknown_emit() {
        let source = r#"
            component <SearchBox onSearchSubmitted:string emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render() = <SearchBox onSearchRequested=<DoSearch search={action.searchString} /> />
        "#;

        let parse_result = parse_str(source, "component-handler-errors.nx");
        let tree = parse_result
            .tree
            .expect("Component handler error source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let messages: Vec<_> = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("collides with emitted action handler")),
            "Expected prop collision diagnostic, got {:?}",
            messages
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("does not emit 'SearchRequested'")),
            "Expected unknown emitted action diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_component_shared_emit_preserves_shared_action_name() {
        let source = r#"
            action SearchSubmitted = { searchString:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let parse_result = parse_str(source, "component-shared-emit.nx");
        let tree = parse_result.tree.expect("Shared emit source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let component = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Component(component) if component.name.as_str() == "SearchBox" => {
                    Some(component)
                }
                _ => None,
            })
            .expect("Expected component item");

        assert_eq!(component.emits.len(), 1);
        assert_eq!(component.emits[0].kind, ComponentEmitKind::Shared);
        assert_eq!(component.emits[0].action_name.as_str(), "SearchSubmitted");
    }

    #[test]
    fn test_lower_component_duplicate_emit_handler_name_diagnostic() {
        let source = r#"
            action SearchSubmitted = { searchString:string }

            component <SearchBox emits { SearchSubmitted SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let parse_result = parse_str(source, "component-duplicate-handler-prop.nx");
        let tree = parse_result
            .tree
            .expect("Duplicate emit source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        let messages: Vec<_> = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert!(
            messages.iter().any(|message| {
                message.contains("emits multiple actions that map to handler 'onSearchSubmitted'")
            }),
            "Expected duplicate handler mapping diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn test_lower_non_matching_on_property_as_regular_component_prop() {
        let source = r#"
            component <SearchBox onClick:string /> = {
              <button />
            }

            let render() = <SearchBox onClick="primary" />
        "#;

        let parse_result = parse_str(source, "component-on-prop.nx");
        let tree = parse_result
            .tree
            .expect("Regular on-prop source should parse");
        let module = lower(tree.root(), SourceId::new(0));

        assert!(
            module.diagnostics().is_empty(),
            "Expected no lowering diagnostics, got {:?}",
            module.diagnostics()
        );

        let render = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "render" => Some(function),
                _ => None,
            })
            .expect("Expected render function");

        let render_element = match module.expr(render.body) {
            Expr::Element { element, .. } => *element,
            other => panic!("Expected component invocation element, got {:?}", other),
        };
        let property = &module.element(render_element).properties[0];
        assert_eq!(property.key.as_str(), "onClick");
        assert!(
            !matches!(module.expr(property.value), Expr::ActionHandler { .. }),
            "Expected onClick to remain a normal prop"
        );
    }

    fn lower_source(source: &str, file_name: &str) -> LoweredModule {
        let parse_result = parse_str(source, file_name);
        let tree = parse_result.tree.expect("Source should parse");
        lower(tree.root(), SourceId::new(0))
    }

    fn find_record<'a>(module: &'a LoweredModule, name: &str) -> &'a RecordDef {
        match module.find_item(name) {
            Some(Item::Record(record)) => record,
            other => panic!("Expected record '{}', got {:?}", name, other),
        }
    }

    fn field_names(fields: &[RecordField]) -> Vec<&str> {
        fields.iter().map(|field| field.name.as_str()).collect()
    }

    #[test]
    fn update_records_are_synthesized_for_records_actions_emits_and_stateful_components() {
        let module = lower_source(
            r#"
            type User = { name:string = "anon" email:string? }
            action Saved = { id:int }
            component <Counter step:int = 1 emits { Reset { to:int } } /> = {
              state { count:int = 0 label:string = "x" }
              <Label />
            }
            component <Plain /> = { <Label /> }
            "#,
            "update-records.nx",
        );
        assert!(
            module.diagnostics().is_empty(),
            "Expected no lowering diagnostics, got {:?}",
            module.diagnostics()
        );

        let user_update = find_record(&module, "User.Update");
        assert_eq!(
            user_update.kind,
            RecordKind::Update {
                target: Name::new("User")
            }
        );
        assert!(!user_update.is_action());
        assert!(!user_update.is_abstract);
        assert_eq!(field_names(&user_update.properties), vec!["name", "email"]);
        assert!(
            user_update
                .properties
                .iter()
                .all(|field| field.default.is_none()),
            "Update record fields must not carry defaults"
        );

        let saved_update = find_record(&module, "Saved.Update");
        assert_eq!(
            saved_update.update_target().map(Name::as_str),
            Some("Saved")
        );
        assert!(!saved_update.is_action());

        let counter_update = find_record(&module, "Counter.Update");
        assert_eq!(
            counter_update.update_target().map(Name::as_str),
            Some("Counter")
        );
        assert_eq!(
            field_names(&counter_update.properties),
            vec!["count", "label"],
            "A component's update record is derived from state and excludes props"
        );

        let reset_update = find_record(&module, "Counter.Reset.Update");
        assert_eq!(field_names(&reset_update.properties), vec!["to"]);

        assert!(
            module.find_item("Plain.Update").is_none(),
            "A component without state has no update record"
        );
        assert!(
            module.find_item("User.Update.Update").is_none(),
            "Update records have no update record of their own"
        );
    }

    #[test]
    fn update_record_fields_are_optional_and_include_inherited_fields() {
        let module = lower_source(
            r#"
            abstract type Named = { name:string }
            type User extends Named = { email:string = "none" }
            "#,
            "update-inherited.nx",
        );
        let prepared = PreparedModule::standalone("update-inherited.nx", module.clone());
        let user_update = find_record(&module, "User.Update");
        let shape = effective_record_shape(&prepared, user_update).expect("Shape should resolve");

        let names = shape
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["name", "email"]);
        assert!(shape
            .fields
            .iter()
            .all(|field| !field.is_required && field.default.is_none()));
        assert!(prepared_record_validation_messages(&module).is_empty());
    }

    #[test]
    fn extending_an_update_record_is_rejected() {
        let module = lower_source(
            r#"
            type User = { name:string }
            action Rename extends User.Update = { }
            "#,
            "update-extends.nx",
        );
        let messages = prepared_record_validation_messages(&module);
        assert!(
            messages
                .iter()
                .any(|message| message.contains("'User.Update' is a derived update record")),
            "Expected the update-record base diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn inline_emit_named_update_is_reserved() {
        let module = lower_source(
            r#"
            component <Form emits { Update { value:string } } /> = { <Panel /> }
            "#,
            "update-reserved-inline.nx",
        );
        let messages = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages.iter().any(|message| message
                .contains("'Form.Update' is reserved for the component's derived update record")),
            "Expected the reservation diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn shared_action_named_update_cannot_be_emitted() {
        let module = lower_source(
            r#"
            action Update = { value:string }
            component <Form emits { Update } /> = { <Panel /> }
            "#,
            "update-reserved-shared.nx",
        );
        let messages = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("cannot emit 'Update'")
                    && message.contains("'onUpdate'")),
            "Expected the reservation diagnostic, got {:?}",
            messages
        );
    }

    fn handler_owner_on(module: &LoweredModule, element: crate::ElementId) -> Option<Name> {
        let property = &module.element(element).properties[0];
        match module.expr(property.value) {
            Expr::ActionHandler { owner, .. } => owner.clone(),
            other => panic!("Expected action handler, got {:?}", other),
        }
    }

    #[test]
    fn handlers_record_the_component_they_are_bound_in() {
        let module = lower_source(
            r#"
            external component <Button emits { Tapped } />
            component <Counter /> = {
              state { count:int = 0 }
              <Button onTapped=<Update count={count + 1} /> />
            }
            let render() = <Button onTapped=<Counter.Update count=1 /> />
            "#,
            "handler-owner.nx",
        );
        assert!(
            module.diagnostics().is_empty(),
            "Expected no lowering diagnostics, got {:?}",
            module.diagnostics()
        );

        let Some(Item::Component(counter)) = module.find_item("Counter") else {
            panic!("Expected Counter component");
        };
        let Expr::Element { element, .. } = module.expr(counter.body.expect("Counter body")) else {
            panic!("Expected Counter body element");
        };
        assert_eq!(
            handler_owner_on(&module, *element)
                .as_ref()
                .map(Name::as_str),
            Some("Counter")
        );

        // The bare `Update` inside the handler is the component's own update record.
        let property = &module.element(*element).properties[0];
        let Expr::ActionHandler { body, .. } = module.expr(property.value) else {
            panic!("Expected action handler");
        };
        match module.expr(*body) {
            Expr::RecordLiteral { record, .. } => assert_eq!(record.as_str(), "Counter.Update"),
            other => panic!("Expected Counter.Update record literal, got {:?}", other),
        }

        let Some(Item::Function(render)) = module.find_item("render") else {
            panic!("Expected render function");
        };
        let Expr::Element { element, .. } = module.expr(render.body) else {
            panic!("Expected render element");
        };
        assert_eq!(handler_owner_on(&module, *element), None);
    }

    #[test]
    fn bare_update_takes_precedence_over_a_same_named_declaration_inside_a_component() {
        let module = lower_source(
            r#"
            type Update = { note:string }
            let note() = <Update note="kept" />
            component <Editor /> = {
              state { pending:Editor.Update = <Update /> }
              <Panel />
            }
            "#,
            "update-precedence.nx",
        );
        assert!(
            module.diagnostics().is_empty(),
            "{:?}",
            module.diagnostics()
        );

        let Some(Item::Component(editor)) = module.find_item("Editor") else {
            panic!("Expected Editor component");
        };
        let default = editor.state[0].default.expect("state default");
        match module.expr(default) {
            Expr::RecordLiteral { record, .. } => assert_eq!(record.as_str(), "Editor.Update"),
            other => panic!("Expected Editor.Update record literal, got {:?}", other),
        }

        // Outside a component the bare name keeps meaning the declaration named `Update`.
        let Some(Item::Function(note)) = module.find_item("note") else {
            panic!("Expected note function");
        };
        match module.expr(note.body) {
            Expr::RecordLiteral { record, .. } => assert_eq!(record.as_str(), "Update"),
            other => panic!("Expected Update record literal, got {:?}", other),
        }
    }

    fn find_union<'a>(module: &'a LoweredModule, name: &str) -> &'a UnionDef {
        match module.find_item(name) {
            Some(Item::Union(union)) => union,
            other => panic!("Expected union '{}', got {:?}", name, other),
        }
    }

    fn case_names(union: &UnionDef) -> Vec<&str> {
        union.cases.iter().map(|case| case.name.as_str()).collect()
    }

    /// Flattens an identifier and member-access chain back to its dotted spelling.
    fn dotted_name(module: &LoweredModule, expr: ExprId) -> String {
        match module.expr(expr) {
            Expr::Ident(name) => name.as_str().to_string(),
            Expr::Member { base, member, .. } => {
                format!("{}.{}", dotted_name(module, *base), member.as_str())
            }
            other => panic!("Expected a dotted name, got {:?}", other),
        }
    }

    #[test]
    fn property_union_of_a_record_lists_its_fields() {
        let module = lower_source(
            r#"
            type User = { name:string email:string? }
            "#,
            "property-record.nx",
        );
        let union = find_union(&module, "User.Property");
        assert_eq!(union.property_target().map(Name::as_str), Some("User"));
        assert_eq!(case_names(union), vec!["name", "email"]);
        assert!(union.is_constant_union());
        assert_eq!(union.visibility, Visibility::Internal);
    }

    #[test]
    fn property_union_of_a_component_is_derived_from_state_and_never_props() {
        let module = lower_source(
            r#"
            component <Counter step:int = 1 /> = { state { count:int = 0 } <Label /> }
            component <Form /> = { <Panel /> }
            "#,
            "property-component.nx",
        );
        let union = find_union(&module, "Counter.Property");
        assert_eq!(union.property_target().map(Name::as_str), Some("Counter"));
        assert_eq!(case_names(union), vec!["count"]);
        assert!(
            module.find_item("Form.Property").is_none(),
            "A component without state has no property union"
        );
    }

    #[test]
    fn property_union_of_an_action_and_an_inline_emit_is_synthesized() {
        let module = lower_source(
            r#"
            action Rename = { name:string }
            component <Form emits { Submit { value:string } } /> = { <Panel /> }
            "#,
            "property-action.nx",
        );
        assert_eq!(
            case_names(find_union(&module, "Rename.Property")),
            vec!["name"]
        );
        assert_eq!(
            case_names(find_union(&module, "Form.Submit.Property")),
            vec!["value"]
        );
    }

    #[test]
    fn property_union_includes_inherited_fields_first_once_completed() {
        let module = lower_source(
            r#"
            abstract type Named = { name:string }
            type User extends Named = { email:string }
            "#,
            "property-inherited.nx",
        );
        // Lowering sees only the declared fields; the base chain is resolved once the module is
        // prepared, exactly as the update record's effective shape is.
        assert_eq!(
            case_names(find_union(&module, "User.Property")),
            vec!["email"]
        );

        let mut prepared = PreparedModule::standalone("property-inherited.nx", module);
        crate::complete_property_unions(&mut prepared);
        let union = find_union(prepared.raw_module(), "User.Property");
        assert_eq!(case_names(union), vec!["name", "email"]);
        assert_eq!(
            case_names(find_union(prepared.raw_module(), "Named.Property")),
            vec!["name"]
        );
    }

    #[test]
    fn derived_items_follow_every_authored_item_with_update_records_first() {
        let module = lower_source(
            r#"
            type User = { name:string }
            component <Counter /> = { state { count:int = 0 } <Label /> }
            let value = 1
            "#,
            "property-order.nx",
        );
        let names = module
            .items()
            .iter()
            .map(|item| item.name().as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "User",
                "Counter",
                "value",
                "User.Update",
                "Counter.Update",
                "User.Property",
                "Counter.Property",
            ]
        );
    }

    #[test]
    fn inline_emit_named_property_is_reserved() {
        let module = lower_source(
            r#"
            component <Form emits { Property { name:string } } /> = { <Panel /> }
            "#,
            "property-reserved-inline.nx",
        );
        let messages = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages.iter().any(|message| message.contains(
                "'Form.Property' is reserved for the component's derived property union"
            )),
            "Expected the reservation diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn shared_action_named_property_cannot_be_emitted() {
        let module = lower_source(
            r#"
            action Property = { name:string }
            component <Form emits { Property } /> = { <Panel /> }
            "#,
            "property-reserved-shared.nx",
        );
        let messages = module
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("cannot emit 'Property'")
                    && message.contains("'Form.Property'")),
            "Expected the reservation diagnostic, got {:?}",
            messages
        );
    }

    #[test]
    fn property_union_cannot_be_extended() {
        let module = lower_source(
            r#"
            type User = { name:string }
            type Extra extends User.Property = | more
            type Wide extends User.Property = { more:string }
            "#,
            "property-extends.nx",
        );
        let union_messages = prepared_union_validation_messages(&module);
        assert!(
            union_messages.iter().any(|message| message
                .contains("'User.Property' is a derived property union and cannot be extended")),
            "Expected the property-union base diagnostic, got {:?}",
            union_messages
        );
        let record_messages = prepared_record_validation_messages(&module);
        assert!(
            record_messages.iter().any(|message| message
                .contains("'User.Property' is a derived property union and cannot be extended")),
            "Expected the property-union base diagnostic, got {:?}",
            record_messages
        );
    }

    #[test]
    fn bare_property_in_a_state_default_and_type_resolves_to_the_component() {
        let module = lower_source(
            r#"
            component <Counter /> = {
              state { count:int = 0 key:Property = {Property.count} }
              <Label />
            }
            "#,
            "property-state.nx",
        );
        assert!(
            module.diagnostics().is_empty(),
            "{:?}",
            module.diagnostics()
        );
        let Some(Item::Component(counter)) = module.find_item("Counter") else {
            panic!("Expected Counter component");
        };
        assert_eq!(counter.state[1].ty, TypeRef::name("Counter.Property"));
        let default = counter.state[1].default.expect("state default");
        assert_eq!(dotted_name(&module, default), "Counter.Property.count");

        // The rewrite reached the update record's field type too.
        let update = find_record(&module, "Counter.Update");
        assert_eq!(update.properties[1].ty, TypeRef::name("Counter.Property"));
    }

    #[test]
    fn bare_property_in_a_handler_body_resolves_to_the_component() {
        let module = lower_source(
            r#"
            external component <Grid emits { Sorted { by:string } } />
            component <People /> = {
              state { sortBy:People.Property = {Property.name} name:string = "" }
              <Grid onSorted=<Update sortBy={Property.name} /> />
            }
            "#,
            "property-handler.nx",
        );
        assert!(
            module.diagnostics().is_empty(),
            "{:?}",
            module.diagnostics()
        );
        let Some(Item::Component(people)) = module.find_item("People") else {
            panic!("Expected People component");
        };
        let Expr::Element { element, .. } = module.expr(people.body.expect("People body")) else {
            panic!("Expected People body element");
        };
        let property = &module.element(*element).properties[0];
        let Expr::ActionHandler { body, .. } = module.expr(property.value) else {
            panic!("Expected action handler");
        };
        let Expr::RecordLiteral {
            record, properties, ..
        } = module.expr(*body)
        else {
            panic!("Expected update record literal");
        };
        assert_eq!(record.as_str(), "People.Update");
        assert_eq!(
            dotted_name(&module, properties[0].value),
            "People.Property.name"
        );
    }

    #[test]
    fn bare_property_outside_a_component_is_left_as_written() {
        let module = lower_source(
            r#"
            type User = { name:string }
            let key = {Property.name}
            "#,
            "property-root.nx",
        );
        let Some(Item::Value(key)) = module.find_item("key") else {
            panic!("Expected key value");
        };
        assert_eq!(dotted_name(&module, key.value), "Property.name");
    }

    #[test]
    fn bare_property_takes_precedence_over_a_same_named_declaration_inside_a_component() {
        let module = lower_source(
            r#"
            type Property = { note:string }
            let note() = {Property.note}
            component <Counter /> = {
              state { count:int = 0 key:Counter.Property = {Property.count} }
              <Label />
            }
            "#,
            "property-precedence.nx",
        );
        let Some(Item::Component(counter)) = module.find_item("Counter") else {
            panic!("Expected Counter component");
        };
        let default = counter.state[1].default.expect("state default");
        assert_eq!(dotted_name(&module, default), "Counter.Property.count");

        let Some(Item::Function(note)) = module.find_item("note") else {
            panic!("Expected note function");
        };
        assert_eq!(dotted_name(&module, note.body), "Property.note");
    }
}
