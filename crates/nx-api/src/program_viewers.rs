//! The viewer declarations a program loads: which modules a program reaches from its entry, the
//! viewer declarations in them with the declaration each targets, and the duplicates two of those
//! modules declare for one target.
//!
//! <para>A viewer declaration applies without being exported or imported: one in any module the
//! program loads applies, and one in a module the program does not load does not. So the set is
//! decided by the program's import graph, not by any one module's bindings.</para>

use crate::artifacts::{workspace_import_target_identity, LibraryArtifact, ProgramBuildContext};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::ast::{Expr, Literal};
use nx_hir::{DeclaringOrigin, ExprId, LoweredModule, ViewerRecord, ViewerTarget};
use nx_types::ModuleArtifact;
use rustc_hash::FxHashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// One module a program loads, with the entry module's import it was first reached through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramModule {
    /// The module's identity.
    pub identity: String,
    /// The span, in the entry module, of the import through which the program first loads this
    /// module; absent for the entry module itself.
    pub via: Option<TextSpan>,
}

/// Every module a program whose entry is the workspace module `entry` loads, entry first, then in
/// the order the imports reach them.
///
/// <para>A workspace module's import reaches the workspace module it names, or every module of
/// the library it names, and a library reaches the libraries it depends on. `imports_of` gives a
/// workspace module's imports, synthesized implicit imports included; it answers `None` for an
/// identity the workspace does not hold, which is then not followed. The prelude declares no
/// viewers and is left out.</para>
pub fn program_modules<'a>(
    entry: &str,
    imports_of: impl Fn(&str) -> Option<&'a [nx_hir::Import]>,
    build_context: &ProgramBuildContext,
) -> Vec<ProgramModule> {
    let mut modules = vec![ProgramModule {
        identity: entry.to_string(),
        via: None,
    }];
    let mut seen_modules = FxHashSet::from_iter([entry.to_string()]);
    let mut seen_libraries = FxHashSet::<PathBuf>::default();
    let mut pending = vec![(entry.to_string(), None::<TextSpan>)];
    let mut pending_libraries = Vec::<(Arc<LibraryArtifact>, Option<TextSpan>)>::new();

    while !pending.is_empty() || !pending_libraries.is_empty() {
        while let Some((identity, via)) = pending.pop() {
            let Some(imports) = imports_of(&identity) else {
                continue;
            };
            for import in imports {
                // Only the entry's own imports are where a program-level diagnostic can point.
                let via = via.or_else(|| (identity == entry).then_some(import.span));
                let Ok(target) = workspace_import_target_identity(&identity, &import.library_path)
                else {
                    continue;
                };
                if imports_of(&target).is_some() {
                    if seen_modules.insert(target.clone()) {
                        modules.push(ProgramModule {
                            identity: target.clone(),
                            via,
                        });
                        pending.push((target, via));
                    }
                } else if let Some(library) = build_context.library_for_import_target(&target) {
                    pending_libraries.push((library, via));
                }
            }
        }
        while let Some((library, via)) = pending_libraries.pop() {
            if !seen_libraries.insert(library.root_path.clone()) {
                continue;
            }
            for module in &library.modules {
                if seen_modules.insert(module.file_name.clone()) {
                    modules.push(ProgramModule {
                        identity: module.file_name.clone(),
                        via,
                    });
                }
            }
            for root in &library.dependency_roots {
                if let Some(dependency) = build_context.library_for_root(root) {
                    pending_libraries.push((dependency, via));
                }
            }
        }
    }
    modules
}

/// One viewer declaration of a loaded module, with the declaration its target names.
#[derive(Debug, Clone)]
pub struct ProgramViewer {
    /// The identity of the module that holds the viewer declaration.
    pub module: String,
    /// The viewer declaration's index among its module's viewer declarations.
    pub index: usize,
    /// The target as written.
    pub target_name: String,
    /// What the target resolved to.
    pub target: ViewerTarget,
    /// The whole viewer declaration.
    pub span: TextSpan,
    /// The span, in the entry module, of the import through which the program loads the module.
    pub via: Option<TextSpan>,
    /// The value, read without evaluating it.
    pub value: ViewerValue,
}

impl ProgramViewer {
    /// Whether the module holding the viewer declaration also declares its target.
    pub fn is_in_declaring_module(&self) -> bool {
        self.target.origin.module_identity() == self.module
    }
}

