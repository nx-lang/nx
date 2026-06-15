## Context

NX currently has a strong semantic pipeline: source is parsed and lowered into HIR, analyzed into
per-file `ModuleArtifact`s, assembled into `ProgramArtifact`s, and executed by the interpreter
through `ResolvedProgram`. The CLI also has code generation for exported DTO/type surfaces, but
that path is intentionally declaration-oriented and does not emit executable NX behavior.

The desired JavaScript/TypeScript backend needs a different boundary. It must use resolved,
type-checked program semantics rather than raw HIR, while keeping the generated output readable
enough to debug and neutral enough to run on both browser and server hosts.

## Goals / Non-Goals

**Goals:**

- Add `crates/nx-codegen` as the home for executable code generation.
- Build a backend-facing `CodegenProgram` from `ProgramArtifact`.
- Preserve enough module, type, symbol, source map, and entrypoint information for deterministic
  TypeScript/JavaScript emission.
- Emit executable TypeScript and JavaScript for supported non-reactive NX functions, values,
  records, unions, enums, and element expressions that evaluate to serializable `NxValue` records.
- Keep generated output host-neutral so client and server use the same emitted program model.
- Validate generated output against the interpreter for supported scenarios.

**Non-Goals:**

- Implement functional reactivity, dependency tracking, invalidation, subscriptions, or reactive
  host adapters.
- Emit framework-specific React/Vue/Svelte/etc. components.
- Change DTO/type generation semantics beyond renaming the CLI surface from `nxlang generate` to
  `nxlang typegen`.
- Emit executable components, component lifecycle APIs, component state evaluation, dispatch, or
  action handlers.
- Add optimized bytecode, minified bundles, tree-shaking, or bundler-specific packaging.
- Guarantee support for every interpreter feature in the first pass; unsupported executable
  constructs should fail during codegen with diagnostics rather than emitting partial behavior.

## Decisions

### 1. Introduce `nx-codegen` with a target-neutral `CodegenProgram`

`nx-codegen` will own both the backend-facing model and the initial TS/JS emitters. The central
model should be named `CodegenProgram`, not `JsSemanticProgram`, because it represents resolved NX
semantics prepared for generation rather than JavaScript syntax.

Alternatives considered:

- Reuse raw HIR directly. Rejected because HIR does not carry whole-program symbol resolution,
  selected library snapshots, entrypoint identity, or all type information needed by generated
  imports and runtime references.
- Name the model after JavaScript. Rejected because the model should remain usable by future
  non-JS backends as long as it does not contain target-specific import, identifier, or helper-call
  decisions.

### 2. Use `ProgramArtifact` as the codegen input boundary

The builder will derive `CodegenProgram` from a successful `ProgramArtifact`, using:

- `ProgramArtifact.resolved_program` for runtime module IDs, module-qualified item references,
  imported symbol tables, and entrypoint lookup.
- Preserved `ModuleArtifact.lowered_module` values for declarations and expressions.
- Preserved `ModuleArtifact.type_env` values for inferred expression and binding types.
- Artifact provenance, fingerprints, source text, and source spans for deterministic output,
  source maps, and diagnostics.

This keeps codegen aligned with the interpreter-visible executable world and avoids re-resolving
imports in the backend.

`ProgramArtifact` already preserves source text internally for diagnostics. This change should
expose that data through a small read-only `nx-api` surface, such as source lookup by module
identity and iteration over preserved source entries, so `nx-codegen` can build source maps without
re-parsing or re-reading files.

### 3. Split semantic modeling from target emission

`CodegenProgram` should contain NX semantic concepts such as modules, declarations, expressions,
resolved references, source spans, and runtime value categories. TypeScript/JavaScript emitters
should perform target-specific decisions in a later phase: identifier escaping, import specifiers,
file names, helper names, comments, and optional type annotations.

This keeps the first implementation simple while leaving room for later `CodegenProgram` consumers
or an optimized execution-plan lowering.

### 4. Emit plain records/enums directly and keep the helper surface small

Generated TypeScript/JavaScript should use ordinary JavaScript data construction for NX records and
enums where no helper semantics are needed. Record expressions should emit direct object literals
with `$type` followed by fields in emitter-controlled declaration/effective-field order. Enum
expressions should emit direct string constants through the generated enum object. Generated
TypeScript should layer erased structural record types with per-property `readonly` modifiers over
record values, avoiding `Readonly<T>` wrappers, and derive enum types from the generated `as const`
enum value object with `typeof Enum[keyof typeof Enum]`.

The interpreter may continue to keep an internal tagged enum value so it can validate enum types,
coerce host-provided strings, and produce type-aware diagnostics. At the public `NxValue` boundary
and in generated JavaScript/TypeScript output, enum values should remain bare authored member
strings.

The runtime helper surface should cover only non-trivial shared behavior, such as element-like
record construction and safe runtime errors. Arrays should emit as ordinary JavaScript arrays with
TypeScript `readonly T[]` types where type syntax is enabled. It should not include component
lifecycle, action handler, reactive cell, subscription APIs, or record/enum construction helpers in
this change.

The runtime module will initially live with the generated output support in `nx-codegen` and be
emitted as local generated support files. Packaging it as a standalone npm package can be a later
distribution change.

### 5. Use one emitter pipeline with optional TypeScript syntax

