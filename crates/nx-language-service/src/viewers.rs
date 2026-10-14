//! Resolving the viewer declarations a document's program loads, and a host's viewer modules,
//! into one viewer per declaration.
//!
//! <para>Viewer declarations apply in order, each later one overriding only what it sets: along
//! inheritance from the furthest base to the declaration itself, and at each step the viewer in
//! the declaring module, then those in the program's other modules, then the host's viewer modules
//! in the order the host names them. `PropertyViewer`s and `CaseViewer`s merge by `of`. A base's
//! `name` is not inherited, since it names the base.</para>

use crate::{DeclarationOrigin, WorkspaceDeclarations};
use nx_api::{ProgramModule, ProgramViewer, ViewerAttribute, ViewerChild, ViewerValue};
use nx_hir::ViewerRecord;
use rustc_hash::{FxHashMap, FxHashSet};

/// The viewer declarations that apply to one document's program, by target.
#[derive(Debug, Default)]
pub(crate) struct ProgramViewers {
    /// Each target's viewer declarations from the program, the declaring module's first.
    program: FxHashMap<DeclarationOrigin, Vec<ProgramViewer>>,
    /// Each target's viewer declarations from the host's viewer modules, in the order named.
    host: FxHashMap<DeclarationOrigin, Vec<ProgramViewer>>,
    /// The host viewer modules the snapshot does not hold.
    pub missing: Vec<String>,
}

impl ProgramViewers {
    /// Gathers the viewer declarations of the program whose entry is `entry`, and of the host
    /// viewer modules named in `hosts`.
    ///
    /// <para>Two viewer declarations of one target in two program modules that do not declare it
    /// are a `duplicate-viewer` error, which analysis reports, and neither applies. A viewer
    /// declaration with errors counts as one of two such, and never applies itself.</para>
    pub fn gather(
        workspace: &WorkspaceDeclarations,
        entry: &str,
        is_document: impl Fn(&str) -> bool,
        build_context: &nx_api::ProgramBuildContext,
        hosts: &[String],
    ) -> Self {
        let modules = nx_api::program_modules(
            entry,
            |identity| {
                is_document(identity)
                    .then(|| workspace.artifact(identity))
                    .flatten()
                    .map(|artifact| artifact.imports.as_slice())
            },
            build_context,
        );
        let viewers = nx_api::program_viewers(&modules, |identity| workspace.artifact(identity));
        let duplicates = nx_api::duplicate_program_viewers(&viewers)
            .into_iter()
            .map(|(origin, _)| origin)
            .collect::<FxHashSet<_>>();

        let mut gathered = ProgramViewers::default();
        // The declaring module's viewer declaration applies before any other module's.
        let (declaring, others): (Vec<_>, Vec<_>) = viewers
            .into_iter()
            .partition(ProgramViewer::is_in_declaring_module);
        for viewer in declaring
            .into_iter()
            .chain(
                others
                    .into_iter()
                    .filter(|viewer| !duplicates.contains(&viewer.target.origin)),
            )
            .filter(|viewer| viewer.clean)
        {
            gathered
                .program
                .entry(origin_of(&viewer))
                .or_default()
                .push(viewer);
        }

        for host in hosts {
            if !is_document(host) || workspace.artifact(host).is_none() {
                gathered.missing.push(host.clone());
                continue;
            }
            let module = [ProgramModule {
                identity: host.clone(),
                via: None,
            }];
            for viewer in nx_api::program_viewers(&module, |identity| workspace.artifact(identity))
                .into_iter()
                .filter(|viewer| viewer.clean)
            {
                gathered
                    .host
                    .entry(origin_of(&viewer))
                    .or_default()
                    .push(viewer);
            }
        }
        gathered
    }

    /// The viewer declarations of `origin`, in the order they apply.
    fn of(&self, origin: &DeclarationOrigin) -> impl Iterator<Item = &ProgramViewer> {
        self.program
            .get(origin)
            .into_iter()
            .flatten()
            .chain(self.host.get(origin).into_iter().flatten())
    }

    /// The resolved viewer of the declaration at `origin`, whose bases are `chain`, nearest first;
    /// `None` when no viewer declaration of it or of a base applies.
    pub fn resolve(
        &self,
        origin: &DeclarationOrigin,
        chain: &[DeclarationOrigin],
    ) -> Option<ResolvedViewer> {
        let mut resolved: Option<ResolvedViewer> = None;
        for step in chain.iter().rev().chain(std::iter::once(origin)) {
            let own = step == origin;
            for viewer in self.of(step) {
                let current =
                    resolved.get_or_insert_with(|| ResolvedViewer::new(viewer.value.record));
                current.apply(&viewer.value, own);
            }
        }
        resolved
    }
}

