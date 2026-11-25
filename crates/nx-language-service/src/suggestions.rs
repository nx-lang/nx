//! Value suggestions: the named values a document can reach, for each property type that can
//! take one, so a viewer can offer `env.space.md` for a padding without a query of its own.
//!
//! <para>A property's suggestion key comes from its declared type: an alias of a primitive is its
//! own key, so spacing and radius differ although both are numbers, and a record, union or action
//! type is the key of every alias that names it. A named value matches a key by the same rule,
//! read from its declared type or, without one, from its inferred type.</para>
//!
//! <para>The candidates are the values the module's top-level scope resolves without a
//! diagnostic, grouped by where they come from, each followed into its record fields. A preview
//! is the value at the defaults, found by following names, defaults and constructions without
//! running anything.</para>

use super::{base_chain, item_at, swatch, TreeBuilder};
use crate::environment::{binding_origin, qualified_name};
use crate::{type_ref_display, DeclarationOrigin, DocumentScope, WorkspaceDeclarations};
use nx_hir::{
    ast::{Expr, Literal, Occurrence, TypeRef, UnOp},
    environment_value_name, ExprId, Import, ImportKind, Item, PreparedBindingOrigin,
    PreparedNamespace, Visibility,
};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// How many record fields deep a value's fields are followed.
const FIELD_DEPTH: usize = 4;

/// How many steps a preview follows before it gives up.
const PREVIEW_STEPS: usize = 16;

/// One named value a document can reach, with the key of the properties it fits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSuggestion {
    /// The value as the document would write it: `env.space.md`, `brandRed`,
    /// `d.standardPalette.blue600`.
    pub name: String,
    /// The suggestion key of the properties it fits.
    pub key: String,
    /// Where it comes from.
    pub group: SourceSuggestionGroup,
    /// For a parameter or state field, the table index of the top-level declaration whose body
    /// may use it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<u32>,
    /// Its type, spelled in NX.
    #[serde(rename = "type")]
    pub ty: String,
    /// The table index of the declaration it starts from.
    pub declaration: u32,
    /// The doc comment of the field or declaration it names, rendered as hover renders it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Its value at the defaults, when that can be found without running the program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<SourcePreview>,
    /// The table indices of the top-level declarations inside which it is not offered.
    #[serde(rename = "self", default, skip_serializing_if = "Vec::is_empty")]
    pub self_: Vec<u32>,
}

/// The group a suggestion belongs to, and the group's label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSuggestionGroup {
    /// Which kind of group.
    pub kind: SourceSuggestionGroupKind,
    /// The group's label: `Environment`, a declaration's name, `This document`, a peer file's
    /// path or an import's path.
    pub label: String,
}

/// Where a suggestion comes from, in the order groups are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceSuggestionGroupKind {
    /// An environment value or one of its fields.
    Environment,
    /// A parameter or state field of one top-level declaration.
    Declaration,
    /// A top-level value of this document.
    Document,
    /// A value of another file the module sees without an import.
    Peer,
    /// A value an import makes visible.
    Import,
}

/// A value found without running the program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePreview {
    /// What kind of value it is.
    pub kind: SourcePreviewKind,
    /// The value: a string's text, a number as written, `true` or `false`, a case's name, or a
    /// color's swatch text.
    pub value: String,
    /// Whether a color is outside sRGB.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub outside_srgb: bool,
    /// A color inside sRGB as the hex literal that writes it: `#rrggbb`, with two more digits when
    /// it is not opaque. Absent for any other value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
}

/// The kind of a preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePreviewKind {
    /// A string literal.
    String,
    /// A number literal.
    Number,
    /// `true` or `false`.
    Boolean,
    /// A bare union case.
    Case,
    /// A color the color tooling recognizes.
    Color,
}

/// What a type is, read through its aliases.
struct ResolvedType {
    /// The aliases on the way, the one written first.
    aliases: Vec<DeclarationOrigin>,
    /// The record, union or action at the end, when it ends at one.
    declaration: Option<DeclarationOrigin>,
    /// Whether the chain ends at a primitive type.
    primitive: bool,
    /// Whether the type, or an alias on the way, is optional.
    optional: bool,
    /// Whether the type is a sequence or any other type without a key.
    other: bool,
}

/// The type of a candidate: as declared, in the module that declares it, or as inferred.
enum CandidateType {
    Declared(String, TypeRef),
    Inferred(nx_types::Type),
}

/// A named value the walk starts from, before its fields are followed.
struct Root {
    name: String,
    group: SourceSuggestionGroupKind,
    label: String,
    scope: Option<DeclarationOrigin>,
    start: DeclarationOrigin,
    ty: CandidateType,
    doc: Option<String>,
    /// The module and expression of its value, or of its default.
    value: Option<(String, ExprId)>,
}

/// Whether `name` is a primitive type's name.
fn is_primitive(name: &str) -> bool {
    nx_hir::ast::PrimitiveType::from_name(name).is_some()
        || matches!(name, "uitext" | "float" | "object" | "void")
}