The TypeScript target should produce the canonical readable generated source with types where the
current type model makes that practical. Record declarations should emit exported structural types
whose top-level fields use the `readonly` keyword directly, function parameters and returns should
use mapped NX types when available, and enum declarations should emit ordinary `as const` value
objects plus exported types derived with
`typeof Enum[keyof typeof Enum]`. The JavaScript target should use the same emitter pipeline with
type-only syntax disabled, producing executable ESM suitable for Node, browser bundlers, and edge
runtimes.

If a future implementation chooses to transpile generated TypeScript into JavaScript internally,
that should remain an implementation detail behind the JavaScript generation API.

### 5a. Treat object property order as deterministic output, not value semantics

The emitter should choose a stable property write order for generated record object literals so
generated source remains deterministic. JavaScript object property order should not become part of
NX record equality or interpreter parity semantics. Tests that compare generated execution with the
interpreter should compare parsed values or otherwise ignore property ordering, while snapshot and
determinism tests may still assert stable generated source.

### 6. Make the first pass eager and non-reactive

The generated runtime behavior in this change should match ordinary interpreter evaluation for
supported constructs that produce serializable `NxValue` output: expressions evaluate when the
generated entrypoint is invoked, loops produce values eagerly, and element expressions are supported
only when they produce record-like `NxValue` data. There should be no reactive graph, no automatic
invalidation, no subscription behavior, and no generated component lifecycle/action-handler model.

This keeps the initial semantic contract small and creates a stable foundation for a later
reactivity pass that can add explicit reactive nodes or an execution-plan lowering.

### 7. Split executable and types-only CLI surfaces

The CLI should expose executable generation through a new `nxlang codegen` command. The existing
types-only generation surface should move from `nxlang generate` to `nxlang typegen` so the command
matches its purpose: it generates contracts for interacting with NX-authored code, but it does not
execute NX-authored behavior. There is no need to keep a compatibility alias.

This gives the CLI two clear lanes:

- `nxlang typegen`: type/DTO contracts only.
- `nxlang codegen`: executable TypeScript/JavaScript generated from NX program semantics.

Keeping executable output under `codegen` avoids overloading the types-only surface and lets the new
command grow runtime-specific options over time.

Alternatives considered:

- Add `--language javascript-runtime` or similar to `nxlang typegen`. Rejected because it would
  mix declaration generation and executable program generation under one command with different
  input, output, and runtime-helper expectations.
- Use `nxlang compile`. Rejected because this first pass emits readable generated source rather
  than a stronger compile artifact such as bytecode, optimized bundles, or framework-specific
  runtime packages.

### 8. Clarify the `nx-interpreter` dependency

`nx-codegen` will initially depend on `nx-interpreter` as a build dependency because
`ProgramArtifact.resolved_program` exposes `ResolvedProgram`, `RuntimeModuleId`, and
module-qualified item references from that crate. The dependency is for the shared resolved program
model, not for executing the interpreter during code generation. Parity tests may also use the
interpreter as a dev/test dependency.

A later cleanup can move the resolved-program model into a lower shared crate if the dependency
becomes too heavy or confusing.

### 9. Expose source access from `nx-api` for codegen

`nx-codegen` should not reach into `ProgramArtifact` internals or duplicate source loading. Instead,
`nx-api` should expose read-only access to the source text already preserved by `ProgramArtifact`.
The minimal useful surface is:

- lookup source text by normalized source-provider identity or artifact module file name;
- iterate preserved source entries for source-map construction; and
- preserve ownership with borrowed `&str` or cloned `Arc<str>` values rather than copying all
  source text unnecessarily.

This keeps source-map support in scope without making `source_map` fields public or coupling
codegen to the artifact's internal storage layout.

## Risks / Trade-offs

- [CodegenProgram drifts from interpreter semantics] -> Derive it from `ProgramArtifact` and add
  parity tests that compare emitted JS/TS execution with interpreter results.
- [The semantic model becomes too JS-specific] -> Keep JS import paths, helper calls, identifier
  escaping, and module-format choices in emitter/emit-plan types rather than in `CodegenProgram`.
- [Initial support feels incomplete without reactivity] -> Document non-reactive behavior in the
  spec and reject unsupported reactive/runtime constructs explicitly until a later change.
- [Generated runtime helpers become an accidental framework] -> Keep helpers host-neutral and
  model NX values only; framework adapters are out of scope.
- [Dynamic server-side generation needs a TS toolchain] -> Provide direct JavaScript emission as a
  public target so runtime hosts are not required to execute TypeScript.
- [Local helper files drift from generated program expectations] -> Emit helper files from the same
  `nx-codegen` version as the generated program and prefer overwriting/regenerating support files
  rather than treating them as hand-edited assets.

## Migration Plan

1. Add `nx-codegen` to the Rust workspace without changing existing interpreter or DTO generation
   behavior.
2. Implement `ProgramArtifact` to `CodegenProgram` construction for the supported non-reactive
   subset.
3. Add TypeScript and JavaScript emitters plus runtime helper output.
4. Expose read-only `ProgramArtifact` source access from `nx-api`.
5. Add CLI/API entry points for executable generation through `nxlang codegen` and rename the
   types-only command from `nxlang generate` to `nxlang typegen`.
6. Add parity and snapshot tests.

Rollback is limited to removing the new crate and CLI/API entry points. Existing parsing,
type-checking, interpreter, bindings, and DTO generation behavior should remain unchanged.

## Open Questions

- What component lifecycle/action-handler subset should `nxlang codegen` support when the reactive
  and stateful component design is ready?
