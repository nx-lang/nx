//! Resolves the `[Name]` and `[Type.member]` links in documentation to what they name.
//!
//! A link's first identifier is looked up among the members of the declaration its documentation
//! belongs to or belongs within, innermost first, and then as a name visible at the top level of
//! the module, imports and aliases included. Each further identifier names a member of what the
//! previous part reached. A link that reaches nothing is a warning; it never blocks compilation.

use nx_diagnostics::{Diagnostic, Label, TextSize, TextSpan};
use nx_hir::{
    Component, Doc, DocLink, Item, Name, PreparedModule, PreparedNamespace, RecordDef, RecordKind,
    ResolvedPreparedItem, UnionCaseDef, UnionDef,
};

/// A doc link and what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDocLink {
    /// The source range of `[label]`.
    pub span: TextSpan,
    /// What the link names.
    pub target: DocLinkTarget,
}

/// A top-level declaration, by the name it is visible under in the linking module, or one of its
/// members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLinkTarget {
    /// The declaration's name as visible at the top level of the linking module.
    pub declaration: Name,
    /// The member of the declaration the link names, if it names one.
    pub member: Option<DocLinkMember>,
}

/// A member of a declaration named by a doc link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocLinkMember {
    /// A parameter of a function or an element-style function.
    Parameter(Name),
    /// A field of a record or action.
    RecordField(Name),
    /// A property of a component.
    Property(Name),
    /// A `state` field of a component.
    State(Name),
    /// An `emits` entry of a component.
    Emit(Name),
    /// A case of a union.
    UnionCase(Name),
    /// A field of one union case's payload.
    UnionPayloadField { case: Name, field: Name },
}

/// Resolves every doc link in the module's own declarations.
///
/// Returns the links that resolve, and an `unresolved-doc-link` warning for each that does not.
pub fn resolve_doc_links(
    module: &PreparedModule,
    file_name: &str,
) -> (Vec<ResolvedDocLink>, Vec<Diagnostic>) {
    let mut resolver = Resolver {
        module,
        file_name,
        resolved: Vec::new(),
        diagnostics: Vec::new(),
    };
    for_each_doc(module.raw_module().items(), |doc, scopes| {
        resolver.doc(doc, scopes)
    });
    (resolver.resolved, resolver.diagnostics)
}

/// A name a doc link being written could go on with, and the documentation of what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLinkCandidate {
    /// The declaration the member belongs to, by the name this module sees it under.
    pub declaration: Name,
    /// The member the name would name.
    pub member: DocLinkMember,
    /// The member's documentation, if it has any.
    pub doc: Option<Doc>,
}

impl DocLinkCandidate {
    /// The name to write.
    pub fn name(&self) -> &Name {
        match &self.member {
            DocLinkMember::Parameter(name)
            | DocLinkMember::RecordField(name)
            | DocLinkMember::Property(name)
            | DocLinkMember::State(name)
            | DocLinkMember::Emit(name)
            | DocLinkMember::UnionCase(name)
            | DocLinkMember::UnionPayloadField { field: name, .. } => name,
        }
    }
}

/// The members a doc link written at `offset` could name next.
///
/// <para>With no `qualifier`, those are the members of the declarations the documentation at
/// `offset` belongs to or belongs within, innermost first: the names a link's first identifier is
/// looked up among before the module's top-level names, which the caller adds. With a qualifier,
/// `Type` or `Type.case`, they are the members of what the qualifier names, found the way a link
/// would find it. A qualifier that names nothing with members has no candidates.</para>
pub fn doc_link_candidates(
    module: &PreparedModule,
    offset: TextSize,
    qualifier: &[Name],
) -> Vec<DocLinkCandidate> {
    let mut owners = Vec::new();
    for_each_doc(module.raw_module().items(), |doc, scopes| {
        if owners.is_empty() && doc.is_some_and(|doc| doc_contains(doc, offset)) {
            owners = scopes.to_vec();
        }
    });
    let resolver = Resolver {
        module,
        file_name: "",
        resolved: Vec::new(),
        diagnostics: Vec::new(),
    };

    let mut candidates = Vec::new();
    if qualifier.is_empty() {
        for scope in &owners {
            for candidate in resolver.scope_candidates(*scope) {
                if !candidates
                    .iter()
                    .any(|known: &DocLinkCandidate| known.name() == candidate.name())
                {
                    candidates.push(candidate);
                }
            }
        }
    } else if let Some(target) = resolver.path(qualifier, &owners) {
        candidates = resolver.target_candidates(&target);
    }
    candidates
}

