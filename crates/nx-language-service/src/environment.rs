//! Editor support for environment values: completion after `env.`, hover on reads and overrides,
//! the values a declaration reads, and what a top-level `provide` sets.
//!
//! <para>An environment value is lowered as a value named `env.name`, so the declaration index
//! already holds it under that name, or under `alias.env.name` through an aliased import. What
//! is here is the part only environment values need: finding the value an `env.` spelling names,
//! and walking the expressions a declaration evaluates for the values it reads.</para>

use crate::{
    hover, CompletionItem, CompletionItemKind, Declaration, DeclarationOrigin, DocumentScope,
    ResolvedPosition, WorkspaceSnapshot,
};
use nx_hir::{ast::EnvOverride, environment_binding_name, environment_value_name, Item, ValueDef};
use rustc_hash::FxHashSet;
use text_size::{TextRange as ByteTextRange, TextSize};

/// The longest default a hover spells out; a longer one is cut with `…`.
const DEFAULT_DISPLAY_LIMIT: usize = 60;

impl WorkspaceSnapshot {
    /// The hover at an `env.` read, or at the name of an override, and the name's range: the
    /// declaration of the value it names. Anywhere else, nothing.
    pub(crate) fn environment_hover(
        &self,
        resolved: &ResolvedPosition<'_>,
    ) -> Option<(String, ByteTextRange)> {
        let scope = &resolved.scope;
        let lowered = scope
            .workspace
            .artifact(resolved.document.identity.as_str())?
            .lowered_module
            .clone()?;
        let source = resolved.document.source();
        let offset = TextSize::from(resolved.offset as u32);

        // A read covers `env.name`; a member access on it is the field's, and is answered there.
        let read = lowered
            .exprs()
            .filter_map(|(_, expr)| match expr {
                nx_hir::ast::Expr::EnvRead {
                    binding: Some(binding),
                    span,
                    ..
                } if span.contains_inclusive(offset) => Some((binding.clone(), *span)),
                _ => None,
            })
            .min_by_key(|(_, span)| span.len());
        if let Some((binding, span)) = read {
            let declaration = scope.visible.get(binding.as_str())?;
            return Some((
                self.environment_declaration_hover(scope, declaration)?,
                span,
            ));
        }

        for provided in module_overrides(&lowered) {
            let Some(name_span) = override_name_span(source, provided) else {
                continue;
            };
            if !name_span.contains_inclusive(offset) {
                continue;
            }
            let declaration = scope.visible.get(provided.binding.as_ref()?.as_str())?;
            return Some((
                self.environment_declaration_hover(scope, declaration)?,
                name_span,
            ));
        }
        None
    }

    /// The hover of a top-level `provide` the position is in, and its range, given what the
    /// position answers on its own: the values and field paths the line sets, after that answer
    /// when there is one. A bundle is followed to its `<env.Update>`, so the line can be read
    /// without opening the bundle.
    pub(crate) fn provide_hover(
        &self,
        resolved: &ResolvedPosition<'_>,
        inner: Option<(String, ByteTextRange)>,
    ) -> Option<(String, ByteTextRange)> {
        let scope = &resolved.scope;
        let identity = resolved.document.identity.as_str();
        let lowered = scope.workspace.artifact(identity)?.lowered_module.clone()?;
        let offset = TextSize::from(resolved.offset as u32);
        let Some(provide) = lowered
            .provides()
            .iter()
            .find(|provide| provide.span().contains_inclusive(offset))
        else {
            return block_bundle_hover(scope, identity, &lowered, resolved, inner);
        };
        let set = match provide {
            nx_hir::ProvideDecl::Override(provided) => {
                // The override's own name and value already answer for themselves.
                if inner.is_some() {
                    return inner;
                }
                vec![override_path(provided)]
            }
            nx_hir::ProvideDecl::Bundle { value, .. } => {
                let mut set = Vec::new();
                bundle_paths(scope, identity, *value, &mut set, &mut FxHashSet::default());
                set
            }
        };
        let note = sets_note(&set);
        Some(match inner {
            Some((contents, span)) => (format!("{contents}\n\n{note}"), span),
            None => (note, provide.span()),
        })
    }