/// The type `ty`, written in `module`, read through its aliases.
fn resolve_type(workspace: &WorkspaceDeclarations, module: &str, ty: &TypeRef) -> ResolvedType {
    let mut resolved = ResolvedType {
        aliases: Vec::new(),
        declaration: None,
        primitive: false,
        optional: false,
        other: false,
    };
    let mut module = module.to_string();
    let mut ty = ty.clone();
    let mut first = true;
    loop {
        match &ty {
            TypeRef::Seq { inner, occ } if *occ == Occurrence::OPTIONAL => {
                resolved.optional = true;
                ty = (**inner).clone();
                continue;
            }
            // A constraint written inline is a bare primitive's, which has no key; one an alias
            // carries is the alias's own.
            TypeRef::Constrained { base, .. } if !first => {
                ty = (**base).clone();
                continue;
            }
            TypeRef::Name(name) => {
                let Some(origin) = workspace
                    .type_namespaces
                    .get(&module)
                    .and_then(|namespace| namespace.get(name.as_str()))
                    .cloned()
                else {
                    resolved.primitive = is_primitive(name.as_str());
                    resolved.other = !resolved.primitive;
                    return resolved;
                };
                match item_at(workspace, &origin) {
                    Some(Item::TypeAlias(alias)) => {
                        if resolved.aliases.contains(&origin) {
                            resolved.other = true;
                            return resolved;
                        }
                        resolved.aliases.push(origin.clone());
                        module = origin.0.clone();
                        ty = alias.ty.clone();
                        first = false;
                    }
                    Some(Item::Record(_) | Item::Union(_)) => {
                        resolved.declaration = Some(origin);
                        return resolved;
                    }
                    _ => {
                        resolved.other = true;
                        return resolved;
                    }
                }
            }
            _ => {
                resolved.other = true;
                return resolved;
            }
        }
    }
}

/// The suggestion key of a property typed `ty` in `module`, with a trailing `?` removed.
fn property_key(
    workspace: &WorkspaceDeclarations,
    module: &str,
    ty: &TypeRef,
) -> Option<DeclarationOrigin> {
    let resolved = resolve_type(workspace, module, ty);
    if resolved.other {
        return None;
    }
    if let Some(declaration) = resolved.declaration {
        return Some(declaration);
    }
    if resolved.primitive {
        return resolved.aliases.first().cloned();
    }
    None
}

/// The keys a value of type `ty` matches.
fn candidate_keys(workspace: &WorkspaceDeclarations, ty: &CandidateType) -> Vec<DeclarationOrigin> {
    let declaration = match ty {
        CandidateType::Declared(module, ty) => {
            let resolved = resolve_type(workspace, module, ty);
            if resolved.optional || resolved.other {
                return Vec::new();
            }
            if resolved.primitive {
                return resolved.aliases;
            }
            resolved.declaration
        }
        CandidateType::Inferred(ty) => match ty {
            nx_types::Type::Named(named) => named.origin().cloned(),
            nx_types::Type::Union(union) => union.origin().cloned(),
            nx_types::Type::UnionCase(case) => case.origin().cloned(),
            _ => None,
        }
        .map(|origin| (origin.module_identity().to_string(), origin.definition_id())),
    };
    let Some(declaration) = declaration else {
        return Vec::new();
    };
    if !matches!(
        item_at(workspace, &declaration),
        Some(Item::Record(_) | Item::Union(_))
    ) {
        return Vec::new();
    }
    let mut keys = vec![declaration.clone()];
    keys.extend(base_chain(workspace, &declaration));
    keys
}

/// A key as the answer spells it.
fn key_text(workspace: &WorkspaceDeclarations, key: &DeclarationOrigin) -> String {
    let name = item_at(workspace, key)
        .map(|item| item.name().as_str().to_string())
        .unwrap_or_default();
    format!("{}#{name}", key.0)
}

/// One field of a record: its name, declaring module, type, whether it is optional, and doc.
struct Field<'w> {
    name: String,
    module: String,
    ty: &'w TypeRef,
    optional: bool,
    doc: Option<&'w nx_hir::Doc>,
}

/// The fields of the record at `origin`, inherited ones first, in declaration order.
fn record_fields<'w>(
    workspace: &'w WorkspaceDeclarations,
    origin: &DeclarationOrigin,
) -> Vec<Field<'w>> {
    let chain = base_chain(workspace, origin);
    let mut fields: Vec<Field<'w>> = Vec::new();
    for declaring in chain.iter().rev().chain(std::iter::once(origin)) {
        let Some(Item::Record(record)) = item_at(workspace, declaring) else {
            continue;
        };
        for field in &record.properties {
            let next = Field {
                name: field.name.as_str().to_string(),
                module: declaring.0.clone(),
                ty: &field.ty,
                optional: field.optional,
                doc: field.doc.as_ref(),
            };
            match fields
                .iter_mut()
                .find(|existing| existing.name == next.name)
            {
                Some(existing) => *existing = next,
                None => fields.push(next),
            }
        }
    }
    fields
}

/// The path from the document's directory to the module with identity `target`: `./brand.nx`.
pub(super) fn relative_path(from: &str, target: &str) -> String {
    let from_dirs: Vec<&str> = from.split('/').collect::<Vec<_>>();
    let from_dirs = &from_dirs[..from_dirs.len().saturating_sub(1)];
    let target_parts: Vec<&str> = target.split('/').collect();
    let common = from_dirs
        .iter()
        .zip(target_parts.iter())
        .take_while(|(lhs, rhs)| lhs == rhs)
        .count();
    let ups = from_dirs.len() - common;
    let rest = target_parts[common..].join("/");
    if ups == 0 {
        format!("./{rest}")
    } else {
        format!("{}{rest}", "../".repeat(ups))
    }
}

