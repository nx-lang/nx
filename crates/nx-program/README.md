# nx-program

Shared foundation for NX tooling: workspaces and source graphs, library registry and embedded
standard libraries, analyzed `ProgramArtifact` construction, resolved-program types, tooling
diagnostics, schema queries and visualizer inspection.

Code generation, the language service and the HIR interpreter depend on this crate. It depends
on syntax/HIR/types/diagnostics/value foundations and has no dependency on those services, either
execution engine or `nx-api`. The resolved types have a single definition here; the interpreter
does not re-export them.

```rust
use nx_program::{build_program_artifact_from_source, ProgramBuildContext};

let artifact = build_program_artifact_from_source(
    "let root(): int = { 42 }", "main.nx", &ProgramBuildContext::empty(),
).unwrap();
assert_eq!(artifact.entry_identity, "main.nx");
assert_eq!(artifact.source_text("main.nx"), Some("let root(): int = { 42 }"));
```

`source_text` and `source_entries` provide read-only access to preserved source text and versions.
Artifact storage remains private. Source bytes, library identities, fingerprints and diagnostic
mapping are preserved by the extraction. Pure foundation tests live here; tests that execute or
emit a program live in `nx-api/tests/` to avoid upward dependencies.
