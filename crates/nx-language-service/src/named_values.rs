//! Named values: the values in a document's scope that a property of an element can take.
//!
//! <para>A design system names its colors, sizes and spacings once, as `let`s, and a document
//! refers to them rather than repeating their literals. An editor that offers a property's values
//! offers those names too, so that picking one writes a reference and the document follows the
//! design system. Which names fit is the type checker's answer, so a `Color` alias, a subtype or a
//! union case all count as it would in a hand-written reference.</para>
//!
//! <para>The shape is unstable: it is published for the viewer to build on, and a later change
//! commits to it.</para>

use crate::source_tree::{base_chain, item_at, property_types, SourceFlag, SourceRole};
use crate::{
    hover, type_ref_display, DeclarationOrigin, DocumentUri, SnapshotError, WorkspaceDeclarations,
    WorkspaceSnapshot,
};
use nx_hir::ast::{Expr, Literal};
use nx_hir::{Doc, ExprId, Item, LoweredModule};
use nx_types::{InferenceContext, Type};
use serde::{Deserialize, Serialize};

/// One named value a property can take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedValue {
    /// The reference as the document would write it: identifiers joined by dots, such as
    /// `accent` or `theme.accent`.
    pub path: String,
    /// The value's declared or inferred type, spelled in NX.
    #[serde(rename = "type")]
    pub ty: String,
    /// The doc comment, rendered as hover renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// The value's source text, when it is a literal, such as `"#c8a27a"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl WorkspaceSnapshot {
    /// Answers the named values in the scope of the document at `uri` that the property
    /// `property` of the element at `key` can take: each parameterless `let` the document declares
    /// or imports whose type the property accepts, and each such member of a `let` whose value is
    /// a record, one level deep. The document's own come first, then each import's, each in
    /// declaration order. A key that names no element, or a property the element does not have,
    /// answers an empty list.
    pub fn named_values(
        &self,
        uri: &DocumentUri,
        key: &str,
        property: &str,
    ) -> Result<Vec<NamedValue>, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let identity = document.identity.as_str().to_string();
        let tree = self.source_tree(uri)?;
        let workspace = self.workspace_declarations();
        Ok(named_values_in(&workspace, &identity, &tree, key, property).unwrap_or_default())
    }
}

