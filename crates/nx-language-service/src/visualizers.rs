//! Resolving the visualizer declarations a document's program loads, and a host's visualizer
//! modules, into one visualizer per declaration.
//!
//! <para>Visualizer declarations apply in order, each later one overriding only what it sets: along
//! inheritance from the furthest base to the declaration itself, and at each step the visualizer in
//! the declaring module, then those in the program's other modules, then the host's visualizer
//! modules in the order the host names them. `PropertyVisualizer`s and `CaseVisualizer`s merge by
//! `of`. A base's `name` is not inherited, since it names the base.</para>

use crate::{DeclarationOrigin, WorkspaceDeclarations};
use nx_api::{
    ProgramModule, ProgramVisualizer, VisualizerAttribute, VisualizerChild, VisualizerValue,
};
use nx_hir::VisualizerRecord;
use rustc_hash::{FxHashMap, FxHashSet};

/// The visualizer declarations that apply to one document's program, by target.
#[derive(Debug, Default)]
pub(crate) struct ProgramVisualizers {
    /// Each target's visualizer declarations from the program, the declaring module's first.
    program: FxHashMap<DeclarationOrigin, Vec<ProgramVisualizer>>,
    /// Each target's visualizer declarations from the host's visualizer modules, in the order
    /// named.
    host: FxHashMap<DeclarationOrigin, Vec<ProgramVisualizer>>,
    /// The host visualizer modules the snapshot does not hold.
    pub missing: Vec<String>,
}

impl ProgramVisualizers {
    /// Gathers the visualizer declarations of the program whose entry is `entry`, and of the host
    /// visualizer modules named in `hosts`.
    ///
    /// <para>Two visualizer declarations of one target in two program modules that do not declare
    /// it are a `duplicate-visualizer` error, which analysis reports, and neither applies. A
    /// visualizer declaration with errors counts as one of two such, and never applies
    /// itself.</para>
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
        let visualizers =
            nx_api::program_visualizers(&modules, |identity| workspace.artifact(identity));
        let duplicates = nx_api::duplicate_program_visualizers(&visualizers)
            .into_iter()
            .map(|(origin, _)| origin)
            .collect::<FxHashSet<_>>();

        let mut gathered = ProgramVisualizers::default();
        // The declaring module's visualizer declaration applies before any other module's.
        let (declaring, others): (Vec<_>, Vec<_>) = visualizers
            .into_iter()
            .partition(ProgramVisualizer::is_in_declaring_module);
        for visualizer in declaring
            .into_iter()
            .chain(
                others
                    .into_iter()
                    .filter(|visualizer| !duplicates.contains(&visualizer.target.origin)),
            )
            .filter(|visualizer| visualizer.clean)
        {
            gathered
                .program
                .entry(origin_of(&visualizer))
                .or_default()
                .push(visualizer);
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
            for visualizer in
                nx_api::program_visualizers(&module, |identity| workspace.artifact(identity))
                    .into_iter()
                    .filter(|visualizer| visualizer.clean)
            {
                gathered
                    .host
                    .entry(origin_of(&visualizer))
                    .or_default()
                    .push(visualizer);
            }
        }
        gathered
    }

    /// The visualizer declarations of `origin`, in the order they apply.
    fn of(&self, origin: &DeclarationOrigin) -> impl Iterator<Item = &ProgramVisualizer> {
        self.program
            .get(origin)
            .into_iter()
            .flatten()
            .chain(self.host.get(origin).into_iter().flatten())
    }

    /// The resolved visualizer of the declaration at `origin`, whose bases are `chain`, nearest
    /// first; `None` when no visualizer declaration of it or of a base applies.
    pub fn resolve(
        &self,
        origin: &DeclarationOrigin,
        chain: &[DeclarationOrigin],
    ) -> Option<ResolvedVisualizer> {
        let mut resolved: Option<ResolvedVisualizer> = None;
        for step in chain.iter().rev().chain(std::iter::once(origin)) {
            let own = step == origin;
            for visualizer in self.of(step) {
                let current = resolved
                    .get_or_insert_with(|| ResolvedVisualizer::new(visualizer.value.record));
                current.apply(&visualizer.value, own);
            }
        }
        resolved
    }
}

fn origin_of(visualizer: &ProgramVisualizer) -> DeclarationOrigin {
    (
        visualizer.target.origin.module_identity().to_string(),
        visualizer.target.origin.definition_id(),
    )
}

/// One declaration's visualizer, with every visualizer declaration that applies to it merged in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolvedVisualizer {
    pub record: Option<VisualizerRecord>,
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
    /// The property visualizers, in their resolved order.
    pub properties: Vec<ResolvedChild>,
    /// The case visualizers, in the order they were first written.
    pub cases: Vec<ResolvedChild>,
}

/// One merged `PropertyVisualizer` or `CaseVisualizer`: what it describes and the attributes it
/// sets.
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

impl ResolvedVisualizer {
    fn new(record: VisualizerRecord) -> Self {
        Self {
            record: Some(record),
            ..Self::default()
        }
    }

    /// Applies one visualizer declaration's value over what earlier ones resolved; `own` says
    /// whether it is a visualizer of the declaration itself rather than of a base.
    fn apply(&mut self, value: &VisualizerValue, own: bool) {
        let text = |name: &str| {
            value
                .attribute(name)
                .map(VisualizerAttribute::text)
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
            VisualizerRecord::Element => {
                // The later visualizer's property visualizers come first, in its order; the earlier
                // ones it does not name follow in theirs. Two of one property in one visualizer
                // merge too.
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
            VisualizerRecord::Union => {
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
            VisualizerRecord::Value => {}
        }
    }
}

fn child_of(child: &VisualizerChild) -> Option<String> {
    child.attribute("of").map(VisualizerAttribute::text)
}

fn merge_child(merged: &mut ResolvedChild, child: &VisualizerChild) {
    for (name, value) in &child.attributes {
        let text = value.text();
        if name != "of" && is_known_value(name, &text) {
            merged.set(name, text);
        }
    }
}

/// Whether `text` is a value the visualizer protocol allows for the attribute `name`. Analysis
/// already rejects any other, and a visualizer declaration with an error never applies; this keeps
/// a value outside the protocol's unions from reaching a reader all the same.
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
