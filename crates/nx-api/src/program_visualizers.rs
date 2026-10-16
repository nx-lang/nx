//! The visualizer declarations a program loads: which modules a program reaches from its entry, the
//! visualizer declarations in them with the declaration each targets, and the duplicates two of
//! those modules declare for one target.
//!
//! <para>A visualizer declaration applies without being exported or imported: one in any module the
//! program loads applies, and one in a module the program does not load does not. So the set is
//! decided by the program's import graph, not by any one module's bindings.</para>

use crate::artifacts::{workspace_import_target_identity, LibraryArtifact, ProgramBuildContext};
use nx_diagnostics::{Diagnostic, Label, Severity, TextSpan};
use nx_hir::ast::{Expr, Literal};
use nx_hir::{DeclaringOrigin, ExprId, LoweredModule, VisualizerRecord, VisualizerTarget};
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
/// visualizers and is left out.</para>
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

/// One visualizer declaration of a loaded module, with the declaration its target names.
#[derive(Debug, Clone)]
pub struct ProgramVisualizer {
    /// The identity of the module that holds the visualizer declaration.
    pub module: String,
    /// The visualizer declaration's index among its module's visualizer declarations.
    pub index: usize,
    /// The target as written.
    pub target_name: String,
    /// What the target resolved to.
    pub target: VisualizerTarget,
    /// The whole visualizer declaration.
    pub span: TextSpan,
    /// The span, in the entry module, of the import through which the program loads the module.
    pub via: Option<TextSpan>,
    /// The value, read without evaluating it.
    pub value: VisualizerValue,
    /// Whether analysis reported no error inside the declaration. One that is not clean still
    /// counts as one of two duplicates, but never applies.
    pub clean: bool,
}

impl ProgramVisualizer {
    /// Whether the module holding the visualizer declaration also declares its target.
    pub fn is_in_declaring_module(&self) -> bool {
        self.target.origin.module_identity() == self.module
    }
}

/// The visualizer declarations of `modules`, in module order and then in source order, each with
/// its target resolved in its own module.
///
/// <para>A visualizer declaration whose target does not resolve to a declaration that can have a
/// visualizer, or whose value is not the visualizer record for that target, is left out: its
/// module's analysis reports it. So is the second of two visualizer declarations of one target in
/// one module, which analysis reports as `duplicate-visualizer` whether or not the first has
/// errors. A declaration with any other error is kept but not `clean`, so it still counts as a
/// duplicate across modules and never applies.</para>
pub fn program_visualizers<'a>(
    modules: &[ProgramModule],
    artifact_of: impl Fn(&str) -> Option<&'a ModuleArtifact>,
) -> Vec<ProgramVisualizer> {
    let mut visualizers = Vec::new();
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
        for (index, visualizer) in lowered.visualizers().iter().enumerate() {
            let Ok(target) = nx_hir::resolve_visualizer_target(prepared, &visualizer.target) else {
                continue;
            };
            // The first visualizer declaration of a target claims it, as analysis decides, even
            // when it has errors.
            if !targets.insert(target.origin.clone()) {
                continue;
            }
            let Some(value) = read_visualizer_value(lowered, visualizer.value) else {
                continue;
            };
            if value.record != target.record {
                continue;
            }
            visualizers.push(ProgramVisualizer {
                module: module.identity.clone(),
                index,
                target_name: visualizer.target.as_str().to_string(),
                target,
                span: visualizer.span,
                via: module.via,
                value,
                clean: !has_errors_within(artifact, &module.identity, visualizer.span),
            });
        }
    }
    visualizers
}

/// Whether analysis reported an error inside `span` of the module, other than
/// `duplicate-visualizer`, which says which of two clean visualizer declarations applies rather
/// than that one is wrong.
fn has_errors_within(artifact: &ModuleArtifact, identity: &str, span: TextSpan) -> bool {
    artifact.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity() == Severity::Error
            && diagnostic.code() != Some("duplicate-visualizer")
            && diagnostic.labels().iter().any(|label| {
                label.primary
                    && (label.file == artifact.file_name || label.file == identity)
                    && span.contains_range(label.range)
            })
    })
}

/// The targets two or more loaded modules other than the target's declaring module each declare a
/// visualizer for, each with those visualizer declarations in module order.
///
/// <para>Visualizer declarations in the declaring module apply first and never conflict; two in
/// other modules have no order between them, so neither applies.</para>
pub fn duplicate_program_visualizers(
    visualizers: &[ProgramVisualizer],
) -> Vec<(DeclaringOrigin, Vec<&ProgramVisualizer>)> {
    let mut by_target: Vec<(DeclaringOrigin, Vec<&ProgramVisualizer>)> = Vec::new();
    for visualizer in visualizers
        .iter()
        .filter(|visualizer| !visualizer.is_in_declaring_module())
    {
        match by_target
            .iter_mut()
            .find(|(origin, _)| *origin == visualizer.target.origin)
        {
            Some((_, group)) => group.push(visualizer),
            None => by_target.push((visualizer.target.origin.clone(), vec![visualizer])),
        }
    }
    by_target.retain(|(_, group)| group.len() > 1);
    by_target
}

