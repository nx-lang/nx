//! The conformance corpus as the runtime's tests read it: from disk, with nothing of the compiler.

#![allow(dead_code)]

use nx_ir_runtime::{LinkOptions, PreparedModule, Program};
use nx_value::NxValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub struct Entrypoint {
    pub module: String,
    pub function: String,
}

pub struct Lifecycle {
    pub module: String,
    pub component: String,
    pub props: BTreeMap<String, NxValue>,
    pub batches: Vec<Vec<NxValue>>,
}

pub struct CorpusProgram {
    pub name: String,
    /// Each emitted module's image, by identity: with its debug section, and stripped.
    pub images: Vec<(String, Vec<u8>)>,
    pub stripped_images: Vec<(String, Vec<u8>)>,
    pub entrypoints: Vec<Entrypoint>,
    pub lifecycles: Vec<Lifecycle>,
    pub results: serde_json::Value,
}

pub fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs/ir-conformance")
}

pub fn value(json: &serde_json::Value) -> NxValue {
    NxValue::from_json_str(&json.to_string()).expect("a canonical value")
}

pub fn properties(json: Option<&serde_json::Value>) -> BTreeMap<String, NxValue> {
    json.and_then(|json| json.as_object())
        .map(|object| {
            object
                .iter()
                .map(|(key, item)| (key.clone(), value(item)))
                .collect()
        })
        .unwrap_or_default()
}

pub fn load_corpus() -> Vec<CorpusProgram> {
    let root = corpus_root();
    let mut names: Vec<String> = fs::read_dir(&root)
        .expect("the corpus directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let dir = root.join(&name);
            let read_json = |path: PathBuf| -> serde_json::Value {
                serde_json::from_str(
                    &fs::read_to_string(&path)
                        .unwrap_or_else(|error| panic!("{}: {error}", path.display())),
                )
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            };
            let manifest = read_json(dir.join("program.json"));
            let text =
                |json: &serde_json::Value, key: &str| json[key].as_str().expect(key).to_string();
            let images = |suffix: &str| -> Vec<(String, Vec<u8>)> {
                manifest["emit"]
                    .as_array()
                    .expect("emit")
                    .iter()
                    .map(|identity| {
                        let identity = identity.as_str().expect("identity").to_string();
                        let path = dir
                            .join("expected")
                            .join(format!("{}{suffix}", identity.replace('/', "__")));
                        let bytes = fs::read(&path)
                            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                        (identity, bytes)
                    })
                    .collect()
            };
            CorpusProgram {
                images: images(".nxir"),
                stripped_images: images(".stripped.nxir"),
                entrypoints: manifest["entrypoints"]
                    .as_array()
                    .map(|entrypoints| {
                        entrypoints
                            .iter()
                            .map(|entrypoint| Entrypoint {
                                module: text(entrypoint, "module"),
                                function: text(entrypoint, "function"),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                lifecycles: manifest["lifecycles"]
                    .as_array()
                    .map(|lifecycles| {
                        lifecycles
                            .iter()
                            .map(|lifecycle| Lifecycle {
                                module: text(lifecycle, "module"),
                                component: text(lifecycle, "component"),
                                props: properties(lifecycle.get("props")),
                                batches: lifecycle["batches"]
                                    .as_array()
                                    .expect("batches")
                                    .iter()
                                    .map(|batch| {
                                        batch
                                            .as_array()
                                            .expect("a batch")
                                            .iter()
                                            .map(value)
                                            .collect()
                                    })
                                    .collect(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                results: read_json(dir.join("expected").join("results.json")),
                name,
            }
        })
        .collect()
}

/// Prepares a program's images and returns a function that links any one of them against the
/// rest. Only the program's own modules are supplied, so an image that names the prelude links
/// it through the runtime's built-in one.
pub fn prepare_all(images: &[(String, Vec<u8>)]) -> BTreeMap<String, PreparedModule> {
    images
        .iter()
        .map(|(identity, bytes)| {
            let module = PreparedModule::prepare(bytes.clone())
                .unwrap_or_else(|error| panic!("{identity}: {error}"));
            (identity.clone(), module)
        })
        .collect()
}

pub fn link(modules: &BTreeMap<String, PreparedModule>, entry: &str) -> Program {
    let module = modules
        .get(entry)
        .unwrap_or_else(|| panic!("the corpus emits no image for {entry}"));
    Program::link(
        module,
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{entry}: {error}"))
}

/// Canonical equality: numbers by value, so `3` and `3.0` are one value, records by their
/// fields whatever their order, and a wide integer in either of its spellings.
pub fn canonical_eq(left: &NxValue, right: &NxValue) -> bool {
    // An integer outside JavaScript's safe range is `{ "$type": "nx.int", "value": "<digits>" }`
    // in canonical JSON; in Rust it is the integer.
    let wide = |value: &NxValue| match value {
        NxValue::Record {
            type_name: Some(name),
            properties,
        } if name == "nx.int" => match properties.get("value") {
            Some(NxValue::String(digits)) => digits.parse::<i64>().ok(),
            _ => None,
        },
        NxValue::Int(value) => Some(*value),
        _ => None,
    };
    if let (Some(left), Some(right)) = (wide(left), wide(right)) {
        return left == right;
    }
    let number = |value: &NxValue| match value {
        NxValue::Int32(value) => Some(f64::from(*value)),
        NxValue::Int(value) => Some(*value as f64),
        NxValue::Float32(value) => Some(f64::from(*value)),
        NxValue::Float(value) => Some(*value),
        _ => None,
    };
    if let (Some(left), Some(right)) = (number(left), number(right)) {
        return left == right;
    }
    match (left, right) {
        (NxValue::Array(left), NxValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| canonical_eq(left, right))
        }
        (
            NxValue::Record {
                type_name: left_type,
                properties: left,
            },
            NxValue::Record {
                type_name: right_type,
                properties: right,
            },
        ) => {
            left_type == right_type
                && left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| canonical_eq(value, other))
                })
        }
        _ => left == right,
    }
}

pub fn json(value: &NxValue) -> String {
    value
        .to_json_string()
        .unwrap_or_else(|error| format!("<{error}>"))
}