/// The index of the written import of the module with identity `from` that makes `visible`
/// visible, with `target` its declaring module.
fn import_for(from: &str, imports: &[Import], visible: &str, target: &str) -> Option<usize> {
    let written = |import: &Import| !import.span.is_empty();
    let reaches = |import: &Import| import_reaches(from, &import.library_path, target);
    for (index, import) in imports.iter().enumerate() {
        if !written(import) {
            continue;
        }
        match &import.kind {
            ImportKind::Selective { entries } => {
                if entries.iter().any(|entry| {
                    let spelled = match &entry.qualifier {
                        Some(qualifier) => format!("{qualifier}.{}", entry.name),
                        None => entry.name.as_str().to_string(),
                    };
                    spelled == visible
                }) && reaches(import)
                {
                    return Some(index);
                }
            }
            ImportKind::Wildcard { alias: Some(alias) } => {
                if visible.starts_with(&format!("{alias}.")) && reaches(import) {
                    return Some(index);
                }
            }
            ImportKind::Wildcard { alias: None } => {}
        }
    }
    imports.iter().position(|import| {
        written(import)
            && matches!(import.kind, ImportKind::Wildcard { alias: None })
            && !visible.contains('.')
            && reaches(import)
    })
}

/// The names the module's own imports leave ambiguous, which reading would report.
fn ambiguous_names(artifact: &nx_types::ModuleArtifact) -> FxHashSet<String> {
    artifact
        .prepared_module
        .as_deref()
        .map(|prepared| prepared.ambiguous_names().iter().cloned().collect())
        .unwrap_or_default()
}

/// The identity a written import path names, from the module with identity `from`: `./brand.nx`
/// from `tenant/main.nx` is `tenant/brand.nx`. A library path is returned as written.
fn import_target(from: &str, path: &str) -> String {
    let path = path.trim();
    if !(path.starts_with("./") || path.starts_with("../")) {
        return path.to_string();
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for segment in path.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Whether the module with identity `target` is what the import of `path` from `from` reaches:
/// the module itself, or a module of the library or directory it names.
fn import_reaches(from: &str, path: &str, target: &str) -> bool {
    let named = import_target(from, path);
    let named = named.trim_end_matches('/');
    target == named
        || target.strip_suffix(".nx") == Some(named)
        || target.starts_with(&format!("{named}/"))
}

/// Every declaration the declaration at `origin` reads, itself included, directly or through the
/// values, environment defaults and functions it names.
fn reads_closure(scope: &DocumentScope, origin: &DeclarationOrigin) -> Vec<DeclarationOrigin> {
    let mut visited: Vec<DeclarationOrigin> = Vec::new();
    let mut seen = FxHashSet::default();
    let mut pending = vec![origin.clone()];
    while let Some(current) = pending.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        visited.push(current.clone());
        let Some(artifact) = scope.workspace.artifact(&current.0) else {
            continue;
        };
        let (Some(lowered), Some(item)) = (
            artifact.lowered_module.clone(),
            scope.item_at_origin(&current),
        ) else {
            continue;
        };
        let span = item.span();
        let locals = artifact
            .prepared_module
            .as_deref()
            .map(nx_hir::local_references)
            .unwrap_or_default();
        for (id, expr) in lowered.exprs() {
            let expr_span = lowered.expr_span(id);
            if expr_span.is_empty() || !span.contains_range(expr_span) {
                continue;
            }
            let target = match expr {
                Expr::EnvRead {
                    binding: Some(binding),
                    ..
                } => binding_origin(scope, &current.0, binding.as_str()),
                Expr::Ident(name) if !locals.contains(&id) => {
                    binding_origin(scope, &current.0, name.as_str())
                }
                Expr::Member { .. } => qualified_name(&lowered, expr)
                    .and_then(|name| binding_origin(scope, &current.0, &name)),
                _ => None,
            };
            if let Some(target) = target {
                pending.push(target);
            }
        }
    }
    visited
}

/// Finds values at the defaults by following names, defaults and constructions.
struct Previewer<'s> {
    scope: &'s DocumentScope,
    steps: usize,
}