/// The viewer declarations of `modules`, in module order and then in source order, each with its
/// target resolved in its own module.
///
/// <para>A viewer declaration whose target does not resolve to a declaration that can have a
/// viewer, or whose value is not the viewer record for that target, is left out: its module's
/// analysis reports it. So is the second of two viewer declarations of one target in one module,
/// which analysis reports as `duplicate-viewer`.</para>
pub fn program_viewers<'a>(
    modules: &[ProgramModule],
    artifact_of: impl Fn(&str) -> Option<&'a ModuleArtifact>,
) -> Vec<ProgramViewer> {
    let mut viewers = Vec::new();
    for module in modules {
        let Some(artifact) = artifact_of(&module.identity) else {
            continue;
        };
        let (Some(lowered), Some(prepared)) = (
            artifact.lowered_module.as_deref(),
            artifact.prepared_module.as_deref(),
        ) else {
            continue;
        };
        let mut targets = FxHashSet::default();
        for (index, viewer) in lowered.viewers().iter().enumerate() {
            let Ok(target) = nx_hir::resolve_viewer_target(prepared, &viewer.target) else {
                continue;
            };
            let Some(value) = read_viewer_value(lowered, viewer.value) else {
                continue;
            };
            if value.record != target.record || !targets.insert(target.origin.clone()) {
                continue;
            }
            viewers.push(ProgramViewer {
                module: module.identity.clone(),
                index,
                target_name: viewer.target.as_str().to_string(),
                target,
                span: viewer.span,
                via: module.via,
                value,
            });
        }
    }
    viewers
}

/// The targets two or more loaded modules other than the target's declaring module each declare a
/// viewer for, each with those viewer declarations in module order.
///
/// <para>Viewer declarations in the declaring module apply first and never conflict; two in other
/// modules have no order between them, so neither applies.</para>
pub fn duplicate_program_viewers(
    viewers: &[ProgramViewer],
) -> Vec<(DeclaringOrigin, Vec<&ProgramViewer>)> {
    let mut by_target: Vec<(DeclaringOrigin, Vec<&ProgramViewer>)> = Vec::new();
    for viewer in viewers
        .iter()
        .filter(|viewer| !viewer.is_in_declaring_module())
    {
        match by_target
            .iter_mut()
            .find(|(origin, _)| *origin == viewer.target.origin)
        {
            Some((_, group)) => group.push(viewer),
            None => by_target.push((viewer.target.origin.clone(), vec![viewer])),
        }
    }
    by_target.retain(|(_, group)| group.len() > 1);
    by_target
}

/// The `duplicate-viewer` diagnostics of the program whose entry is `entry`: one per target two
/// other modules declare a viewer for, at the entry's own viewer declaration when it holds one of
/// them and otherwise at the entry's import through which the program loads the first.
pub fn duplicate_viewer_diagnostics(entry: &str, viewers: &[ProgramViewer]) -> Vec<Diagnostic> {
    duplicate_program_viewers(viewers)
        .into_iter()
        .filter_map(|(_, group)| {
            let here = group.iter().find(|viewer| viewer.module == entry);
            let (span, others) = match here {
                Some(here) => (
                    here.span,
                    group
                        .iter()
                        .filter(|viewer| viewer.module != entry)
                        .collect::<Vec<_>>(),
                ),
                None => (group[0].via?, group.iter().collect::<Vec<_>>()),
            };
            let modules = group
                .iter()
                .map(|viewer| format!("'{}'", viewer.module))
                .collect::<Vec<_>>()
                .join(" and ");
            let target = &group[0].target;
            let mut builder = Diagnostic::error("duplicate-viewer")
                .with_message(format!(
                    "'{}' has a viewer in {}, and neither declares '{}', so neither applies; keep one, or move it into the module that declares '{}'",
                    group[0].target_name, modules, target.declared_name, target.declared_name
                ))
                .with_label(Label::primary(entry, span));
            for other in others {
                if other.module != entry {
                    builder = builder.with_label(
                        Label::secondary(other.module.as_str(), other.span)
                            .with_message("a viewer declaration of the same target"),
                    );
                }
            }
            Some(builder.build())
        })
        .collect()
}

