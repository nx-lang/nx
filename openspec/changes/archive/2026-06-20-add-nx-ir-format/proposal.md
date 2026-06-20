## Why

NX can evaluate programs through the native interpreter and can emit executable JavaScript, but it
does not yet have a stable, portable intermediate representation that can be cached, inspected, and
loaded by a JavaScript/TypeScript runtime. A human-readable NX IR gives hosts a durable artifact
between source analysis and execution while keeping hot-path performance in the runtime's prepared
in-memory representation.

## What Changes

- Add a versioned NX IR program format, initially as deterministic JSON, emitted from successful
  `ProgramArtifact` inputs.
- Preserve resolved modules, module-qualified references, public function/component entrypoints,
  declaration metadata, executable expressions, type/schema metadata, source-map metadata, and
  program fingerprint/runtime ABI information in the IR.
- Add a TypeScript runtime package/module that can load NX IR JSON, prepare it once into efficient
  in-memory evaluators, and evaluate supported NX functions and components.
- Support non-reactive component descriptor construction, component initialization, explicit state
  component evaluation, and validated host-owned component state updates through the TypeScript IR
  runtime.
- Include IR/runtime coverage for eager expressions needed by current NX component authoring,
  including conditionals, `if is`/match forms, loops, records, unions, enums, arrays, elements, and
  component descriptors.
- Add tests comparing IR runtime behavior against existing interpreter and generated JavaScript
  behavior for supported non-reactive programs and components.
- Expose public Rust/native and managed .NET APIs for emitting NX IR from a `ProgramArtifact`.
- Add CLI support for writing the NX IR JSON artifact from the existing program analysis pipeline.

## Capabilities

### New Capabilities

- `nx-ir-format`: Defines the versioned NX IR JSON program artifact, resolved reference model,
  supported expression/declaration schema, deterministic encoding rules, and emission contract from
  `ProgramArtifact`.
- `typescript-ir-runtime`: Defines the TypeScript/JavaScript runtime behavior for loading,
  preparing, validating, and evaluating NX IR programs.

### Modified Capabilities

- `executable-code-generation`: Adds NX IR emission as a public artifact-first output alongside
  executable JavaScript/TypeScript generation, with parity expectations against interpreter
  semantics.
- `cli-code-generation`: Adds CLI support for writing NX IR JSON from a source/workspace program
  input.
- `dotnet-binding`: Adds managed APIs that emit NX IR from `NxProgramArtifact` instances and
  surface diagnostics through the existing managed exception path.

## Impact

- Adds a new Rust crate or module for the stable IR model, JSON serialization, and
  `ProgramArtifact`-to-IR lowering.
- Adds TypeScript runtime source and tests for IR loading/preparation/evaluation.
- Extends codegen/native/.NET binding surfaces with artifact-first IR emission APIs.
- Extends `nxlang codegen` or an equivalent CLI entry point with an explicit IR output option.
- Adds cross-runtime parity tests that execute IR through the TypeScript runtime and compare output
  with the interpreter for supported programs.