fn named_values_in(
    workspace: &WorkspaceDeclarations,
    identity: &str,
    tree: &crate::SourceTree,
    key: &str,
    property: &str,
) -> Option<Vec<NamedValue>> {
    let node = tree.nodes.iter().find(|node| node.key == key)?;
    if node.role != SourceRole::Element {
        return None;
    }
    let declaration = tree.declarations.get(node.declaration? as usize)?;
    let declared = declaration
        .properties
        .iter()
        .find(|candidate| candidate.name == property)?;
    if declared.flags.contains(&SourceFlag::Content) {
        return None;
    }
    // The element's declaration, and the property's type as the module that declared it writes it.
    let origin = workspace
        .by_origin
        .keys()
        .filter(|origin| origin.0 == declaration.module)
        .find(|origin| {
            matches!(
                item_at(workspace, origin),
                Some(item @ (Item::Record(_) | Item::Component(_) | Item::Function(_)))
                    if item.name().as_str() == declaration.name
            )
        })?
        .clone();
    let chain = base_chain(workspace, &origin);
    let (_, declaring_module, _, written) = property_types(workspace, &origin, &chain)
        .into_iter()
        .find(|(name, _, _, _)| name == property)?;

    let prepared = workspace.artifact(identity)?.prepared_module.clone()?;
    let mut context = InferenceContext::new(&prepared);
    let expected = context.resolve_written_type(Some(&declaring_module), &written);
    if matches!(expected, Type::Error | Type::Unknown) {
        return Some(Vec::new());
    }
    // An optional property admits absence, as the checker reads `p?:T`: an optional member or a
    // `T?` value fits it.
    let expected = nx_types::read_type(&expected, declared.flags.contains(&SourceFlag::Optional));

    // The declaration the element is in cannot refer to itself.
    let mut root = node;
    while let Some(parent) = root
        .parent
        .and_then(|parent| tree.nodes.get(parent as usize))
    {
        root = parent;
    }
    let enclosing = root.name.clone();

    // The document's own values first, then each import's, each in declaration order.
    let mut candidates = workspace
        .visible_bindings
        .get(identity)?
        .iter()
        .filter(|(name, origin)| {
            matches!(item_at(workspace, origin), Some(Item::Value(_)))
                && !(origin.0 == identity && Some(name) == enclosing.as_ref())
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, origin)| {
        let span = item_at(workspace, origin).map(|item| item.span().start());
        (origin.0 != identity, origin.0.clone(), span)
    });
    candidates.dedup_by(|left, right| left.0 == right.0);

    let mut values = Vec::new();
    for (name, origin) in candidates {
        let Some(Item::Value(value)) = item_at(workspace, &origin) else {
            continue;
        };
        let Some(lowered) = lowered(workspace, &origin) else {
            continue;
        };
        let actual = match &value.ty {
            Some(ty) => context.resolve_written_type(Some(&origin.0), ty),
            None => workspace
                .artifact(&origin.0)
                .and_then(|artifact| artifact.type_env.get_expr_type(value.value).cloned())
                .unwrap_or(Type::Unknown),
        };
        if matches!(actual, Type::Error | Type::Unknown) {
            continue;
        }
        if context.satisfies(&actual, &expected) {
            values.push(NamedValue {
                path: name.clone(),
                ty: value
                    .ty
                    .as_ref()
                    .map(type_ref_display)
                    .unwrap_or_else(|| actual.to_string()),
                doc: render_doc(workspace, &origin.0, value.doc.as_ref()),
                value: literal_text(workspace, &origin.0, lowered, value.value),
            });
        }
        // One level of a record's members.
        let Type::Named(named) = actual.item() else {
            continue;
        };
        let Some(record) = named.origin() else {
            continue;
        };
        let Some(record_module) = workspace
            .artifact(record.module_identity())
            .and_then(|artifact| artifact.prepared_module.clone())
        else {
            continue;
        };
        let Ok(Some(shape)) = nx_hir::effective_record_shape_at(&record_module, record) else {
            continue;
        };
        for field in &shape.fields {
            if field.is_content {
                continue;
            }
            let field_ty = context.resolve_written_type(Some(&field.module_identity), &field.ty);
            if matches!(field_ty, Type::Error | Type::Unknown) {
                continue;
            }
            // An optional member may be absent, so it fits only a site that admits that.
            let field_ty = if field.optional {
                nx_types::read_type(&field_ty, true)
            } else {
                field_ty
            };
            if !context.satisfies(&field_ty, &expected) {
                continue;
            }
            let written = member_value(lowered, value.value, field.name.as_str())
                .and_then(|expr| literal_text(workspace, &origin.0, lowered, expr))
                .or_else(|| {
                    let default = field.default.as_ref()?;
                    let module = lowered_in(workspace, &default.module_identity)?;
                    literal_text(workspace, &default.module_identity, module, default.expr_id)
                });
            values.push(NamedValue {
                path: format!("{name}.{}", field.name),
                ty: type_ref_display(&field.ty),
                doc: render_doc(workspace, &field.module_identity, field.doc.as_ref()),
                value: written,
            });
        }
    }
    Some(values)
}

fn lowered<'w>(
    workspace: &'w WorkspaceDeclarations,
    origin: &DeclarationOrigin,
) -> Option<&'w LoweredModule> {
    lowered_in(workspace, &origin.0)
}

fn lowered_in<'w>(workspace: &'w WorkspaceDeclarations, module: &str) -> Option<&'w LoweredModule> {
    workspace.artifact(module)?.lowered_module.as_deref()
}

fn render_doc(
    workspace: &WorkspaceDeclarations,
    module: &str,
    doc: Option<&Doc>,
) -> Option<String> {
    let doc = doc?;
    let links = workspace
        .artifact(module)
        .map(|artifact| artifact.doc_links.as_slice())
        .unwrap_or_default();
    Some(hover::render_doc(doc, links))
}

/// The expression an element or record literal writes for the property `name`.
fn member_value(module: &LoweredModule, expr: ExprId, name: &str) -> Option<ExprId> {
    match module.expr(expr) {
        Expr::Element { element, .. } => module
            .element(*element)
            .properties
            .iter()
            .find(|property| property.key.as_str() == name)
            .map(|property| property.value),
        Expr::RecordLiteral { properties, .. } => properties
            .iter()
            .find(|property| property.name.as_str() == name)
            .map(|property| property.value),
        _ => None,
    }
}

/// The source text of a literal expression, a negated number included.
fn literal_text(
    workspace: &WorkspaceDeclarations,
    module: &str,
    lowered: &LoweredModule,
    expr: ExprId,
) -> Option<String> {
    let literal = match lowered.expr(expr) {
        Expr::Literal(_) => true,
        Expr::UnaryOp { expr: operand, .. } => {
            matches!(
                lowered.expr(*operand),
                Expr::Literal(Literal::Int(_) | Literal::Float(_))
            )
        }
        _ => false,
    };
    if !literal {
        return None;
    }
    let span = lowered.known_expr_span(expr)?;
    let source = workspace.sources.get(module)?;
    source
        .get(usize::from(span.start())..usize::from(span.end()))
        .map(str::to_string)
}