/// A viewer declaration's value as written: the `@nx/viewer` record, its attributes and its
/// child viewer records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerValue {
    /// Which viewer record the value is.
    pub record: ViewerRecord,
    /// The attributes, in source order.
    pub attributes: Vec<(String, ViewerAttribute)>,
    /// The `PropertyViewer`s of an `ElementViewer`, or the `CaseViewer`s of a `UnionViewer`, in
    /// source order.
    pub children: Vec<ViewerChild>,
}

/// One child viewer record: a `PropertyViewer` or a `CaseViewer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerChild {
    /// The attributes, in source order.
    pub attributes: Vec<(String, ViewerAttribute)>,
}

impl ViewerValue {
    /// The attribute named `name`, if the value sets it.
    pub fn attribute(&self, name: &str) -> Option<&ViewerAttribute> {
        attribute(&self.attributes, name)
    }
}

impl ViewerChild {
    /// The attribute named `name`, if the child sets it.
    pub fn attribute(&self, name: &str) -> Option<&ViewerAttribute> {
        attribute(&self.attributes, name)
    }
}

fn attribute<'a>(
    attributes: &'a [(String, ViewerAttribute)],
    name: &str,
) -> Option<&'a ViewerAttribute> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

/// An attribute's value as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerAttribute {
    /// A string literal.
    Text(String),
    /// A boolean literal.
    Boolean(bool),
    /// A bare name: a property or a case.
    Name(String),
}

impl ViewerAttribute {
    /// The value as text: a string's text, a name, or `true`/`false`.
    pub fn text(&self) -> String {
        match self {
            ViewerAttribute::Text(text) | ViewerAttribute::Name(text) => text.clone(),
            ViewerAttribute::Boolean(value) => value.to_string(),
        }
    }
}

/// Reads a viewer declaration's value: an element of an `@nx/viewer` record, whose attributes are
/// literals and bare names and whose content is child viewer records.
///
/// <para>Answers `None` for a value that is not a viewer record's element. Anything else written
/// in it — which analysis reports — is skipped.</para>
pub fn read_viewer_value(module: &LoweredModule, value: ExprId) -> Option<ViewerValue> {
    let (tag, attributes, content) = element_parts(module, value)?;
    let record = match tag.as_str() {
        "ElementViewer" => ViewerRecord::Element,
        "UnionViewer" => ViewerRecord::Union,
        "ValueViewer" => ViewerRecord::Value,
        _ => return None,
    };
    let children = content
        .into_iter()
        .filter_map(|child| element_parts(module, child))
        .filter(|(tag, _, _)| matches!(tag.as_str(), "PropertyViewer" | "CaseViewer"))
        .map(|(_, attributes, _)| ViewerChild { attributes })
        .collect();
    Some(ViewerValue {
        record,
        attributes,
        children,
    })
}

type ElementParts = (String, Vec<(String, ViewerAttribute)>, Vec<ExprId>);

fn element_parts(module: &LoweredModule, expr: ExprId) -> Option<ElementParts> {
    match module.expr(expr) {
        Expr::Element { element, .. } => {
            let element = module.element(*element);
            let attributes = element
                .properties
                .iter()
                .filter_map(|property| {
                    Some((
                        property.key.as_str().to_string(),
                        attribute_value(module, property.value)?,
                    ))
                })
                .collect();
            Some((
                element.tag.as_str().to_string(),
                attributes,
                element.content.clone(),
            ))
        }
        Expr::RecordLiteral {
            record, properties, ..
        } => Some((
            record.as_str().to_string(),
            properties
                .iter()
                .filter_map(|property| {
                    Some((
                        property.name.as_str().to_string(),
                        attribute_value(module, property.value)?,
                    ))
                })
                .collect(),
            Vec::new(),
        )),
        _ => None,
    }
}

fn attribute_value(module: &LoweredModule, expr: ExprId) -> Option<ViewerAttribute> {
    match module.expr(expr) {
        Expr::Literal(Literal::String(text)) => Some(ViewerAttribute::Text(text.to_string())),
        Expr::Literal(Literal::Boolean(value)) => Some(ViewerAttribute::Boolean(*value)),
        Expr::Ident(name) => Some(ViewerAttribute::Name(name.as_str().to_string())),
        Expr::ResolvedUnionCase { case, .. } => {
            Some(ViewerAttribute::Name(case.as_str().to_string()))
        }
        _ => None,
    }
}