/// The `duplicate-visualizer` diagnostics of the program whose entry is `entry`: one per target two
/// other modules declare a visualizer for, at the entry's own visualizer declaration when it holds
/// one of them and otherwise at the entry's import through which the program loads the first.
pub fn duplicate_visualizer_diagnostics(
    entry: &str,
    visualizers: &[ProgramVisualizer],
) -> Vec<Diagnostic> {
    duplicate_program_visualizers(visualizers)
        .into_iter()
        .filter_map(|(_, group)| {
            let here = group.iter().find(|visualizer| visualizer.module == entry);
            let (span, others) = match here {
                Some(here) => (
                    here.span,
                    group
                        .iter()
                        .filter(|visualizer| visualizer.module != entry)
                        .collect::<Vec<_>>(),
                ),
                None => (group[0].via?, group.iter().collect::<Vec<_>>()),
            };
            let modules = group
                .iter()
                .map(|visualizer| format!("'{}'", visualizer.module))
                .collect::<Vec<_>>()
                .join(" and ");
            let target = &group[0].target;
            let mut builder = Diagnostic::error("duplicate-visualizer")
                .with_message(format!(
                    "'{}' has a visualizer in {}, and neither declares '{}', so neither applies; keep one, or move it into the module that declares '{}'",
                    group[0].target_name, modules, target.declared_name, target.declared_name
                ))
                .with_label(Label::primary(entry, span));
            for other in others {
                if other.module != entry {
                    builder = builder.with_label(
                        Label::secondary(other.module.as_str(), other.span)
                            .with_message("a visualizer declaration of the same target"),
                    );
                }
            }
            Some(builder.build())
        })
        .collect()
}

/// A visualizer declaration's value as written: the `@nx/visualizer` record, its attributes and its
/// child visualizer records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizerValue {
    /// Which visualizer record the value is.
    pub record: VisualizerRecord,
    /// The attributes, in source order.
    pub attributes: Vec<(String, VisualizerAttribute)>,
    /// The `PropertyVisualizer`s of an `ElementVisualizer`, or the `CaseVisualizer`s of a
    /// `UnionVisualizer`, in source order.
    pub children: Vec<VisualizerChild>,
}

/// One child visualizer record: a `PropertyVisualizer` or a `CaseVisualizer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizerChild {
    /// The attributes, in source order.
    pub attributes: Vec<(String, VisualizerAttribute)>,
}

impl VisualizerValue {
    /// The attribute named `name`, if the value sets it.
    pub fn attribute(&self, name: &str) -> Option<&VisualizerAttribute> {
        attribute(&self.attributes, name)
    }
}

impl VisualizerChild {
    /// The attribute named `name`, if the child sets it.
    pub fn attribute(&self, name: &str) -> Option<&VisualizerAttribute> {
        attribute(&self.attributes, name)
    }
}

fn attribute<'a>(
    attributes: &'a [(String, VisualizerAttribute)],
    name: &str,
) -> Option<&'a VisualizerAttribute> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

/// An attribute's value as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualizerAttribute {
    /// A string literal.
    Text(String),
    /// A boolean literal.
    Boolean(bool),
    /// A bare name: a property or a case.
    Name(String),
}

impl VisualizerAttribute {
    /// The value as text: a string's text, a name, or `true`/`false`.
    pub fn text(&self) -> String {
        match self {
            VisualizerAttribute::Text(text) | VisualizerAttribute::Name(text) => text.clone(),
            VisualizerAttribute::Boolean(value) => value.to_string(),
        }
    }
}

/// Reads a visualizer declaration's value: an element of an `@nx/visualizer` record, whose
/// attributes are literals and bare names and whose content is child visualizer records.
///
/// <para>Answers `None` for a value that is not a visualizer record's element. Anything else
/// written in it — which analysis reports — is skipped.</para>
pub fn read_visualizer_value(module: &LoweredModule, value: ExprId) -> Option<VisualizerValue> {
    let (tag, attributes, content) = element_parts(module, value)?;
    let record = match tag.as_str() {
        "ElementVisualizer" => VisualizerRecord::Element,
        "ValueVisualizer" => VisualizerRecord::Value,
        _ => return None,
    };
    let children = content
        .into_iter()
        .filter_map(|child| element_parts(module, child))
        .filter(|(tag, _, _)| tag == "PropertyVisualizer")
        .map(|(_, attributes, _)| VisualizerChild { attributes })
        .collect();
    Some(VisualizerValue {
        record,
        attributes,
        children,
    })
}

type ElementParts = (String, Vec<(String, VisualizerAttribute)>, Vec<ExprId>);

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

fn attribute_value(module: &LoweredModule, expr: ExprId) -> Option<VisualizerAttribute> {
    match module.expr(expr) {
        Expr::Literal(Literal::String(text)) => Some(VisualizerAttribute::Text(text.to_string())),
        Expr::Literal(Literal::Boolean(value)) => Some(VisualizerAttribute::Boolean(*value)),
        Expr::Ident(name) => Some(VisualizerAttribute::Name(name.as_str().to_string())),
        Expr::ResolvedUnionCase { case, .. } => {
            Some(VisualizerAttribute::Name(case.as_str().to_string()))
        }
        _ => None,
    }
}
