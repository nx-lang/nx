# nx-api

External Rust entry point for NX tooling. Program construction and inspection are re-exported
flat from `nx-program`. Program-aware value formatting remains independent of execution. The `codegen`, `language_service` and `ir_runtime` namespaces
re-export the service crates without wrappers or changed defaults.

A caller needs only `nx-api` as an NX dependency to build source, emit IR and execute it:

```rust
use nx_api::{build_program_artifact_from_source, codegen, ir_runtime, ProgramBuildContext};
use std::collections::BTreeMap;

let artifact = build_program_artifact_from_source(
    "let root(): int = { 6 * 7 }", "main.nx", &ProgramBuildContext::empty(),
).unwrap();
let images = codegen::emit_nx_ir(&artifact, &codegen::NxIrEmitOptions {
    modules: Some(Vec::new()), // Emit the entry and its libraries, including the prelude.
    ..Default::default()
}).unwrap();
let modules: BTreeMap<_, _> = images.into_iter().map(|image| {
    (image.identity, ir_runtime::PreparedModule::prepare(image.bytes).unwrap())
}).collect();
let program = ir_runtime::Program::link(
    &modules["main.nx"], |identity| modules.get(identity).cloned(),
    &ir_runtime::LinkOptions::default(),
).unwrap();
let value = program.evaluate_function("root", &[], &ir_runtime::RuntimeOptions::default()).unwrap();
assert_eq!(value.to_json_string().unwrap(), "42");
```

`tests/facade.rs` exercises this sequence against an explicit expected result. Runtime-only hosts
should depend on `nx-ir-runtime` directly to avoid compiling the compiler and other services.

`format_program_artifact_value` spells canonical host values as annotated NX text;
`format_program_artifact_function_value` formats a read-only IR function result while retaining
constant-case and function identities. Neither performs execution.

The flat `eval_*`, source/artifact component lifecycle, `ResourceLimits`, interpreter-value
conversion and `format_nx_text` APIs have been removed. See
[the migration guide](../../docs/hir-interpreter-retirement.md) for exact removals and limitations.
