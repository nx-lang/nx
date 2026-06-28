## Why

Directory-loaded libraries can validate and evaluate successfully through the Node SDK while later
failing NX IR generation with `codegen-missing-semantic-data` for nominal types imported from those
libraries. This blocks real ReachMe chat-link runtime bundle generation after
`NxLibraryRegistry.loadFromDirectory` loads built-ins such as `QuestionFlow` and `FlowStep`.

## What Changes

- Preserve or reconstruct semantic bindings for declarations that originate in loaded library
  artifacts so IR generation has the same nominal type information used by validation and
  evaluation.
- Add SDK coverage for directory-loaded libraries whose exported types reference types from another
  loaded library.
- Ensure program fingerprints exposed through Node SDK IR metadata are safe for JavaScript
  consumers and do not rely on lossy `number` precision.
- Keep existing validation, evaluation, and deterministic IR contracts intact for in-memory
  workspaces and source-built artifacts.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `library-registry`: Loaded library snapshots must retain the semantic data needed by downstream
  program artifacts and IR generation, including transitive cross-library type references.
- `nx-ir-format`: IR generation must emit module-qualified nominal type metadata for declarations
  imported from loaded library artifacts and expose fingerprints in a lossless form.
- `sdk-node`: Node SDK IR metadata and tests must cover directory-loaded library graphs and expose
  program fingerprints without JavaScript numeric precision loss.

## Impact

- Affected Rust areas include library artifact snapshotting, program build context construction,
  resolved program or semantic binding handoff, and the shared IR emission pipeline.
- Affected Node areas include `NxLibraryRegistry.loadFromDirectory`, `NxProgramArtifact.generateNxIr`,
  generated TypeScript declarations, and SDK tests.
- Downstream ReachMe bundle generation should be able to treat `codegen-missing-semantic-data` for
  these built-in libraries as a temporary unsupported state until this NX-side fix lands.