fn origin_of(viewer: &ProgramViewer) -> DeclarationOrigin {
    (
        viewer.target.origin.module_identity().to_string(),
        viewer.target.origin.definition_id(),
    )
}

/// One declaration's viewer, with every viewer declaration that applies to it merged in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedViewer {
    pub record: Option<ViewerRecord>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub badge: Option<String>,
    pub image: Option<String>,
    pub id: Option<String>,
    pub id_position: Option<String>,
    pub icon: Option<String>,
    pub layout: Option<String>,
    pub format: Option<String>,
    /// The property viewers, in their resolved order.
    pub properties: Vec<ResolvedChild>,
    /// The case viewers, in the order they were first written.
    pub cases: Vec<ResolvedChild>,
}

/// One merged `PropertyViewer` or `CaseViewer`: what it describes and the attributes it sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedChild {
    pub of: String,
    pub attributes: Vec<(String, String)>,
}

impl ResolvedChild {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn set(&mut self, name: &str, value: String) {
        match self.attributes.iter_mut().find(|(key, _)| key == name) {
            Some((_, existing)) => *existing = value,
            None => self.attributes.push((name.to_string(), value)),
        }
    }
}

impl ResolvedViewer {
    fn new(record: ViewerRecord) -> Self {
        Self {
            record: Some(record),
            ..Self::default()
        }
    }

    /// Applies one viewer declaration's value over what earlier ones resolved; `own` says whether
    /// it is a viewer of the declaration itself rather than of a base.
    fn apply(&mut self, value: &ViewerValue, own: bool) {
        let text = |name: &str| {
            value
                .attribute(name)
                .map(ViewerAttribute::text)
                .filter(|text| is_known_value(name, text))
        };
        if own {
            if let Some(name) = text("name") {
                self.name = Some(name);
            }
        }
        for (slot, name) in [
            (&mut self.title, "title"),
            (&mut self.subtitle, "subtitle"),
            (&mut self.badge, "badge"),
            (&mut self.image, "image"),
            (&mut self.id, "id"),
            (&mut self.id_position, "idPosition"),
            (&mut self.icon, "icon"),
            (&mut self.layout, "layout"),
            (&mut self.format, "format"),
        ] {
            if let Some(value) = text(name) {
                *slot = Some(value);
            }
        }
        match value.record {
            ViewerRecord::Element => {
                // The later viewer's property viewers come first, in its order; the earlier ones
                // it does not name follow in theirs. Two of one property in one viewer merge too.
                let mut earlier = std::mem::take(&mut self.properties);
                for child in &value.children {
                    let Some(of) = child_of(child) else {
                        continue;
                    };
                    if let Some(merged) = self
                        .properties
                        .iter_mut()
                        .find(|existing| existing.of == of)
                    {
                        merge_child(merged, child);
                        continue;
                    }
                    let mut merged = match earlier.iter().position(|existing| existing.of == of) {
                        Some(index) => earlier.remove(index),
                        None => ResolvedChild {
                            of,
                            attributes: Vec::new(),
                        },
                    };
                    merge_child(&mut merged, child);
                    self.properties.push(merged);
                }
                self.properties.extend(earlier);
            }
            ViewerRecord::Union => {
                for child in &value.children {
                    let Some(of) = child_of(child) else {
                        continue;
                    };
                    match self.cases.iter_mut().find(|existing| existing.of == of) {
                        Some(existing) => merge_child(existing, child),
                        None => {
                            let mut merged = ResolvedChild {
                                of,
                                attributes: Vec::new(),
                            };
                            merge_child(&mut merged, child);
                            self.cases.push(merged);
                        }
                    }
                }
            }
            ViewerRecord::Value => {}
        }
    }
}

fn child_of(child: &ViewerChild) -> Option<String> {
    child.attribute("of").map(ViewerAttribute::text)
}

fn merge_child(merged: &mut ResolvedChild, child: &ViewerChild) {
    for (name, value) in &child.attributes {
        let text = value.text();
        if name != "of" && is_known_value(name, &text) {
            merged.set(name, text);
        }
    }
}

/// Whether `text` is a value the viewer protocol allows for the attribute `name`. Analysis already
/// rejects any other, and a viewer declaration with an error never applies; this keeps a value
/// outside the protocol's unions from reaching a reader all the same.
fn is_known_value(name: &str, text: &str) -> bool {
    match name {
        "layout" => matches!(text, "card" | "row" | "inline"),
        "place" => matches!(text, "body" | "details"),
        "idPosition" => matches!(text, "beginning" | "end"),
        "format" => matches!(
            text,
            "text" | "markdown" | "url" | "image" | "color" | "email" | "phone" | "id"
        ),
        _ => true,
    }
}
