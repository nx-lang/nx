//! The runtime builds without the compiler: it and the two crates it depends on name no other
//! crate of this workspace as a dependency.

use std::path::Path;

/// The names under `[dependencies]` in a crate's manifest.
fn dependencies(krate: &str) -> Vec<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(krate)
        .join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("{}: {error}", manifest.display()));
    text.lines()
        .skip_while(|line| line.trim() != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter_map(|line| line.split(['=', '.', ' ']).next())
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_runtime_depends_on_no_compiler_crate() {
    assert_eq!(
        dependencies("nx-ir-runtime"),
        ["nx-ir", "nx-value", "serde"]
    );
    assert_eq!(dependencies("nx-ir"), [] as [&str; 0]);
    let workspace: Vec<String> = dependencies("nx-value")
        .into_iter()
        .filter(|name| name.starts_with("nx-"))
        .collect();
    assert_eq!(workspace, [] as [&str; 0]);
}
