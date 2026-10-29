//! Local IR-to-PNG host. Output is published by atomically replacing a pointer to an immutable revision.
use nx_graphics::{Config, DeclarationResolver, Identity, TextRequest, TextService};
use nx_graphics_skia::{render_png, Fonts};
use nx_ir_runtime::{LinkOptions, Origins, PreparedModule, Program, RuntimeOptions};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Resolver {
    identities: BTreeMap<String, Identity>,
    sources: BTreeMap<String, Value>,
}
impl DeclarationResolver for Resolver {
    fn resolve(&self, path: &str) -> Option<Identity> {
        self.identities.get(path).cloned()
    }
    fn provenance(&self, path: &str) -> Option<Value> {
        self.sources.get(path).cloned()
    }
}
fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|s| s == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
/// Reject typos before loading fonts, compiling, or allocating render resources.
fn options(raw: impl IntoIterator<Item = String>) -> std::result::Result<Vec<String>, String> {
    let known = [
        "--entry",
        "--output",
        "--width",
        "--height",
        "--scale",
        "--root",
        "--fonts",
        "--limits",
        "--source-root",
        "--measure-text",
        "--inspect",
        "--revision",
        "--id",
    ];
    let mut values = BTreeMap::new();
    let mut raw = raw.into_iter();
    while let Some(option) = raw.next() {
        let (name, inline) = option
            .split_once('=')
            .map_or((option.as_str(), None), |(k, v)| (k, Some(v.to_owned())));
        if !known.contains(&name) {
            return Err(format!("unknown option {name}"));
        }
        let value = inline
            .or_else(|| raw.next())
            .filter(|v| !v.starts_with("--"))
            .ok_or_else(|| format!("{name} requires a value"))?;
        if values.insert(name.to_owned(), value).is_some() {
            return Err(format!("duplicate option {name}"));
        }
    }
    Ok(values.into_iter().flat_map(|(k, v)| [k, v]).collect())
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FontFace {
    family: String,
    #[serde(default = "normal_weight")]
    weight: u16,
    #[serde(default)]
    italic: bool,
    path: String,
    sha256: Option<String>,
    license: Option<String>,
    upstream: Option<String>,
}
fn normal_weight() -> u16 {
    400
}
fn named_object<T: serde::de::DeserializeOwned>(
    value: Value,
    name: &str,
) -> std::result::Result<T, Box<dyn std::error::Error>> {
    if !value.is_object() {
        return Err(format!("{name} must be a JSON object").into());
    }
    serde_json::from_value(value).map_err(|e| format!("{name}: {e}").into())
}
fn run(
    program_revision: &std::cell::Cell<Option<u64>>,
) -> std::result::Result<Value, Box<dyn std::error::Error>> {
    let args = options(std::env::args().skip(1)).map_err(std::io::Error::other)?;
    let mut fonts = if let Some(path) = arg(&args, "--fonts") {
        let entries: Vec<Value> = serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|e| std::io::Error::other(format!("font manifest {path}: {e}")))?;
        let mut fonts = Fonts::default();
        for (index, entry) in entries.into_iter().enumerate() {
            let face: FontFace =
                named_object(entry, &format!("font manifest {path}, face {index}"))?;
            let file = Path::new(&path)
                .parent()
                .unwrap_or(Path::new("."))
                .join(&face.path);
            let bytes = std::fs::read(&file).map_err(|e| {
                std::io::Error::other(format!("font face {index} {}: {e}", file.display()))
            })?;
            if let Some(expected) = face.sha256 {
                use sha2::{Digest, Sha256};
                if format!("{:x}", Sha256::digest(&bytes)) != expected {
                    return Err(
                        format!("font face {index} {}: sha256 mismatch", file.display()).into(),
                    );
                }
            }
            let _metadata = (face.license, face.upstream);
            fonts
                .register(&face.family, face.weight, face.italic, &bytes)
                .map_err(|e| {
                    std::io::Error::other(format!("font face {index} {}: {e}", file.display()))
                })?;
        }
        fonts
    } else {
        Fonts::fixture()?
    };
    if let Some(request) = arg(&args, "--measure-text") {
        let request: TextRequest = named_object(serde_json::from_str(&request)?, "text request")?;
        return Ok(
            json!({"profile":nx_graphics::PROFILE,"request":request,"metrics":fonts.shape(&request)?,"fonts":fonts.identities(),"renderer":"skia-0.153.3-cpu-aa-unhinted-linear-subpixel-no-baseline-snap"}),
        );
    }
    if let Some(manifest) = arg(&args, "--inspect") {
        let v: Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
        let scene: nx_graphics::Scene = serde_json::from_value(v["scene"].clone())?;
        return Ok(serde_json::to_value(scene.inspect(
            &arg(&args, "--revision").ok_or("--revision required")?,
            &arg(&args, "--id").ok_or("--id required")?,
        )?)?);
    }
    let source=arg(&args,"--entry").ok_or("usage: --entry scene.nx --output directory [--width 640 --height 480 --scale 1 --root root --fonts fonts.json]; or --measure-text JSON; or --inspect manifest.json --revision REV --id ID")?;
    let output = PathBuf::from(arg(&args, "--output").ok_or("--output directory required")?);
    let source_path = std::fs::canonicalize(&source)?;
    let workspace_root = arg(&args, "--source-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| source_path.parent().unwrap_or(Path::new(".")).to_path_buf());
    let workspace_root = std::fs::canonicalize(workspace_root)?;
    let entry_identity = source_path
        .strip_prefix(&workspace_root)?
        .to_string_lossy()
        .replace('\\', "/");
    let workspace = nx_api::NxWorkspace::from_directory(&workspace_root)?;
    let artifact = nx_api::build_workspace_program_artifact(
        &workspace,
        &entry_identity,
        &nx_api::ProgramBuildContext::empty(),
    )
    .map_err(|d| std::io::Error::other(serde_json::to_string(&d).unwrap_or_default()))?;
    program_revision.set(Some(artifact.resolved_program.fingerprint));
    let images = nx_codegen::emit_nx_ir(
        &artifact,
        &nx_codegen::NxIrEmitOptions {
            modules: Some(vec![]),
            debug: true,
        },
    )?;
    let modules: BTreeMap<String, PreparedModule> = images
        .into_iter()
        .map(|image| Ok((image.identity, PreparedModule::prepare(image.bytes)?)))
        .collect::<std::result::Result<_, nx_ir_runtime::NxIrRuntimeError>>()?;
    let entry = modules
        .get(&artifact.entry_identity)
        .ok_or("entry image missing")?;
    let program = Program::link(
        entry,
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )?;
    let origins = Arc::new(Origins::new());
    let value = program.evaluate_function(
        &arg(&args, "--root").unwrap_or("root".into()),
        &[],
        &RuntimeOptions {
            origins: Some(origins.clone()),
            ..RuntimeOptions::default()
        },
    )?;
    let version = nx_api::standard_library_entry("@nx/graphics")
        .ok_or("graphics library unavailable")?
        .version();
    let mut resolver = Resolver {
        identities: BTreeMap::new(),
        sources: BTreeMap::new(),
    };
    // Origins identify the authored constructing element; resolve its tag through the compiled
    // program's declaration table, never through the host value's display name.
    let declarations = artifact.construction_declarations();
    for origin in origins.entries() {
        if let Some(identity) = declarations.get(&(
            origin.source.identity.clone(),
            origin.source.start as usize,
            origin.source.end as usize,
        )) {
            resolver.identities.insert(
                origin.path.clone(),
                Identity {
                    module: identity.module.clone(),
                    version: version.clone(),
                    declaration: identity.declaration.clone(),
                },
            );
        }
        resolver.sources.insert(origin.path,json!({"module":origin.source.identity,"start":origin.source.start,"end":origin.source.end}));
    }
    let mut config = Config {
        graphics_version: version,
        renderer: "skia-0.153.3-cpu-aa-unhinted-linear-subpixel-no-baseline-snap".into(),
        program_revision: format!("{:016x}", artifact.resolved_program.fingerprint),
        fonts: fonts.identities(),
        ..Config::default()
    };
    for (name, dest) in [
        ("--width", &mut config.width),
        ("--height", &mut config.height),
        ("--scale", &mut config.scale),
    ] {
        if let Some(value) = arg(&args, name) {
            *dest = value.parse()?;
        }
    }
    if let Some(path) = arg(&args, "--limits") {
        config.limits = named_object(serde_json::from_slice(&std::fs::read(path)?)?, "limits")?;
    }
    let scene = nx_graphics::prepare_value(&value, &resolver, &mut fonts, config)?;
    let png = render_png(&scene, &fonts)?;
    std::fs::create_dir_all(&output)?;
    let output = std::fs::canonicalize(output)?;
    let revision_dir = output.join(&scene.revision);
    let stage = output.join(format!(".stage-{}", std::process::id()));
    if stage.exists() {
        std::fs::remove_dir_all(&stage)?;
    }
    std::fs::create_dir(&stage)?;
    let manifest = json!({"revision":scene.revision,"profile":scene.profile,"renderer":"skia-0.153.3-cpu-aa-unhinted-linear-subpixel-no-baseline-snap","image":"image.png","diagnostics":[],"scene":scene});
    let publish = (|| -> std::io::Result<()> {
        std::fs::write(stage.join("image.png"), png)?;
        std::fs::write(
            stage.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        if revision_dir.exists() {
            std::fs::remove_dir_all(&stage)?;
        } else {
            std::fs::rename(&stage, &revision_dir)?;
        }
        let pointer = output.join(format!(".current-{}.json", std::process::id()));
        std::fs::write(
            &pointer,
            serde_json::to_vec(
                &json!({"revision":scene.revision,"manifest":Path::new(&scene.revision).join("manifest.json")}),
            )?,
        )?;
        std::fs::rename(pointer, output.join("current.json"))?;
        Ok(())
    })();
    if publish.is_err() {
        let _ = std::fs::remove_dir_all(stage);
    }
    publish?;
    Ok(
        json!({"revision":scene.revision,"manifest":revision_dir.join("manifest.json"),"image":revision_dir.join("image.png")}),
    )
}
fn main() {
    let program_revision = std::cell::Cell::new(None);
    match run(&program_revision) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            use sha2::{Digest, Sha256};
            let raw = std::env::args().skip(1).collect::<Vec<_>>();
            let args = options(raw.clone()).unwrap_or(raw);
            let mut hash = Sha256::new();
            let normalized = options(args.clone()).unwrap_or_else(|_| args.clone());
            let mut normalized: BTreeMap<String, String> = normalized
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[k, v]| (k.clone(), v.clone()))
                .collect();
            for (key, value) in [
                ("--width", "640"),
                ("--height", "480"),
                ("--scale", "1"),
                ("--root", "root"),
            ] {
                normalized.entry(key.into()).or_insert_with(|| value.into());
            }
            for (key, raw_value) in normalized {
                if [
                    "--output",
                    "--entry",
                    "--source-root",
                    "--fonts",
                    "--limits",
                ]
                .contains(&key.as_str())
                {
                    continue;
                }
                hash.update(key.as_bytes());
                let value = raw_value
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .map(|n| n.to_string())
                    .unwrap_or(raw_value);
                hash.update(value.as_bytes());
                hash.update([0]);
            }
            if let Some(fingerprint) = program_revision.get() {
                hash.update(fingerprint.to_le_bytes());
            }
            for library in nx_api::standard_libraries() {
                hash.update(library.root.as_bytes());
                hash.update(library.version().as_bytes());
            }
            if let Some(entry) = arg(&args, "--entry") {
                let entry = std::fs::canonicalize(&entry).unwrap_or(PathBuf::from(entry));
                let source_root = arg(&args, "--source-root")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        Path::new(&entry)
                            .parent()
                            .unwrap_or(Path::new("."))
                            .to_path_buf()
                    });
                let source_root = std::fs::canonicalize(&source_root).unwrap_or(source_root);
                hash.update(
                    entry
                        .strip_prefix(&source_root)
                        .unwrap_or(&entry)
                        .to_string_lossy()
                        .as_bytes(),
                );
                hash_sources(&source_root, &source_root, &mut hash);
            }
            hash.update(b"skia-0.153.3-cpu-aa-unhinted-linear-subpixel-no-baseline-snap");
            if arg(&args, "--fonts").is_none() {
                hash.update(include_bytes!("../../fixtures/fonts/DejaVuSans.ttf"));
            }
            for option in ["--fonts", "--limits"] {
                if let Some(path) = arg(&args, option) {
                    if let Ok(bytes) = std::fs::read(&path) {
                        hash.update(&bytes);
                        if option == "--fonts" {
                            if let Ok(Value::Array(faces)) = serde_json::from_slice::<Value>(&bytes)
                            {
                                for face in faces {
                                    if let Some(file) = face["path"].as_str() {
                                        if let Ok(bytes) = std::fs::read(
                                            Path::new(&path)
                                                .parent()
                                                .unwrap_or(Path::new("."))
                                                .join(file),
                                        ) {
                                            hash.update(bytes);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let diagnostics = if let Some(runtime) =
                error.downcast_ref::<nx_ir_runtime::NxIrRuntimeError>()
            {
                Value::Array(runtime.diagnostics.iter().map(|d|json!({"code":d.code,"message":d.message,"declaration":d.declaration,"source":d.source.as_ref().map(|s|json!({"module":s.identity,"start":s.start,"end":s.end}))})).collect())
            } else if let Some(graphics) = error.downcast_ref::<nx_graphics::Diagnostic>() {
                json!([graphics])
            } else {
                serde_json::from_str::<Value>(&error.to_string())
                    .ok()
                    .filter(Value::is_array)
                    .unwrap_or_else(
                        || json!([{"code":"preview-input","message":error.to_string()}]),
                    )
            };
            println!(
                "{}",
                json!({"status":"error","requestRevision":format!("{:x}",hash.finalize()),"diagnostics":diagnostics})
            );
            std::process::exit(1)
        }
    }
}

fn hash_sources(root: &Path, path: &Path, hash: &mut sha2::Sha256) {
    use sha2::Digest;
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut entries = entries.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                hash_sources(root, &path, hash);
            } else if path.extension().is_some_and(|e| e == "nx") {
                hash.update(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .as_bytes(),
                );
                hash.update([0]);
                if let Ok(bytes) = std::fs::read(path) {
                    hash.update(bytes);
                    hash.update([0]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn options_are_strict_and_support_inline_values() {
        let parse = |args: &[&str]| options(args.iter().map(|s| s.to_string()));
        for args in [
            &["--widht", "100"][..],
            &["--width"][..],
            &["--width", "--height", "10"][..],
            &["--width", "1", "--width=2"][..],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
        assert!(named_object::<nx_graphics::Limits>(json!([]), "limits").is_err());
        assert!(named_object::<nx_graphics::Limits>(json!([1]), "limits").is_err());
        assert!(named_object::<TextRequest>(
            json!(["a", "Fixture", 18, 400, false]),
            "text request"
        )
        .is_err());
        assert!(
            named_object::<FontFace>(json!(["Fixture", 400, false, "font.ttf"]), "font face")
                .is_err()
        );
        assert_eq!(
            parse(&["--width=50", "--scale", "2"]).unwrap(),
            vec!["--scale", "2", "--width", "50"]
        );
        assert!(serde_json::from_value::<Vec<FontFace>>(
            json!([{"family":"Fixture","path":"font.ttf","weight":"700"}])
        )
        .is_err());
        assert!(serde_json::from_value::<Vec<FontFace>>(
            json!([{"family":"Fixture","path":"font.ttf","italic":"true"}])
        )
        .is_err());
    }
}