impl Previewer<'_> {
    fn lowered(&self, module: &str) -> Option<std::sync::Arc<nx_hir::LoweredModule>> {
        self.scope
            .workspace
            .artifact(module)?
            .lowered_module
            .clone()
    }

    fn step(&mut self) -> Option<()> {
        self.steps += 1;
        (self.steps <= PREVIEW_STEPS).then_some(())
    }

    /// The value a top-level value or environment value at `origin` has at its default.
    fn value_of(&self, origin: &DeclarationOrigin) -> Option<(String, ExprId)> {
        match item_at(&self.scope.workspace, origin)? {
            Item::Value(value) => Some((origin.0.clone(), value.value)),
            _ => None,
        }
    }

    /// Follows references from `id` to the expression that writes the value.
    fn resolve(&mut self, module: &str, id: ExprId) -> Option<(String, ExprId)> {
        let mut module = module.to_string();
        let mut id = id;
        loop {
            self.step()?;
            let lowered = self.lowered(&module)?;
            let next = match lowered.expr(id) {
                Expr::Ident(name) => {
                    let origin = binding_origin(self.scope, &module, name.as_str())?;
                    self.value_of(&origin)?
                }
                Expr::EnvRead {
                    binding: Some(binding),
                    ..
                } => {
                    let origin = binding_origin(self.scope, &module, binding.as_str())?;
                    self.value_of(&origin)?
                }
                expr @ Expr::Member { base, member, .. } => {
                    match qualified_name(&lowered, expr)
                        .and_then(|name| binding_origin(self.scope, &module, &name))
                    {
                        Some(origin) => self.value_of(&origin)?,
                        None => {
                            let (base_module, base) = self.resolve(&module, *base)?;
                            self.field(&base_module, base, member.as_str())?
                        }
                    }
                }
                Expr::Block {
                    stmts,
                    expr: Some(expr),
                    ..
                } if stmts.is_empty() => (module.clone(), *expr),
                Expr::Match {
                    scrutinee,
                    arms,
                    else_branch,
                    ..
                } => {
                    let case = self.environment_case(&module, *scrutinee)?;
                    let arm = arms.iter().find(|arm| {
                        arm.patterns
                            .iter()
                            .any(|pattern| case_name(&lowered, *pattern).as_deref() == Some(&case))
                    });
                    match (arm, else_branch) {
                        (Some(arm), _) => (module.clone(), arm.body),
                        (None, Some(otherwise)) => (module.clone(), *otherwise),
                        (None, None) => return None,
                    }
                }
                _ => return Some((module, id)),
            };
            module = next.0;
            id = next.1;
        }
    }

    /// The case an `env.` read's own default names, when the default is a bare case.
    fn environment_case(&mut self, module: &str, scrutinee: ExprId) -> Option<String> {
        let lowered = self.lowered(module)?;
        let Expr::EnvRead {
            binding: Some(binding),
            ..
        } = lowered.expr(scrutinee)
        else {
            return None;
        };
        let origin = binding_origin(self.scope, module, binding.as_str())?;
        let (default_module, default) = self.value_of(&origin)?;
        case_name(&*self.lowered(&default_module)?, default)
    }

    /// The expression that sets `member` on the construction at `id`, or the field's default.
    fn field(&mut self, module: &str, id: ExprId, member: &str) -> Option<(String, ExprId)> {
        self.step()?;
        let lowered = self.lowered(module)?;
        let workspace = &self.scope.workspace;
        let record = match lowered.expr(id) {
            Expr::Element { element, .. } => {
                let element = lowered.element(*element);
                if let Some(property) = element
                    .properties
                    .iter()
                    .find(|property| property.key.as_str() == member)
                {
                    return Some((module.to_string(), property.value));
                }
                workspace
                    .type_namespaces
                    .get(module)?
                    .get(element.tag.as_str())
                    .cloned()?
            }
            Expr::RecordLiteral {
                record,
                properties,
                declared_in,
                ..
            } => {
                if let Some(property) = properties
                    .iter()
                    .find(|property| property.name.as_str() == member)
                {
                    return Some((module.to_string(), property.value));
                }
                match declared_in {
                    Some(origin) => (origin.module_identity().to_string(), origin.definition_id()),
                    None => workspace
                        .type_namespaces
                        .get(module)?
                        .get(record.as_str())
                        .cloned()?,
                }
            }
            _ => return None,
        };
        let chain = base_chain(workspace, &record);
        for declaring in std::iter::once(&record).chain(chain.iter()) {
            if let Some(Item::Record(definition)) = item_at(workspace, declaring) {
                if let Some(field) = definition
                    .properties
                    .iter()
                    .find(|field| field.name.as_str() == member)
                {
                    return Some((declaring.0.clone(), field.default?));
                }
            }
        }
        None
    }

    /// The preview of the value at `start` and then down `path`.
    fn preview(&mut self, start: (String, ExprId), path: &[String]) -> Option<SourcePreview> {
        let (mut module, mut id) = start;
        for member in path {
            let (resolved_module, resolved) = self.resolve(&module, id)?;
            (module, id) = self.field(&resolved_module, resolved, member)?;
        }
        let (module, id) = self.resolve(&module, id)?;
        self.classify(&module, id)
    }

    /// The preview of a written value: a literal, a bare case, or a color.
    fn classify(&self, module: &str, id: ExprId) -> Option<SourcePreview> {
        let lowered = self.lowered(module)?;
        let workspace = &self.scope.workspace;
        let text = |id: ExprId| -> Option<String> {
            let span = lowered.known_expr_span(id)?;
            let source = workspace.sources.get(module)?;
            source
                .get(usize::from(span.start())..usize::from(span.end()))
                .map(str::to_string)
        };
        let plain = |kind, value: String| {
            Some(SourcePreview {
                kind,
                value,
                outside_srgb: false,
                hex: None,
            })
        };
        match lowered.expr(id) {
            Expr::Literal(Literal::String(value)) => {
                plain(SourcePreviewKind::String, value.to_string())
            }
            Expr::Literal(Literal::Boolean(value)) => {
                plain(SourcePreviewKind::Boolean, value.to_string())
            }
            Expr::Literal(literal) => {
                let fallback = match literal {
                    Literal::Int(value) => value.to_string(),
                    Literal::Int32(value) => value.to_string(),
                    Literal::Float(value) | Literal::Float32(value) => value.0.to_string(),
                    _ => return None,
                };
                plain(SourcePreviewKind::Number, text(id).unwrap_or(fallback))
            }
            Expr::UnaryOp {
                op: UnOp::Neg,
                expr,
                ..
            } if matches!(lowered.expr(*expr), Expr::Literal(_)) => {
                plain(SourcePreviewKind::Number, text(id)?)
            }
            Expr::ResolvedUnionCase { case, .. } => {
                plain(SourcePreviewKind::Case, case.as_str().to_string())
            }
            Expr::ContextualName { name, .. } => {
                plain(SourcePreviewKind::Case, name.as_str().to_string())
            }
            Expr::Element { element, .. } => {
                let written = text(id)?;
                if written.starts_with('#') {
                    return color_preview(nx_value::color::parse(&written));
                }
                let element = lowered.element(*element);
                let (type_name, case) = element.tag.as_str().rsplit_once('.')?;
                let origin = workspace.type_namespaces.get(module)?.get(type_name)?;
                if !is_canonical_color(workspace, origin) {
                    return None;
                }
                let space = nx_value::color::Space::from_case(case)?;
                let mut channels = [0.0, 0.0, 0.0, 1.0];
                let mut seen = [false, false, false, true];
                for property in &element.properties {
                    let Some(channel) = space
                        .keys()
                        .iter()
                        .position(|key| *key == property.key.as_str())
                    else {
                        continue;
                    };
                    channels[channel] = number(&lowered, property.value)?;
                    seen[channel] = true;
                }
                if !seen.iter().all(|seen| *seen) {
                    return None;
                }
                color_preview(nx_value::color::make_in(space, channels))
            }
            Expr::Call { func, args, .. } => {
                let callee = qualified_name(&lowered, lowered.expr(*func))?;
                let origin = binding_origin(self.scope, module, &callee)?;
                let Some(Item::Function(function)) = item_at(workspace, &origin) else {
                    return None;
                };
                if !matches!(
                    origin.0.as_str(),
                    "@nx/color/color.nx" | "@nx/color/operations.nx"
                ) {
                    return None;
                }
                let name = function.name.as_str();
                let args = args
                    .iter()
                    .map(|arg| match lowered.expr(*arg) {
                        Expr::Literal(Literal::String(text)) if name == "parse" => {
                            Some(nx_value::NxValue::String(text.to_string()))
                        }
                        _ if name != "parse" => {
                            number(&lowered, *arg).map(nx_value::NxValue::Float)
                        }
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                color_preview(static_color(name, args)?)
            }
            _ => None,
        }
    }
}

/// The number a literal, or a negated one, writes.
fn number(lowered: &nx_hir::LoweredModule, id: ExprId) -> Option<f64> {
    match lowered.expr(id) {
        Expr::Literal(Literal::Int(value)) => Some(*value as f64),
        Expr::Literal(Literal::Int32(value)) => Some(f64::from(*value)),
        Expr::Literal(Literal::Float(value) | Literal::Float32(value)) => Some(value.0),
        Expr::UnaryOp {
            op: UnOp::Neg,
            expr,
            ..
        } => number(lowered, *expr).map(|value| -value),
        _ => None,
    }
}

/// The case a pattern or a value names, when it is a bare case.
fn case_name(lowered: &nx_hir::LoweredModule, id: ExprId) -> Option<String> {
    match lowered.expr(id) {
        Expr::ResolvedUnionCase { case, .. } => Some(case.as_str().to_string()),
        Expr::ContextualName { name, .. } => Some(name.as_str().to_string()),
        _ => None,
    }
}

/// Whether `origin` is the canonical `Color` union.
fn is_canonical_color(workspace: &WorkspaceDeclarations, origin: &DeclarationOrigin) -> bool {
    let mut current = origin.clone();
    for _ in 0..8 {
        match item_at(workspace, &current) {
            Some(Item::Union(union_def)) => {
                return current.0 == "@nx/color/color.nx" && union_def.name.as_str() == "Color"
            }
            Some(Item::TypeAlias(alias)) => {
                let Some(TypeRef::Name(name)) = Some(&alias.ty) else {
                    return false;
                };
                let Some(next) = workspace
                    .type_namespaces
                    .get(&current.0)
                    .and_then(|namespace| namespace.get(name.as_str()))
                    .cloned()
                else {
                    return false;
                };
                current = next;
            }
            _ => return false,
        }
    }
    false
}

/// The color a `@nx/color` constructor named `name` makes from literal arguments, as the color
/// tooling reads it: `parse`, `rgb`, `rgba`, `srgb`, `displayP3`, `oklab` or `oklch`.
pub(super) fn static_color(
    name: &str,
    mut args: Vec<nx_value::NxValue>,
) -> Option<Result<nx_value::NxValue, nx_value::color::Error>> {
    if !matches!(
        name,
        "parse" | "rgb" | "rgba" | "srgb" | "displayP3" | "oklab" | "oklch"
    ) {
        return None;
    }
    let op = nx_value::color::NAMES
        .iter()
        .position(|candidate| *candidate == name)?;
    if matches!(name, "srgb" | "rgba" | "displayP3" | "oklab" | "oklch") && args.len() == 3 {
        args.push(nx_value::NxValue::Float(1.0));
    }
    Some(nx_value::color::evaluate(op as u32, &args))
}

/// The preview of a color, as a source node's swatch reads it.
/// A parameter or state field: its name, type, whether it is optional, its default and its doc.
type ScopedField<'a> = (
    &'a nx_hir::Name,
    &'a TypeRef,
    bool,
    Option<ExprId>,
    Option<&'a nx_hir::Doc>,
);