/// Whether `offset` is on one of `doc`'s lines, its end included.
fn doc_contains(doc: &Doc, offset: TextSize) -> bool {
    doc.text
        .split('\n')
        .zip(doc.line_starts.iter())
        .any(|(line, start)| {
            *start <= offset && offset <= *start + TextSize::from(line.len() as u32)
        })
}

/// Calls `visit` with every doc the author wrote in `items`, and the declarations, innermost first,
/// whose members a link in it is looked up among.
fn for_each_doc<'a>(items: &'a [Item], mut visit: impl FnMut(Option<&'a Doc>, &[Scope<'a>])) {
    // An inline `emits` entry is lowered to an action record that repeats the entry's doc, which
    // the component itself carries; only the record's fields are the record's own, and they are
    // written inside the component, so its members are in scope for them too.
    let inline_emit_owners: Vec<(&Name, &Component)> = items
        .iter()
        .filter_map(|item| match item {
            Item::Component(component) => Some(component),
            _ => None,
        })
        .flat_map(|component| {
            component
                .emits
                .iter()
                .filter(|emit| emit.kind == nx_hir::ComponentEmitKind::Inline)
                .map(move |emit| (&emit.action_name, component))
        })
        .collect();

    for item in items {
        match item {
            Item::Function(function) => {
                let scopes = [Scope::Function(function)];
                visit(function.doc.as_ref(), &scopes);
                for param in &function.params {
                    visit(param.doc.as_ref(), &scopes);
                }
            }
            Item::Value(value) => visit(value.doc.as_ref(), &[]),
            Item::TypeAlias(alias) => visit(alias.doc.as_ref(), &[]),
            // Update records mirror a declaration's fields, docs included, and are not written by
            // the author.
            Item::Record(record) if matches!(record.kind, RecordKind::Update { .. }) => {}
            Item::Record(record) => {
                let owner = inline_emit_owners
                    .iter()
                    .find(|(action, _)| *action == &record.name)
                    .map(|(_, component)| *component);
                let scopes = match owner {
                    Some(component) => vec![Scope::Record(record), Scope::Component(component)],
                    None => {
                        visit(record.doc.as_ref(), &[Scope::Record(record)]);
                        vec![Scope::Record(record)]
                    }
                };
                for field in &record.properties {
                    visit(field.doc.as_ref(), &scopes);
                }
            }
            // So do the cases of a derived property union.
            Item::Union(union) if union.property_target.is_some() => {}
            Item::Union(union) => {
                visit(union.doc.as_ref(), &[Scope::Union(union)]);
                for case in &union.cases {
                    let scopes = [Scope::Case(union, case), Scope::Union(union)];
                    visit(case.doc.as_ref(), &scopes);
                    for field in &case.fields {
                        visit(field.doc.as_ref(), &scopes);
                    }
                }
            }
            Item::Component(component) => {
                let scopes = [Scope::Component(component)];
                visit(component.doc.as_ref(), &scopes);
                for member in component.props.iter().chain(&component.state) {
                    visit(member.doc.as_ref(), &scopes);
                }
                for emit in &component.emits {
                    // An inline entry's own payload fields come first.
                    let payload = items.iter().find_map(|item| match item {
                        Item::Record(record)
                            if emit.kind == nx_hir::ComponentEmitKind::Inline
                                && record.name == emit.action_name =>
                        {
                            Some(record)
                        }
                        _ => None,
                    });
                    match payload {
                        Some(record) => visit(
                            emit.doc.as_ref(),
                            &[Scope::Record(record), Scope::Component(component)],
                        ),
                        None => visit(emit.doc.as_ref(), &scopes),
                    }
                }
            }
        }
    }
}