    /// An environment value's declaration as its author wrote it, with its documentation.
    pub(crate) fn environment_declaration_hover(
        &self,
        scope: &DocumentScope,
        declaration: &Declaration,
    ) -> Option<String> {
        let Item::Value(value) = scope.item(declaration)? else {
            return None;
        };
        let signature = environment_signature(scope, &declaration.origin.0, value)?;
        let content = hover::fenced(hover::with_origin_label(&declaration.origin.0, signature));
        Some(hover::with_doc(content, declaration.doc.as_deref()))
    }

    /// The completions after `env.` or `env.name.` written up to the cursor, in an expression,
    /// an element's attributes or a top-level `provide`; `None` where the cursor follows neither.
    pub(crate) fn environment_completions(
        &self,
        resolved: &ResolvedPosition<'_>,
    ) -> Option<Vec<CompletionItem>> {
        let before = &resolved.document.source()[..resolved.offset];
        let start = before
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .map(|index| index + 1)
            .unwrap_or(0);
        let written = &before[start..];
        let (qualifier, _partial) = written.rsplit_once('.')?;
        let scope = &resolved.scope;

        // `env.` and `design.env.`: the environment values seen under that spelling.
        if qualifier == "env" || qualifier.ends_with(".env") {
            let prefix = format!("{qualifier}.");
            let mut items = scope
                .visible_declarations()
                .into_iter()
                .filter_map(|declaration| {
                    let name = declaration.name.strip_prefix(&prefix)?;
                    if name.contains('.') {
                        return None;
                    }
                    let Some(Item::Value(value)) = scope.item(declaration) else {
                        return None;
                    };
                    environment_value_name(value.name.as_str())?;
                    Some(CompletionItem {
                        label: name.to_string(),
                        kind: CompletionItemKind::Property,
                        detail: value.ty.as_ref().map(crate::type_ref_display),
                        documentation: declaration.doc.clone(),
                    })
                })
                .collect::<Vec<_>>();
            items.sort_by(|lhs, rhs| lhs.label.cmp(&rhs.label));
            return Some(items);
        }

        // `env.name.` and deeper: the fields of the record type at that path.
        let (root, fields) = match qualifier.find("env.") {
            Some(index) if index == 0 || qualifier[..index].ends_with('.') => {
                let rest = &qualifier[index + "env.".len()..];
                let mut parts = rest.split('.');
                let name = parts.next()?;
                (
                    format!("{}env.{name}", &qualifier[..index]),
                    parts.collect::<Vec<_>>(),
                )
            }
            _ => return None,
        };
        let declaration = scope.visible.get(&root)?;
        environment_binding_name(&declaration.name)?;
        let Item::Value(value) = scope.item(declaration)? else {
            return None;
        };
        let mut module = declaration.origin.0.clone();
        let mut record = scope.type_declared_in(&module, &base_type_name(value.ty.as_ref()?))?;
        for field in fields {
            let property = record
                .properties
                .iter()
                .find(|property| property.name == field)?;
            module = property.declaring_module.clone();
            record = scope.type_declared_in(&module, &property.base_type)?;
        }
        Some(
            record
                .properties
                .iter()
                .map(|property| CompletionItem {
                    label: property.name.clone(),
                    kind: CompletionItemKind::Property,
                    detail: Some(property.display_type.clone()),
                    documentation: property.doc.clone(),
                })
                .collect(),
        )
    }
}

