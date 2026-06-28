## Context

The Node SDK can load real NX built-in libraries through `NxLibraryRegistry.loadFromDirectory`,
validate representative ReachMe chat-link sources, and evaluate them to JSON. The same
`NxProgramArtifact.generateNxIr()` path can still fail with `codegen-missing-semantic-data` for
nominal types such as `QuestionFlow` and `FlowStep` when those types originate from loaded library
artifacts.

The current Rust API model stores `LibraryArtifact` snapshots as analyzed module artifacts plus
export/interface tables. The shared IR builder later reconstructs a `PreparedModule` from the
resolved program import table to resolve nominal type references. That reconstruction is not
guaranteed to be equivalent to the prepared bindings used during validation and evaluation,
especially for library modules that reference declarations from peer or dependency libraries.

The file-format documentation already treats `programFingerprint` as a decimal string, but the Node
SDK TypeScript metadata currently exposes it as `number`. Any native `u64` value above JavaScript's
safe integer range can lose precision if represented as a number.

## Goals / Non-Goals

**Goals:**

- Make IR generation use semantic bindings equivalent to those used by validation and evaluation
  for source-provider modules and directory-loaded library modules.
- Preserve module-qualified nominal references when loaded libraries reference types from other
  loaded libraries.
- Add Node SDK regression coverage that reproduces a directory-loaded cross-library type graph.
- Align Node SDK IR metadata with lossless `programFingerprint` handling.

**Non-Goals:**

- Change NX source import syntax, local library path resolution, or library visibility rules.
- Add new IR expression features or expand the TypeScript IR runtime feature set.
- Implement ReachMe runtime bundle generation inside this repository.
- Preserve a backwards-compatible Node SDK `number` fingerprint API.

## Decisions

### Decision: IR generation must consume preserved semantic binding data

The implementation should make `build_codegen_program` obtain type bindings from the analyzed
semantic state associated with each module, not from a partial reconstruction that only sees the
resolved program import table. Two acceptable approaches are:

- Extend `ModuleArtifact` or a nearby codegen-facing snapshot with the prepared binding data needed
  for nominal type lookup.
- Persist a compact, deterministic binding map for each analyzed module and reconstruct a
  `PreparedModule` from that complete map during IR generation.

Rationale: validation, JSON evaluation, and IR generation should agree on the same analyzed
program. If the IR builder recreates bindings, the recreation must be as complete as the original
prepared module for source modules, same-library peer modules, direct library imports, and
transitive library dependencies.

Alternatives considered:

- Keep the current reconstruction and special-case missing names during codegen. Rejected because it
  would continue to make IR generation depend on visible names rather than preserving the resolved
  semantic target.
- Treat all `codegen-missing-semantic-data` from loaded libraries as unsupported IR. Rejected
  because representative inputs have already passed validation and evaluation, so the missing data
  is an NX artifact handoff bug rather than an unsupported language construct.

### Decision: library artifacts retain transitive semantic targets

`LibraryArtifact` should retain enough information for declarations in one loaded library to
reference declarations in another loaded dependency when later selected into a `ProgramArtifact`.
For example, if `chat-link/ChatLinkConfig.nx` exposes a type containing `QuestionFlow`, and
`question-flow/QuestionFlow.nx` contains `FlowStep`, the eventual IR should resolve both as
module-qualified nominal references.

Rationale: `LibraryRegistry` already owns the dependency closure. The selected `ProgramArtifact`
should carry a consistent semantic view of that closure into the resolved program and codegen
pipeline.

Alternatives considered:

- Re-analyze every loaded dependency from disk during IR generation. Rejected because loaded
  library snapshots are intended to be reusable artifacts and because codegen should not depend on
  current filesystem contents after the artifact is built.

### Decision: expose program fingerprints as decimal strings in Node metadata

The Node SDK `NxIrMetadata.programFingerprint` should be typed and returned as a string matching the
IR JSON document. Tests should compare fingerprints as strings and should not parse them through
JavaScript `Number`.

Rationale: the native fingerprint is a `u64`; JavaScript numbers cannot represent all `u64` values
exactly. A string keeps cache-key and equality comparisons stable across Rust, .NET, TypeScript, and
JSON boundaries.

Alternatives considered:

- Expose `bigint` in Node metadata. Rejected for now because JSON artifacts and existing TypeScript
  runtime metadata already use strings, and string values are easier to persist and compare across
  package boundaries.

### Decision: unbound `Element` remains an object-shaped compiler boundary type

Codegen should resolve declared `Element` bindings before applying the compiler-level `Element`
fallback. When no source or library declaration named `Element` is visible, the fallback represents
the element supertype as an object-shaped IR type so existing component and intrinsic element
contracts can continue to use `Element` annotations without requiring a source declaration.

Alternatives considered:

- Always treat `Element` as the object-shaped fallback before binding lookup. Rejected because a
  user or library can declare a real type named `Element`, and that declaration must retain nominal
  identity in IR.

## Risks / Trade-offs

- [Preserving prepared binding data can increase artifact memory usage] -> Mitigation: persist only
  the binding target metadata required for downstream nominal type and reference resolution.
- [Changing `programFingerprint` from `number` to `string` affects Node SDK consumers] -> Mitigation:
  update TypeScript declarations, README examples, and tests together; this SDK is still
  source-consumed and not constrained by backwards compatibility.
- [Directory-loaded fixture tests can become path-sensitive] -> Mitigation: create temporary
  library directories inside tests and assert module-qualified references by logical names or
  normalized endings rather than absolute temp paths.
- [Reconstructed binding maps could diverge from analysis again] -> Mitigation: add tests that run
  validation, JSON evaluation, and IR generation on the same artifact and assert that all three
  succeed for cross-library nominal references.

## Migration Plan

1. Add a failing Rust or Node-level regression test for a directory-loaded library graph with
   cross-library type references.
2. Preserve or reconstruct complete semantic binding data for analyzed modules selected into
   `ProgramArtifact`.
3. Update codegen type-reference resolution to use the preserved semantic binding data.
4. Change Node SDK IR metadata declarations and parsing expectations so `programFingerprint` is a
   string.
5. Add or update SDK tests and documentation for directory-loaded libraries and fingerprint
   metadata.
6. Run the relevant Rust codegen/API tests plus Node SDK typecheck and Vitest coverage.

Rollback is source-level: revert the semantic-binding handoff and Node metadata type changes if the
regression test exposes wider artifact model issues. There is no runtime data migration.

## Open Questions

- Should the preserved semantic binding data live directly on `ModuleArtifact`, or should codegen
  define a smaller serializable binding snapshot owned by `ProgramArtifact`?
- Do we want a Rust API regression test in addition to the Node SDK test so the native handoff is
  covered before JavaScript wrappers are involved?