/// A declaration whose members a link's first identifier is looked up among.
#[derive(Clone, Copy)]
enum Scope<'a> {
    Function(&'a nx_hir::Function),
    Record(&'a RecordDef),
    Union(&'a UnionDef),
    Case(&'a UnionDef, &'a UnionCaseDef),
    Component(&'a Component),
}

struct Resolver<'a> {
    module: &'a PreparedModule,
    file_name: &'a str,
    resolved: Vec<ResolvedDocLink>,
    diagnostics: Vec<Diagnostic>,
}

impl Resolver<'_> {
    fn doc(&mut self, doc: Option<&Doc>, scopes: &[Scope<'_>]) {
        let Some(doc) = doc else {
            return;
        };
        for link in doc.links.iter() {
            match self.link(link, scopes) {
                Some(target) => self.resolved.push(ResolvedDocLink {
                    span: link.span,
                    target,
                }),
                None => self.diagnostics.push(
                    Diagnostic::warning("unresolved-doc-link")
                        .with_message(format!(
                            "Doc link `[{}]` does not name a visible declaration or member",
                            link.label
                        ))
                        .with_label(Label::primary(self.file_name, link.span))
                        .with_help(
                            "Name a declaration or one of its members, or write the text in a \
                             code span to keep it from being a link",
                        )
                        .build(),
                ),
            }
        }
    }

    fn link(&self, link: &DocLink, scopes: &[Scope<'_>]) -> Option<DocLinkTarget> {
        self.path(&link.path, scopes)
    }

    /// What the dotted `path` names, looked up as a link's path is.
    fn path(&self, path: &[Name], scopes: &[Scope<'_>]) -> Option<DocLinkTarget> {
        let (first, rest) = path.split_first()?;

        for scope in scopes {
            if let Some(target) = self.scope_member(*scope, first) {
                return self.walk(target, rest);
            }
        }

        // A visible name may itself be dotted, as an import alias is, so the longest prefix of
        // the path that names something is the declaration and the rest are its members.
        for split in (1..=path.len()).rev() {
            let name = Name::new(
                &path[..split]
                    .iter()
                    .map(Name::as_str)
                    .collect::<Vec<_>>()
                    .join("."),
            );
            if self.visible_declaration(&name).is_some() {
                return self.walk(
                    DocLinkTarget {
                        declaration: name,
                        member: None,
                    },
                    &path[split..],
                );
            }
        }
        None
    }

    /// Resolves the member `name` of the declaration `scope` stands for.
    fn scope_member(&self, scope: Scope<'_>, name: &Name) -> Option<DocLinkTarget> {
        let (declaration, member) = match scope {
            Scope::Function(function) => (
                &function.name,
                function
                    .params
                    .iter()
                    .any(|param| &param.name == name)
                    .then(|| DocLinkMember::Parameter(name.clone())),
            ),
            Scope::Record(record) => (&record.name, self.record_member(&record.name, record, name)),
            Scope::Union(union) => (&union.name, union_member(union, name)),
            Scope::Case(union, case) => (
                &union.name,
                case.fields
                    .iter()
                    .any(|field| &field.name == name)
                    .then(|| DocLinkMember::UnionPayloadField {
                        case: case.name.clone(),
                        field: name.clone(),
                    }),
            ),
            Scope::Component(component) => (
                &component.name,
                self.component_member(&component.name, component, name),
            ),
        };
        Some(DocLinkTarget {
            declaration: declaration.clone(),
            member: Some(member?),
        })
    }

    /// Follows the rest of a path from what its earlier parts reached.
    fn walk(&self, mut target: DocLinkTarget, rest: &[Name]) -> Option<DocLinkTarget> {
        for name in rest {
            target.member = Some(match &target.member {
                None => match self.visible_declaration(&target.declaration)?? {
                    Item::Record(record) => {
                        self.record_member(&target.declaration, &record, name)?
                    }
                    Item::Union(union) => union_member(&union, name)?,
                    Item::Component(component) => {
                        self.component_member(&target.declaration, &component, name)?
                    }
                    _ => return None,
                },
                Some(DocLinkMember::UnionCase(case)) => {
                    let Some(Item::Union(union)) = self.visible_declaration(&target.declaration)?
                    else {
                        return None;
                    };
                    let case = union.cases.iter().find(|declared| &declared.name == case)?;
                    case.fields.iter().find(|field| &field.name == name)?;
                    DocLinkMember::UnionPayloadField {
                        case: case.name.clone(),
                        field: name.clone(),
                    }
                }
                Some(_) => return None,
            });
        }
        Some(target)
    }

    /// A field of `record`, which this module names `visible`, inherited ones included where the
    /// base chain resolves.
    fn record_member(
        &self,
        visible: &Name,
        record: &RecordDef,
        name: &Name,
    ) -> Option<DocLinkMember> {
        let declared = record.properties.iter().any(|field| &field.name == name);
        let inherited = || {
            nx_hir::effective_record_shape_for_name(self.module, visible)
                .ok()
                .flatten()
                .is_some_and(|shape| shape.fields.iter().any(|field| &field.name == name))
        };
        (declared || inherited()).then(|| DocLinkMember::RecordField(name.clone()))
    }

    /// A property, `state` field, or `emits` entry of `component`, which this module names
    /// `visible`, inherited properties and entries included where the base chain resolves.
    fn component_member(
        &self,
        visible: &Name,
        component: &Component,
        name: &Name,
    ) -> Option<DocLinkMember> {
        if component.props.iter().any(|prop| &prop.name == name) {
            return Some(DocLinkMember::Property(name.clone()));
        }
        if component.state.iter().any(|field| &field.name == name) {
            return Some(DocLinkMember::State(name.clone()));
        }
        if component.emits.iter().any(|emit| &emit.name == name) {
            return Some(DocLinkMember::Emit(name.clone()));
        }
        let contract = nx_hir::effective_component_contract_for_name(self.module, visible)
            .ok()
            .flatten()?;
        if contract.props.iter().any(|prop| &prop.name == name) {
            Some(DocLinkMember::Property(name.clone()))
        } else if contract.emits.iter().any(|emit| &emit.emit.name == name) {
            Some(DocLinkMember::Emit(name.clone()))
        } else {
            None
        }
    }

    /// The members of the declaration `scope` stands for, as a link's first identifier finds them.
    fn scope_candidates(&self, scope: Scope<'_>) -> Vec<DocLinkCandidate> {
        match scope {
            Scope::Function(function) => function
                .params
                .iter()
                .map(|param| DocLinkCandidate {
                    declaration: function.name.clone(),
                    member: DocLinkMember::Parameter(param.name.clone()),
                    doc: param.doc.clone(),
                })
                .collect(),
            Scope::Record(record) => self.record_candidates(&record.name, record),
            Scope::Union(union) => union_candidates(&union.name, union),
            Scope::Case(union, case) => case_candidates(&union.name, case),
            Scope::Component(component) => self.component_candidates(&component.name, component),
        }
    }

    /// The members of what `target` names: a declaration's members, or a union case's fields.
    fn target_candidates(&self, target: &DocLinkTarget) -> Vec<DocLinkCandidate> {
        let Some(Some(item)) = self.visible_declaration(&target.declaration) else {
            return Vec::new();
        };
        match (&target.member, &item) {
            (None, Item::Record(record)) => self.record_candidates(&target.declaration, record),
            (None, Item::Union(union)) => union_candidates(&target.declaration, union),
            (None, Item::Component(component)) => {
                self.component_candidates(&target.declaration, component)
            }
            (Some(DocLinkMember::UnionCase(case)), Item::Union(union)) => union
                .cases
                .iter()
                .find(|declared| &declared.name == case)
                .map(|case| case_candidates(&target.declaration, case))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// The fields of `record`, which this module names `visible`, inherited ones after its own.
    fn record_candidates(&self, visible: &Name, record: &RecordDef) -> Vec<DocLinkCandidate> {
        let mut candidates = record
            .properties
            .iter()
            .map(|field| DocLinkCandidate {
                declaration: visible.clone(),
                member: DocLinkMember::RecordField(field.name.clone()),
                doc: field.doc.clone(),
            })
            .collect::<Vec<_>>();
        if let Ok(Some(shape)) = nx_hir::effective_record_shape_for_name(self.module, visible) {
            for field in shape.fields {
                if !record.properties.iter().any(|own| own.name == field.name) {
                    candidates.push(DocLinkCandidate {
                        declaration: visible.clone(),
                        member: DocLinkMember::RecordField(field.name),
                        doc: field.doc,
                    });
                }
            }
        }
        candidates
    }

    /// The properties, `state` fields, and `emits` entries of `component`, which this module names
    /// `visible`, inherited properties and entries after its own.
    fn component_candidates(&self, visible: &Name, component: &Component) -> Vec<DocLinkCandidate> {
        let mut candidates = Vec::new();
        let mut push = |member: DocLinkMember, doc: Option<&Doc>| {
            let candidate = DocLinkCandidate {
                declaration: visible.clone(),
                member,
                doc: doc.cloned(),
            };
            if !candidates
                .iter()
                .any(|known: &DocLinkCandidate| known.name() == candidate.name())
            {
                candidates.push(candidate);
            }
        };
        for prop in &component.props {
            push(
                DocLinkMember::Property(prop.name.clone()),
                prop.doc.as_ref(),
            );
        }
        for field in &component.state {
            push(DocLinkMember::State(field.name.clone()), field.doc.as_ref());
        }
        for emit in &component.emits {
            push(DocLinkMember::Emit(emit.name.clone()), emit.doc.as_ref());
        }
        if let Ok(Some(contract)) =
            nx_hir::effective_component_contract_for_name(self.module, visible)
        {
            for prop in &contract.props {
                push(
                    DocLinkMember::Property(prop.name.clone()),
                    prop.doc.as_ref(),
                );
            }
            for emit in &contract.emits {
                push(
                    DocLinkMember::Emit(emit.emit.name.clone()),
                    emit.emit.doc.as_ref(),
                );
            }
        }
        candidates
    }

    /// The declaration visible at the top level under `name`: `Some(Some(item))` for one whose
    /// members a link can name, `Some(None)` for an imported function or value, which has none.
    fn visible_declaration(&self, name: &Name) -> Option<Option<Item>> {
        [
            PreparedNamespace::Type,
            PreparedNamespace::Element,
            PreparedNamespace::Value,
        ]
        .into_iter()
        .find_map(|namespace| {
            let binding = self.module.resolve_binding(namespace, name)?;
            Some(match self.module.resolve_prepared_item(binding)? {
                ResolvedPreparedItem::Raw { item, .. } => Some(item),
                ResolvedPreparedItem::Imported { item, .. } => nx_hir::interface_record(&item)
                    .map(Item::Record)
                    .or_else(|| nx_hir::interface_union(&item).map(Item::Union))
                    .or_else(|| nx_hir::interface_component(&item).map(Item::Component)),
            })
        })
    }
}

fn union_candidates(declaration: &Name, union: &UnionDef) -> Vec<DocLinkCandidate> {
    union
        .cases
        .iter()
        .map(|case| DocLinkCandidate {
            declaration: declaration.clone(),
            member: DocLinkMember::UnionCase(case.name.clone()),
            doc: case.doc.clone(),
        })
        .collect()
}

fn case_candidates(declaration: &Name, case: &UnionCaseDef) -> Vec<DocLinkCandidate> {
    case.fields
        .iter()
        .map(|field| DocLinkCandidate {
            declaration: declaration.clone(),
            member: DocLinkMember::UnionPayloadField {
                case: case.name.clone(),
                field: field.name.clone(),
            },
            doc: field.doc.clone(),
        })
        .collect()
}

fn union_member(union: &UnionDef, name: &Name) -> Option<DocLinkMember> {
    union
        .cases
        .iter()
        .any(|case| &case.name == name)
        .then(|| DocLinkMember::UnionCase(name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze_str;

    fn analyze(source: &str) -> crate::ModuleArtifact {
        analyze_str(source, "links.nx")
    }

    fn link_codes(artifact: &crate::ModuleArtifact) -> Vec<&str> {
        artifact
            .diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.code())
            .collect()
    }

    fn target(declaration: &str, member: Option<DocLinkMember>) -> DocLinkTarget {
        DocLinkTarget {
            declaration: Name::new(declaration),
            member,
        }
    }

    #[test]
    fn a_link_to_a_top_level_declaration_resolves() {
        let artifact = analyze(
            "let renderNotice(): string = \"hi\"\n/// See [renderNotice].\nlet root() = { renderNotice() }\n",
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(artifact.doc_links.len(), 1);
        assert_eq!(artifact.doc_links[0].target, target("renderNotice", None));
        let source = "let renderNotice(): string = \"hi\"\n/// See ";
        assert_eq!(
            usize::from(artifact.doc_links[0].span.start()),
            source.len()
        );
    }

    #[test]
    fn a_link_to_a_union_case_resolves() {
        let artifact = analyze(
            "type LoadState = idle | loading\n/// Starts as [LoadState.idle].\nlet state: LoadState = idle\n",
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "LoadState",
                Some(DocLinkMember::UnionCase(Name::new("idle")))
            )
        );
    }

    #[test]
    fn a_link_to_a_payload_field_resolves() {
        let artifact = analyze(
            "type LoadState =\n  | idle\n  | failed { message:string }\n/// Shows [LoadState.failed.message].\nlet x = 1\n",
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "LoadState",
                Some(DocLinkMember::UnionPayloadField {
                    case: Name::new("failed"),
                    field: Name::new("message"),
                })
            )
        );
    }

    #[test]
    fn a_property_doc_links_to_a_sibling_property() {
        let artifact = analyze(
            r#"component <SearchBox
  value:string
  placeholder:string   /// Ignored when [value] is set.
/> = { <span /> }
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "SearchBox",
                Some(DocLinkMember::Property(Name::new("value")))
            )
        );
    }

    #[test]
    fn a_member_of_the_owner_is_found_before_a_top_level_name() {
        let artifact = analyze(
            r#"let value = 1
type Contact = {
  value:string
  name:string   /// Shown beside [value].
}
"#,
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "Contact",
                Some(DocLinkMember::RecordField(Name::new("value")))
            )
        );
    }

    #[test]
    fn a_link_to_an_inherited_field_resolves() {
        let artifact = analyze(
            r#"abstract type Named = { name:string }
type User extends Named = {
  email:string   /// Sent to [name].
}
/// See [User.name].
let x = 1
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(artifact.doc_links.len(), 2);
    }

    #[test]
    fn a_link_to_a_component_emit_resolves() {
        let artifact = analyze(
            r#"/// Raises [ValueChanged].
component <SearchBox
  emits {
    ValueChanged { value:string }   /// Carries [value].
  }
/> = { <span /> }
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "SearchBox",
                Some(DocLinkMember::Emit(Name::new("ValueChanged")))
            )
        );
        assert_eq!(
            artifact.doc_links[1].target,
            target(
                "SearchBox.ValueChanged",
                Some(DocLinkMember::RecordField(Name::new("value")))
            )
        );
    }

    #[test]
    fn a_link_to_an_inherited_component_member_resolves() {
        let artifact = analyze(
            r#"abstract component <Base
  label:string
  emits { Clicked { at:int } }
/>
component <Derived extends Base
  tone:string   /// Unlike [label].
/> = { <span /> }
/// See [Derived.label] and [Derived.Clicked].
let x = 1
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        let targets = artifact
            .doc_links
            .iter()
            .map(|link| link.target.clone())
            .collect::<Vec<_>>();
        assert!(targets.contains(&target(
            "Derived",
            Some(DocLinkMember::Property(Name::new("label")))
        )));
        assert!(targets.contains(&target(
            "Derived",
            Some(DocLinkMember::Emit(Name::new("Clicked")))
        )));
    }

    #[test]
    fn a_link_to_a_hyphenated_property_resolves() {
        let artifact = analyze(
            r#"component <Box
  aria-label:string
  tone:string   /// Read after [aria-label].
/> = { <span /> }
/// See [Box.aria-label].
let x = 1
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(artifact.doc_links.len(), 2);
        assert!(artifact.doc_links.iter().all(|link| link.target
            == target(
                "Box",
                Some(DocLinkMember::Property(Name::new("aria-label")))
            )));
    }

    #[test]
    fn an_inline_emit_payload_field_can_link_to_its_components_properties() {
        let artifact = analyze(
            r#"component <SearchBox
  placeholder:string
  emits {
    ValueChanged {
      value:string   /// Never [placeholder].
    }
  }
/> = { <span /> }
"#,
        );
        assert!(
            link_codes(&artifact).is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert_eq!(
            artifact.doc_links[0].target,
            target(
                "SearchBox",
                Some(DocLinkMember::Property(Name::new("placeholder")))
            )
        );
    }

    /// The candidate names at the `⟨cursor⟩` in `marked`, with the qualifier the link has so far.
    fn candidates(marked: &str, qualifier: &[&str]) -> Vec<String> {
        let offset = marked.find("⟨cursor⟩").expect("a cursor");
        let source = marked.replace("⟨cursor⟩", "");
        let artifact = analyze(&source);
        let module = artifact.prepared_module.clone().expect("a prepared module");
        let qualifier = qualifier
            .iter()
            .map(|part| Name::new(part))
            .collect::<Vec<_>>();
        doc_link_candidates(&module, TextSize::from(offset as u32), &qualifier)
            .iter()
            .map(|candidate| candidate.name().to_string())
            .collect()
    }

    #[test]
    fn the_owners_members_are_candidates_innermost_first() {
        let source = r#"type LoadState =
  | idle
  | failed { message:string }   /// Shows [⟨cursor⟩
"#;
        assert_eq!(candidates(source, &[]), ["message", "idle", "failed"]);

        let source = r#"/// Starts at [⟨cursor⟩
component <Counter
  start:int = 0
/> = {
  state { count:int = 0 }
  <span />
}
"#;
        assert_eq!(candidates(source, &[]), ["start", "count"]);
    }

    #[test]
    fn a_qualifier_offers_the_members_of_what_it_names() {
        let source = r#"abstract type Named = { name:string /// The name.
}
type User extends Named = { email:string }
type LoadState =
  | idle   /// Nothing yet.
  | failed { message:string }
/// See [⟨cursor⟩
let x = 1
"#;
        assert_eq!(candidates(source, &["LoadState"]), ["idle", "failed"]);
        assert_eq!(candidates(source, &["LoadState", "failed"]), ["message"]);
        assert_eq!(candidates(source, &["User"]), ["email", "name"]);
        assert!(candidates(source, &["Missing"]).is_empty());
        assert!(candidates(source, &["x"]).is_empty());
    }

    #[test]
    fn a_candidate_carries_its_documentation() {
        let source = r#"type LoadState =
  | idle   /// Nothing yet.
  | loading
/// See [⟨cursor⟩
let x = 1
"#;
        let offset = source.find("⟨cursor⟩").unwrap();
        let artifact = analyze(&source.replace("⟨cursor⟩", ""));
        let module = artifact.prepared_module.clone().unwrap();
        let found = doc_link_candidates(
            &module,
            TextSize::from(offset as u32),
            &[Name::new("LoadState")],
        );
        assert_eq!(
            found[0].doc.as_ref().map(|doc| doc.text.as_str()),
            Some("Nothing yet.")
        );
        assert_eq!(found[1].doc, None);
    }

    #[test]
    fn an_unresolved_link_is_a_warning_and_the_module_still_compiles() {
        let source = "/// See [Missing].\nlet root() = { 1 }\n";
        let artifact = analyze(source);
        assert!(artifact.is_ok(), "{:?}", artifact.diagnostics);
        assert!(artifact.doc_links.is_empty());
        let warning = artifact
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code() == Some("unresolved-doc-link"))
            .expect("unresolved-doc-link");
        assert_eq!(warning.severity(), nx_diagnostics::Severity::Warning);
        let label = &warning.labels()[0];
        assert_eq!(&source[label.range], "[Missing]");
    }

    #[test]
    fn a_path_through_a_member_that_has_no_members_does_not_resolve() {
        let artifact =
            analyze("type Contact = { name:string }\n/// See [Contact.name.first].\nlet x = 1\n");
        assert_eq!(link_codes(&artifact), vec!["unresolved-doc-link"]);
    }

    #[test]
    fn brackets_in_code_and_explicit_links_report_nothing() {
        let artifact = analyze(
            "/// Read `items[index]` and [the spec](https://nxlang.org/spec).\nlet x = 1\n",
        );
        assert!(
            artifact.diagnostics.is_empty(),
            "{:?}",
            artifact.diagnostics
        );
        assert!(artifact.doc_links.is_empty());
    }

    #[test]
    fn links_in_synthesized_items_are_resolved_once() {
        let artifact = analyze(
            r#"type Contact = {
  name:string   /// See [Missing].
}
component <SearchBox
  emits {
    /// Also [Missing].
    ValueChanged {
      value:string   /// And [Missing].
    }
  }
/> = { <span /> }
"#,
        );
        assert_eq!(
            link_codes(&artifact),
            vec![
                "unresolved-doc-link",
                "unresolved-doc-link",
                "unresolved-doc-link"
            ],
            "{:?}",
            artifact.diagnostics
        );
    }
}