/// `environment name: T = default`, with a long or multi-line default cut short.
fn environment_signature(scope: &DocumentScope, module: &str, value: &ValueDef) -> Option<String> {
    let name = environment_value_name(value.name.as_str())?;
    let ty = value
        .ty
        .as_ref()
        .map(crate::type_ref_display)
        .unwrap_or_default();
    let lowered = scope.workspace.artifact(module)?.lowered_module.clone()?;
    let default = scope.workspace.sources.get(module).and_then(|source| {
        let span = lowered.expr_span(value.value);
        source.get(usize::from(span.start())..usize::from(span.end()))
    });
    let mut signature = format!("environment {name}: {ty}");
    if let Some(default) = default.map(str::trim).filter(|text| !text.is_empty()) {
        let line = default.lines().next().unwrap_or_default();
        let cut = line.chars().count() > DEFAULT_DISPLAY_LIMIT || line.len() < default.len();
        let shown = line.chars().take(DEFAULT_DISPLAY_LIMIT).collect::<String>();
        signature.push_str(&format!(" = {shown}{}", if cut { " …" } else { "" }));
    }
    Some(signature)
}

impl DocumentScope {
    /// The HIR item declared at `origin`, in whichever module declares it.
    fn item_at_origin(&self, origin: &DeclarationOrigin) -> Option<&Item> {
        self.workspace
            .artifact(&origin.0)?
            .lowered_module
            .as_deref()?
            .item_by_definition(origin.1)
    }
}

/// The name a namespace lookup takes for a written type: without its occurrence suffix.
fn base_type_name(ty: &nx_hir::ast::TypeRef) -> String {
    crate::type_ref_display(ty)
        .trim_end_matches(['?', '+', '*'])
        .to_string()
}

