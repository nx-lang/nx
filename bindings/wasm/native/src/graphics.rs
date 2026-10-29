//! Shared neutral preparation for browser hosts. Provenance and text measurements are supplied by
//! the trusted host's IR evaluation, not accepted as an authentication boundary for arbitrary data.
use super::{
    internal_error, parse_request, result_json, Operation, OperationError, ProgramArtifact,
};
use nx_graphics::{Config, DeclarationResolver, Identity, TextRequest, TextRun, TextService};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Origin {
    path: String,
    module: String,
    start: usize,
    end: usize,
    #[serde(default)]
    #[serde(rename = "callSites")]
    call_sites: Vec<Value>,
}
#[derive(Deserialize)]
struct Measurement {
    request: TextRequest,
    run: TextRun,
}
#[derive(Deserialize)]
struct Request {
    value: nx_value::NxValue,
    origins: Vec<Origin>,
    #[serde(default)]
    measurements: Vec<Measurement>,
    #[serde(default)]
    preflight: bool,
    config: Config,
}
struct Resolver {
    ids: BTreeMap<String, Identity>,
    sources: BTreeMap<String, Value>,
}
impl DeclarationResolver for Resolver {
    fn resolve(&self, path: &str) -> Option<Identity> {
        self.ids.get(path).cloned()
    }
    fn provenance(&self, path: &str) -> Option<Value> {
        self.sources.get(path).cloned()
    }
}
struct Measurements(Vec<Measurement>);
impl TextService for Measurements {
    fn shape(&mut self, request: &TextRequest) -> nx_graphics::Result<TextRun> {
        self.0
            .iter()
            .find(|m| m.request == *request)
            .map(|m| m.run.clone())
            .ok_or_else(|| nx_graphics::error("", "missing browser text measurement"))
    }
}

#[derive(Default)]
struct Requests(Vec<TextRequest>);
impl TextService for Requests {
    fn shape(&mut self, request: &TextRequest) -> nx_graphics::Result<TextRun> {
        if !self.0.contains(request) {
            self.0.push(request.clone());
        }
        Ok(TextRun {
            handle: 0,
            font: "preflight".into(),
            advance: 0.0,
            ink: nx_graphics::Bounds::empty(),
            ascent: 0.0,
            descent: 0.0,
            baseline: 0.0,
        })
    }
}

fn text_request_sources(
    node: &nx_graphics::Node,
    seen: &mut std::collections::BTreeSet<String>,
    output: &mut Vec<Value>,
) {
    if let Some(text) = &node.text {
        let request = json!(&text.request);
        if seen.insert(request.to_string()) {
            output.push(json!({"request":request,"element":node.id,"provenance":node.provenance}));
        }
    }
    for child in &node.children {
        text_request_sources(child, seen, output);
    }
}

pub(super) fn prepare(
    program: &ProgramArtifact,
    argument: Result<&str, OperationError>,
) -> Operation {
    let mut request: Request = parse_request(argument?)?;
    let version = nx_api::standard_library_entry("@nx/graphics")
        .ok_or_else(|| internal_error("graphics library unavailable"))?
        .version();
    request.config.graphics_version = version.clone();
    request.config.program_revision = format!("{:016x}", program.resolved_program.fingerprint);
    let mut resolver = Resolver {
        ids: BTreeMap::new(),
        sources: BTreeMap::new(),
    };
    let declarations = program.construction_declarations();
    for origin in request.origins {
        resolver.sources.insert(
            origin.path.clone(),
            json!({"module":origin.module,"start":origin.start,"end":origin.end,"callSites":origin.call_sites}),
        );
        if let Some(identity) = declarations.get(&(origin.module, origin.start, origin.end)) {
            resolver.ids.insert(
                origin.path,
                Identity {
                    module: identity.module.clone(),
                    version: version.clone(),
                    declaration: identity.declaration.clone(),
                },
            );
        }
    }
    if request.preflight {
        let mut requests = Requests::default();
        return match nx_graphics::prepare_value(
            &request.value,
            &resolver,
            &mut requests,
            request.config,
        ) {
            Ok(scene) => {
                let mut texts = Vec::new();
                text_request_sources(
                    &scene.root,
                    &mut std::collections::BTreeSet::new(),
                    &mut texts,
                );
                result_json(&json!({"scene":null,"diagnostics":[],"textRequests":texts}))
            }
            Err(diagnostic) => result_json(&json!({"scene":null,"diagnostics":[diagnostic]})),
        };
    }
    match nx_graphics::prepare_value(
        &request.value,
        &resolver,
        &mut Measurements(request.measurements),
        request.config,
    ) {
        Ok(scene) => result_json(&json!({"scene":scene,"diagnostics":[]})),
        Err(diagnostic) => result_json(&json!({"scene":null,"diagnostics":[diagnostic]})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_deduplicates_text_without_a_font_service() {
        let mut collector = Requests::default();
        let request = TextRequest {
            text: "hello".into(),
            family: "Fixture".into(),
            size: 16.0,
            weight: 400,
            italic: false,
        };
        assert_eq!(collector.shape(&request).unwrap().advance, 0.0);
        collector.shape(&request).unwrap();
        assert_eq!(collector.0, vec![request]);
        assert!(serde_json::from_value::<Request>(json!({})).is_err());
    }
}