fn color_preview(
    value: Result<nx_value::NxValue, nx_value::color::Error>,
) -> Option<SourcePreview> {
    let color = value
        .as_ref()
        .ok()
        .and_then(|value| nx_value::color::read(value).ok());
    let (value, outside_srgb) = swatch(value).ok()?;
    let hex = color.filter(|_| !outside_srgb).map(|color| {
        let [r, g, b, a] = nx_value::color::to_gamut(color, nx_value::color::Space::Srgb)
            .map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8);
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    });
    Some(SourcePreview {
        kind: SourcePreviewKind::Color,
        value,
        outside_srgb,
        hex,
    })
}

impl TreeBuilder<'_> {
    /// Gives each property, state field and case property of the table, and each of this
    /// document's parameterless `let`s, its suggestion key, and lists the named values that match
    /// those keys.
    pub(super) fn attach_suggestions(&mut self) -> Vec<SourceSuggestion> {
        let workspace = std::sync::Arc::clone(&self.scope.workspace);
        let Some(artifact) = workspace.artifact(self.identity) else {
            return Vec::new();
        };
        let Some(lowered) = artifact.lowered_module.clone() else {
            return Vec::new();
        };

        // This document's own values carry keys, so they are listed first.
        for (index, item) in lowered.items().iter().enumerate() {
            if let Item::Value(value) = item {
                if environment_value_name(value.name.as_str()).is_none() {
                    self.entry(&(
                        self.identity.to_string(),
                        nx_hir::LocalDefinitionId::new(index as u32),
                    ));
                }
            }
        }

        // Keys, for the declarations listed so far only.
        let mut used: FxHashSet<DeclarationOrigin> = FxHashSet::default();
        let origins: FxHashMap<u32, DeclarationOrigin> = self
            .table_index
            .iter()
            .map(|(origin, index)| (*index, origin.clone()))
            .collect();
        for index in 0..self.table.len() {
            let Some(origin) = origins.get(&(index as u32)) else {
                continue;
            };
            let Some(item) = item_at(&workspace, origin) else {
                continue;
            };
            let chain = base_chain(&workspace, origin);
            let mut typed: Vec<(String, String, &TypeRef)> = Vec::new();
            for declaring in chain.iter().rev().chain(std::iter::once(origin)) {
                let fields: Vec<(&nx_hir::Name, &TypeRef)> = match item_at(&workspace, declaring) {
                    Some(Item::Record(record)) => record
                        .properties
                        .iter()
                        .map(|field| (&field.name, &field.ty))
                        .collect(),
                    Some(Item::Component(component)) => component
                        .props
                        .iter()
                        .map(|field| (&field.name, &field.ty))
                        .collect(),
                    Some(Item::Function(function)) => function
                        .params
                        .iter()
                        .map(|param| (&param.name, &param.ty))
                        .collect(),
                    _ => Vec::new(),
                };
                for (name, ty) in fields {
                    let next = (name.as_str().to_string(), declaring.0.clone(), ty);
                    match typed.iter_mut().find(|existing| existing.0 == next.0) {
                        Some(existing) => *existing = next,
                        None => typed.push(next),
                    }
                }
            }
            let mut keyed = |key: Option<DeclarationOrigin>| {
                let key = key?;
                let text = key_text(&workspace, &key);
                used.insert(key);
                Some(text)
            };
            let entry = &mut self.table[index];
            for property in &mut entry.properties {
                if let Some((_, module, ty)) =
                    typed.iter().find(|(name, _, _)| *name == property.name)
                {
                    property.suggestion_key = keyed(property_key(&workspace, module, ty));
                }
            }
            match item {
                Item::Component(component) => {
                    for (property, field) in entry.state.iter_mut().zip(&component.state) {
                        property.suggestion_key =
                            keyed(property_key(&workspace, &origin.0, &field.ty));
                    }
                }
                Item::Union(union_def) => {
                    for (case, definition) in entry.cases.iter_mut().zip(&union_def.cases) {
                        for (property, field) in case.properties.iter_mut().zip(&definition.fields)
                        {
                            property.suggestion_key =
                                keyed(property_key(&workspace, &origin.0, &field.ty));
                        }
                    }
                }
                Item::Value(value)
                    if origin.0 == self.identity
                        && environment_value_name(value.name.as_str()).is_none() =>
                {
                    let key = match &value.ty {
                        Some(ty) => property_key(&workspace, &origin.0, ty),
                        None => artifact
                            .type_env
                            .get_expr_type(value.value)
                            .cloned()
                            .map(CandidateType::Inferred)
                            .and_then(|ty| candidate_keys(&workspace, &ty).into_iter().next()),
                    };
                    entry.suggestion_key = keyed(key);
                }
                _ => {}
            }
        }
        if used.is_empty() {
            return Vec::new();
        }

        let roots = self.roots(&workspace, artifact, &lowered);

        // Parameters and state shadow top-level names inside their declaration.
        let mut shadows: Vec<(DeclarationOrigin, FxHashSet<String>)> = Vec::new();
        for (index, item) in lowered.items().iter().enumerate() {
            let names: FxHashSet<String> = match item {
                Item::Component(component) => component
                    .props
                    .iter()
                    .chain(&component.state)
                    .map(|field| field.name.as_str().to_string())
                    .collect(),
                Item::Function(function) => function
                    .params
                    .iter()
                    .map(|param| param.name.as_str().to_string())
                    .collect(),
                _ => continue,
            };
            shadows.push((
                (
                    self.identity.to_string(),
                    nx_hir::LocalDefinitionId::new(index as u32),
                ),
                names,
            ));
        }

        let mut suggestions = Vec::new();
        for root in roots {
            // Matched first, so a value none of whose suggestions some place uses adds nothing to
            // the declaration table, and its reads are not followed.
            let mut found = Vec::new();
            let probe = Context {
                group: SourceSuggestionGroup {
                    kind: root.group,
                    label: root.label.clone(),
                },
                scope: None,
                declaration: 0,
                self_: Vec::new(),
                value: root.value.clone(),
            };
            self.visit(
                &workspace,
                &probe,
                &used,
                root.name.clone(),
                &[],
                &root.ty,
                root.doc.clone(),
                &mut found,
            );
            if found.is_empty() {
                continue;
            }
            let declaration = self.entry(&root.start);
            let scope = root.scope.as_ref().and_then(|scope| self.entry(scope));
            let Some(declaration) = declaration else {
                continue;
            };
            let mut self_origins: Vec<DeclarationOrigin> = Vec::new();
            if root.scope.is_none() {
                for origin in reads_closure(&self.scope, &root.start) {
                    if origin.0 == self.identity
                        && matches!(
                            item_at(&workspace, &origin),
                            Some(Item::Value(_) | Item::Component(_) | Item::Function(_))
                        )
                    {
                        self_origins.push(origin);
                    }
                }
                let first = root.name.split('.').next().unwrap_or_default();
                for (origin, names) in &shadows {
                    if names.contains(first) && !self_origins.contains(origin) {
                        self_origins.push(origin.clone());
                    }
                }
            }
            let mut self_ = self_origins
                .iter()
                .filter_map(|origin| self.entry(origin))
                .collect::<Vec<_>>();
            self_.sort_unstable();
            for mut suggestion in found {
                suggestion.declaration = declaration;
                suggestion.scope = scope;
                suggestion.self_ = self_.clone();
                suggestions.push(suggestion);
            }
        }
        suggestions
    }

    /// Adds the suggestions for one value and, through records, its fields.
    #[allow(clippy::too_many_arguments)]
    fn visit(
        &mut self,
        workspace: &WorkspaceDeclarations,
        context: &Context,
        used: &FxHashSet<DeclarationOrigin>,
        name: String,
        path: &[String],
        ty: &CandidateType,
        doc: Option<String>,
        out: &mut Vec<SourceSuggestion>,
    ) {
        let keys = candidate_keys(workspace, ty);
        let matched: Vec<&DeclarationOrigin> =
            keys.iter().filter(|key| used.contains(*key)).collect();
        if !matched.is_empty() {
            let preview = context.value.clone().and_then(|start| {
                Previewer {
                    scope: &self.scope,
                    steps: 0,
                }
                .preview(start, path)
            });
            let spelled = match ty {
                CandidateType::Declared(_, ty) => type_ref_display(ty),
                CandidateType::Inferred(ty) => ty.to_string(),
            };
            for key in matched {
                out.push(SourceSuggestion {
                    name: name.clone(),
                    key: key_text(workspace, key),
                    group: context.group.clone(),
                    scope: context.scope,
                    ty: spelled.clone(),
                    declaration: context.declaration,
                    doc: doc.clone(),
                    preview: preview.clone(),
                    self_: context.self_.clone(),
                });
            }
        }
        if path.len() >= FIELD_DEPTH {
            return;
        }
        let record = match ty {
            CandidateType::Declared(module, ty) => {
                let resolved = resolve_type(workspace, module, ty);
                if resolved.optional || resolved.other {
                    return;
                }
                resolved.declaration
            }
            CandidateType::Inferred(nx_types::Type::Named(named)) => named
                .origin()
                .map(|origin| (origin.module_identity().to_string(), origin.definition_id())),
            CandidateType::Inferred(_) => None,
        };
        let Some(record) = record else {
            return;
        };
        if !matches!(item_at(workspace, &record), Some(Item::Record(_))) {
            return;
        }
        for field in record_fields(workspace, &record) {
            if field.optional {
                continue;
            }
            let mut field_path = path.to_vec();
            field_path.push(field.name.clone());
            let field_doc = self.scope.render_doc(&field.module, field.doc);
            self.visit(
                workspace,
                context,
                used,
                format!("{name}.{}", field.name),
                &field_path,
                &CandidateType::Declared(field.module.clone(), field.ty.clone()),
                field_doc,
                out,
            );
        }
    }

    /// The named values the module's top-level scope reaches, in group order.
    fn roots(
        &self,
        workspace: &WorkspaceDeclarations,
        artifact: &nx_types::ModuleArtifact,
        lowered: &nx_hir::LoweredModule,
    ) -> Vec<Root> {
        let identity = self.identity;
        let Some(prepared) = artifact.prepared_module.as_deref() else {
            return Vec::new();
        };
        let imports = &lowered.imports;
        let mut ambiguous = ambiguous_names(artifact);
        // Two unaliased imports of workspace modules that export one name bind the first without a
        // diagnostic today; the name is still ambiguous to a reader, so it is left out.
        let mut exported: FxHashMap<String, usize> = FxHashMap::default();
        for import in imports.iter().filter(|import| {
            !import.span.is_empty() && matches!(import.kind, ImportKind::Wildcard { alias: None })
        }) {
            let target = import_target(identity, &import.library_path);
            let target = if workspace.artifact(&target).is_some() {
                target
            } else {
                format!("{target}.nx")
            };
            let Some(module) = workspace
                .artifact(&target)
                .and_then(|artifact| artifact.lowered_module.clone())
            else {
                continue;
            };
            for item in module.items() {
                if matches!(item, Item::Value(_)) && item.visibility() == Visibility::Export {
                    *exported
                        .entry(item.name().as_str().to_string())
                        .or_default() += 1;
                }
            }
        }
        ambiguous.extend(
            exported
                .into_iter()
                .filter(|(_, count)| *count > 1)
                .map(|(name, _)| name),
        );
        let declared = |origin: &DeclarationOrigin, value: &nx_hir::ValueDef| match &value.ty {
            Some(ty) => Some(CandidateType::Declared(origin.0.clone(), ty.clone())),
            None => workspace
                .artifact(&origin.0)?
                .type_env
                .get_expr_type(value.value)
                .cloned()
                .map(CandidateType::Inferred),
        };
        let mut roots = Vec::new();

        // 1. Environment values, as `env.` reads them; an ambiguous one is left out.
        let mut environment: FxHashMap<
            String,
            Vec<(DeclarationOrigin, PreparedBindingOrigin, String)>,
        > = FxHashMap::default();
        for binding in prepared.environment_bindings() {
            let origin = (
                binding.module_identity(identity).to_string(),
                binding.definition_id(),
            );
            let Some(Item::Value(value)) = item_at(workspace, &origin) else {
                continue;
            };
            let Some(name) = environment_value_name(value.name.as_str()) else {
                continue;
            };
            let entries = environment.entry(name.to_string()).or_default();
            if !entries.iter().any(|(existing, _, _)| *existing == origin) {
                entries.push((
                    origin,
                    binding.origin.clone(),
                    binding.visible_name.as_str().to_string(),
                ));
            }
        }
        let mut environment = environment
            .into_iter()
            .filter(|(_, entries)| entries.len() == 1)
            .map(|(name, mut entries)| {
                let (origin, binding, visible) = entries.remove(0);
                let order = match &binding {
                    PreparedBindingOrigin::Local => (0, 0),
                    PreparedBindingOrigin::Peer { module_identity }
                    | PreparedBindingOrigin::Imported { module_identity } => {
                        match import_for(identity, imports, &visible, module_identity) {
                            Some(index) => (2, index),
                            None => (1, 0),
                        }
                    }
                };
                (order, origin, name)
            })
            .collect::<Vec<_>>();
        environment.sort_by(|lhs, rhs| {
            (lhs.0, &lhs.1 .0, lhs.1 .1.as_u32()).cmp(&(rhs.0, &rhs.1 .0, rhs.1 .1.as_u32()))
        });
        for (_, origin, name) in environment {
            let Some(Item::Value(value)) = item_at(workspace, &origin) else {
                continue;
            };
            let Some(ty) = declared(&origin, value) else {
                continue;
            };
            roots.push(Root {
                name: format!("env.{name}"),
                group: SourceSuggestionGroupKind::Environment,
                label: "Environment".to_string(),
                scope: None,
                start: origin.clone(),
                ty,
                doc: self.scope.render_doc(&origin.0, value.doc.as_ref()),
                value: Some((origin.0.clone(), value.value)),
            });
        }

        // 2. Each top-level declaration's parameters and state.
        for (index, item) in lowered.items().iter().enumerate() {
            let origin = (
                identity.to_string(),
                nx_hir::LocalDefinitionId::new(index as u32),
            );
            let fields: Vec<ScopedField<'_>> = match item {
                Item::Component(component) => component
                    .props
                    .iter()
                    .chain(&component.state)
                    .map(|field| {
                        (
                            &field.name,
                            &field.ty,
                            field.optional,
                            field.default,
                            field.doc.as_ref(),
                        )
                    })
                    .collect(),
                Item::Function(function) => function
                    .params
                    .iter()
                    .map(|param| {
                        (
                            &param.name,
                            &param.ty,
                            param.optional,
                            param.default,
                            param.doc.as_ref(),
                        )
                    })
                    .collect(),
                _ => continue,
            };
            for (name, ty, optional, default, doc) in fields {
                if optional {
                    continue;
                }
                roots.push(Root {
                    name: name.as_str().to_string(),
                    group: SourceSuggestionGroupKind::Declaration,
                    label: item.name().as_str().to_string(),
                    scope: Some(origin.clone()),
                    start: origin.clone(),
                    ty: CandidateType::Declared(identity.to_string(), ty.clone()),
                    doc: self.scope.render_doc(identity, doc),
                    value: default.map(|default| (identity.to_string(), default)),
                });
            }
        }

        // 3–5. This document's values, peer files' and imported modules' values.
        let mut values: Vec<(
            SourceSuggestionGroupKind,
            String,
            usize,
            DeclarationOrigin,
            String,
        )> = Vec::new();
        for binding in prepared.bindings(PreparedNamespace::Value) {
            let visible = binding.visible_name.as_str();
            if visible.contains('$') || ambiguous.contains(visible) {
                continue;
            }
            let origin = (
                binding.module_identity(identity).to_string(),
                binding.definition_id(),
            );
            let Some(Item::Value(value)) = item_at(workspace, &origin) else {
                continue;
            };
            if environment_value_name(value.name.as_str()).is_some() {
                continue;
            }
            let (group, label, order) = match &binding.origin {
                PreparedBindingOrigin::Local => (
                    SourceSuggestionGroupKind::Document,
                    "This document".to_string(),
                    0,
                ),
                PreparedBindingOrigin::Peer { module_identity }
                | PreparedBindingOrigin::Imported { module_identity } => {
                    match import_for(identity, imports, visible, module_identity) {
                        Some(index) => (
                            SourceSuggestionGroupKind::Import,
                            imports[index].library_path.trim().to_string(),
                            index,
                        ),
                        // A workspace import binds its names as a peer's; one no import names
                        // is a file of the same library or program.
                        None if matches!(binding.origin, PreparedBindingOrigin::Peer { .. }) => {
                            if value.visibility == Visibility::Private {
                                continue;
                            }
                            (
                                SourceSuggestionGroupKind::Peer,
                                relative_path(identity, module_identity),
                                0,
                            )
                        }
                        None => continue,
                    }
                }
            };
            values.push((group, label, order, origin, visible.to_string()));
        }
        values.sort_by(|lhs, rhs| {
            (
                lhs.0,
                if lhs.0 == SourceSuggestionGroupKind::Peer {
                    lhs.1.as_str()
                } else {
                    ""
                },
                lhs.2,
                &lhs.3 .0,
                lhs.3 .1.as_u32(),
            )
                .cmp(&(
                    rhs.0,
                    if rhs.0 == SourceSuggestionGroupKind::Peer {
                        rhs.1.as_str()
                    } else {
                        ""
                    },
                    rhs.2,
                    &rhs.3 .0,
                    rhs.3 .1.as_u32(),
                ))
        });
        for (group, label, _, origin, visible) in values {
            let Some(Item::Value(value)) = item_at(workspace, &origin) else {
                continue;
            };
            let Some(ty) = declared(&origin, value) else {
                continue;
            };
            roots.push(Root {
                name: visible,
                group,
                label,
                scope: None,
                start: origin.clone(),
                ty,
                doc: self.scope.render_doc(&origin.0, value.doc.as_ref()),
                value: Some((origin.0.clone(), value.value)),
            });
        }
        roots
    }
}

/// What every suggestion that starts from one root shares.
struct Context {
    group: SourceSuggestionGroup,
    scope: Option<u32>,
    declaration: u32,
    self_: Vec<u32>,
    value: Option<(String, ExprId)>,
}