/// Every override written in a module: on elements, in `provide` blocks, in bundles and in
/// top-level provides.
fn module_overrides(lowered: &nx_hir::LoweredModule) -> Vec<&EnvOverride> {
    let mut overrides = lowered
        .exprs()
        .flat_map(|(_, expr)| match expr {
            nx_hir::ast::Expr::Provide { overrides, .. }
            | nx_hir::ast::Expr::EnvUpdate { overrides, .. } => overrides.iter().collect(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    overrides.extend(
        lowered
            .provides()
            .iter()
            .filter_map(|provide| match provide {
                nx_hir::ProvideDecl::Override(provided) => Some(provided),
                nx_hir::ProvideDecl::Bundle { .. } => None,
            }),
    );
    overrides
}

/// Where an override's name is written: `env.space.width` in `env.space.width=3`, or
/// `space.width` in a bundle attribute.
fn override_name_span(source: &str, provided: &EnvOverride) -> Option<ByteTextRange> {
    let written =
        source.get(usize::from(provided.span.start())..usize::from(provided.span.end()))?;
    let path = std::iter::once(provided.name.as_str())
        .chain(provided.path.iter().map(|part| part.as_str()))
        .collect::<Vec<_>>()
        .join(".");
    let (at, len) = match written.find(&format!("env.{path}")) {
        Some(at) => (at, "env.".len() + path.len()),
        None => (written.find(&path)?, path.len()),
    };
    let start = provided.span.start() + TextSize::from(at as u32);
    Some(ByteTextRange::at(start, TextSize::from(len as u32)))
}

/// `env.name.field` for an override.
fn override_path(provided: &EnvOverride) -> String {
    std::iter::once(format!("env.{}", provided.name))
        .chain(provided.path.iter().map(|part| part.to_string()))
        .collect::<Vec<_>>()
        .join(".")
}

/// The paths the bundle expression `value`, written in `module`, sets. A name is followed to the
/// value it names, and a `*.Update` record set as an entry's value contributes its fields.
fn bundle_paths(
    scope: &DocumentScope,
    module: &str,
    value: nx_hir::ExprId,
    set: &mut Vec<String>,
    visited: &mut FxHashSet<DeclarationOrigin>,
) {
    let Some(lowered) = scope
        .workspace
        .artifact(module)
        .and_then(|artifact| artifact.lowered_module.clone())
    else {
        return;
    };
    match lowered.expr(value) {
        // `bundle with step`: the bundle's entries, then the step's.
        nx_hir::ast::Expr::With { left, right, .. } => {
            bundle_paths(scope, module, *left, set, visited);
            match right {
                nx_hir::ast::WithRight::Fields(fields) => {
                    for field in fields {
                        let path = std::iter::once(format!("env.{}", field.path[0]))
                            .chain(field.path[1..].iter().map(|part| part.to_string()))
                            .collect::<Vec<_>>()
                            .join(".");
                        update_paths(&lowered, field.value, &path, set);
                    }
                }
                nx_hir::ast::WithRight::Value(step) => {
                    bundle_paths(scope, module, *step, set, visited)
                }
            }
        }
        nx_hir::ast::Expr::EnvUpdate {
            bases, overrides, ..
        } => {
            for base in bases {
                bundle_paths(scope, module, *base, set, visited);
            }
            for provided in overrides {
                let path = override_path(provided);
                update_paths(&lowered, provided.value, &path, set);
            }
        }
        expr => {
            let Some(name) = qualified_name(&lowered, expr) else {
                return;
            };
            let Some(origin) = binding_origin(scope, module, &name) else {
                return;
            };
            if !visited.insert(origin.clone()) {
                return;
            }
            let Some(Item::Value(bundle)) = scope.item_at_origin(&origin) else {
                return;
            };
            bundle_paths(scope, &origin.0, bundle.value, set, visited);
        }
    }
}

/// `path`, or the paths of the fields set when `value` is a `*.Update` record.
fn update_paths(
    lowered: &nx_hir::LoweredModule,
    value: nx_hir::ExprId,
    path: &str,
    set: &mut Vec<String>,
) {
    let fields = match lowered.expr(value) {
        nx_hir::ast::Expr::Element { element, .. } => {
            let element = lowered.element(*element);
            element.tag.as_str().ends_with(".Update").then(|| {
                element
                    .properties
                    .iter()
                    .map(|property| (property.key.to_string(), property.value))
                    .collect::<Vec<_>>()
            })
        }
        nx_hir::ast::Expr::RecordLiteral {
            record, properties, ..
        } => record.as_str().ends_with(".Update").then(|| {
            properties
                .iter()
                .map(|property| (property.name.to_string(), property.value))
                .collect::<Vec<_>>()
        }),
        _ => None,
    };
    match fields {
        Some(fields) if !fields.is_empty() => {
            for (field, value) in fields {
                update_paths(lowered, value, &format!("{path}.{field}"), set);
            }
        }
        _ => set.push(path.to_string()),
    }
}

/// `Sets `env.a`, `env.b.c`.`
fn sets_note(set: &[String]) -> String {
    if set.is_empty() {
        return "Sets no environment values it can name.".to_string();
    }
    format!(
        "Sets {}.",
        set.iter()
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// `name` or `alias.name` when `expr` is a name or a member access on one.
fn qualified_name(lowered: &nx_hir::LoweredModule, expr: &nx_hir::ast::Expr) -> Option<String> {
    match expr {
        nx_hir::ast::Expr::Ident(name) => Some(name.to_string()),
        nx_hir::ast::Expr::Member { base, member, .. } => Some(format!(
            "{}.{member}",
            qualified_name(lowered, lowered.expr(*base))?
        )),
        _ => None,
    }
}

/// The declaration `name` binds in the module with identity `module`.
fn binding_origin(scope: &DocumentScope, module: &str, name: &str) -> Option<DeclarationOrigin> {
    scope
        .workspace
        .visible_bindings
        .get(module)?
        .iter()
        .filter(|(visible, _)| visible == name)
        .map(|(_, origin)| origin)
        .find(|origin| {
            scope
                .item_at_origin(origin)
                .is_some_and(|item| matches!(item, Item::Function(_) | Item::Value(_)))
        })
        .cloned()
}

/// The environment values the declaration at `origin` reads, directly or through the functions
/// and values it names, by their names after `env.`.
///
/// <para>A component a body writes as an element is not followed: its own reads happen when it
/// renders, in the environment its place gives it. A value's default is not followed either,
/// since a read of the value is what is listed.</para>
pub(crate) fn environment_reads(scope: &DocumentScope, origin: &DeclarationOrigin) -> Vec<String> {
    let mut reads = Vec::new();
    let mut seen_reads = FxHashSet::default();
    let mut visited = FxHashSet::default();
    let mut pending = vec![origin.clone()];
    while let Some(current) = pending.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
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
            match expr {
                nx_hir::ast::Expr::EnvRead {
                    binding: Some(binding),
                    ..
                } => {
                    let Some(target) = binding_origin(scope, &current.0, binding.as_str()) else {
                        continue;
                    };
                    let Some(Item::Value(value)) = scope.item_at_origin(&target) else {
                        continue;
                    };
                    if let Some(name) = environment_value_name(value.name.as_str()) {
                        if seen_reads.insert(target) {
                            reads.push(name.to_string());
                        }
                    }
                }
                nx_hir::ast::Expr::Ident(name) if !locals.contains(&id) => {
                    if let Some(target) = binding_origin(scope, &current.0, name.as_str()) {
                        pending.push(target);
                    }
                }
                nx_hir::ast::Expr::Member { .. } => {
                    if let Some(target) = qualified_name(&lowered, expr)
                        .and_then(|name| binding_origin(scope, &current.0, &name))
                    {
                        pending.push(target);
                    }
                }
                _ => {}
            }
        }
    }
    reads.sort();
    reads
}

/// `Reads `env.a`, `env.b`.`, or nothing for a declaration that reads none.
pub(crate) fn reads_note(scope: &DocumentScope, declaration: &Declaration) -> Option<String> {
    match scope.item(declaration)? {
        Item::Function(_) | Item::Component(_) => {}
        Item::Value(value) if environment_value_name(value.name.as_str()).is_none() => {}
        _ => return None,
    }
    let reads = environment_reads(scope, &declaration.origin);
    if reads.is_empty() {
        return None;
    }
    Some(format!(
        "Reads {}.",
        reads
            .iter()
            .map(|name| format!("`env.{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// The hover on the `env with` of a `provide` block the position is on, and its range: what the
/// bundle sets, after what the position answers on its own. Anywhere else, `inner` unchanged.
fn block_bundle_hover(
    scope: &DocumentScope,
    identity: &str,
    lowered: &nx_hir::LoweredModule,
    resolved: &ResolvedPosition<'_>,
    inner: Option<(String, ByteTextRange)>,
) -> Option<(String, ByteTextRange)> {
    let source = resolved.document.source();
    let offset = resolved.offset;
    let (bundle, keyword) = lowered.exprs().find_map(|(_, expr)| {
        let nx_hir::ast::Expr::Provide {
            bundle: Some(bundle),
            span,
            ..
        } = expr
        else {
            return None;
        };
        // `env with` is written between `provide` and the bundle.
        let start = usize::from(span.start());
        let end = usize::from(lowered.expr_span(*bundle).start());
        let written = source.get(start..end)?;
        let at = start + written.find("env")?;
        let keyword_end = start + written.rfind("with")? + "with".len();
        (at <= offset && offset <= keyword_end).then(|| {
            (
                *bundle,
                ByteTextRange::new(
                    TextSize::from(at as u32),
                    TextSize::from(keyword_end as u32),
                ),
            )
        })
    })?;
    let mut set = Vec::new();
    bundle_paths(scope, identity, bundle, &mut set, &mut FxHashSet::default());
    let note = sets_note(&set);
    Some(match inner {
        Some((contents, span)) => (format!("{contents}\n\n{note}"), span),
        None => (note, keyword),
    })
}
