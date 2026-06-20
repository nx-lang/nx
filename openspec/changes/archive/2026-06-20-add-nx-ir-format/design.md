## Context

NX currently has several internal executable representations: `ProgramArtifact` preserves analyzed
source and libraries, `ResolvedProgram` provides module-qualified runtime lookup for the native
interpreter, and `CodegenProgram` provides a target-neutral model used by JavaScript/TypeScript
emitters. Hosts can also cache generated JavaScript program modules, but generated source is not a
stable semantic interchange format and is awkward to inspect or transform.

The requested IR should be human-readable enough for debugging, deterministic enough for caching,
and efficient after loading in a JavaScript/TypeScript environment. It also needs to fit the
existing artifact-first architecture so imported libraries, workspace entry identities, diagnostics,
and program fingerprints remain coherent.

## Goals / Non-Goals

**Goals:**

- Define a versioned NX IR JSON program format emitted from successful `ProgramArtifact` inputs.
- Preserve resolved, module-qualified executable semantics instead of raw syntax or unresolved HIR.
- Provide a TypeScript runtime that loads IR, prepares it once, then evaluates functions and
  components efficiently.
- Support non-reactive component descriptor construction, initialization, explicit state evaluation,
  and validated host-owned state updates.
- Include eager expression support needed by current component authoring, including match-style
  `if is` forms and state-dependent component content.
- Keep the IR ready for future reactivity by assigning stable expression, declaration, prop, and
  state slot identities.
- Validate behavior through parity tests against the native interpreter and, where relevant,
  existing generated JavaScript output.

**Non-Goals:**

- Implement reactive dependency tracking, subscriptions, invalidation, or signal APIs in this
  change.
- Add NX-owned reducer/state-transition syntax beyond validating and normalizing host-owned state
  values.
- Support action-handler serialization or invocation in the TypeScript IR runtime unless existing
  non-reactive semantics can represent the construct safely.
- Replace generated JavaScript/TypeScript source output or the native interpreter.
- Define the binary IR encoding in this change; JSON is the v1 wire format, with a binary mirror
  left for later if parse size becomes a bottleneck.

## Decisions

### 1. Define a stable semantic IR above HIR and beside codegen

The IR should be produced from `ProgramArtifact` after whole-program analysis, import resolution,
type analysis, and runtime module ID assignment. It should borrow the semantic shape of
`CodegenProgram`, including declarations, expression kinds, component metadata, source spans, and
module-qualified references, but it should not serialize Rust HIR internals or emitter-specific
generated names.

Serializing raw HIR was rejected because it would expose compiler implementation details and force
the TypeScript runtime to redo semantic resolution. Treating generated JavaScript as the IR was
rejected because source text is optimized for host execution, not stable schema evolution,
inspection, validation, or alternate runtimes.

### 2. Use deterministic JSON for v1

The first IR artifact should be UTF-8 JSON with deterministic ordering for modules, declarations,
fields, properties, entrypoints, and expression tables. JSON makes artifacts easy to inspect,
golden-test, store in ordinary caches, and load in TypeScript without new dependencies.

MessagePack or CBOR can be added later as a byte-for-byte equivalent binary encoding if JSON parse
time or artifact size becomes a measured problem. The v1 data model should avoid JSON-only
semantics so a future binary representation can share the same schema.

### 3. Make runtime loading a two-phase operation

The TypeScript runtime should expose a load/prepare phase that validates the IR version and runtime
ABI, resolves references, interns repeated strings, builds declaration and expression lookup
tables, compiles expression trees or expression tables into slot-based evaluator functions, and
precomputes schema validators/default evaluators. Hot-path evaluation should then avoid repeated
schema walking and string-keyed runtime lookup wherever possible.

This balances a human-readable on-disk format with the performance goal once the IR is loaded.

### 4. Keep component state host-owned in v1

The runtime should support component state by exposing pure operations:

- initialize a component from props and state defaults;
- validate and normalize a complete state object;
- apply a host-provided state patch to produce a normalized next state; and
- evaluate a component body with normalized props and current state.

This satisfies host-driven state updates without inventing hidden mutable component instances or
new NX reducer semantics. Future NX-managed state transitions can be added as explicit IR
declarations and runtime operations.

### 5. Preserve atomic component descriptor semantics

Constructing a component expression in IR evaluation should produce a descriptor payload with
normalized props and content. It must not evaluate the referenced component body unless the host
selects that component for initialization or evaluation. This matches existing generated codegen
semantics and keeps parent/child component composition predictable for both native and TypeScript
runtimes.

### 6. Include match and conditional content in the IR v1 expression set

The current JavaScript codegen subset rejects match-style expressions, but stateful component value
evaluation needs conditional content that can inspect component state. The IR should include these
non-reactive expression forms in v1 and the TypeScript runtime should execute them eagerly. This
keeps the IR useful for the motivating component workflows instead of merely mirroring current
codegen limitations.

### 7. Treat unsupported constructs as explicit diagnostics

The IR builder must reject unsupported executable constructs with diagnostics rather than emitting
partial IR. The TypeScript runtime should also reject an IR document that advertises unsupported
features or contains unknown operation tags for the declared version.

## Risks / Trade-offs

- [JSON artifacts may be larger than generated JavaScript or binary IR] -> Keep JSON deterministic
  and schema-driven in v1, measure load costs, and add a binary mirror only if evidence supports it.
- [IR and native interpreter semantics may drift] -> Add parity tests for every supported
  expression and component scenario, and treat interpreter behavior as the semantic oracle.
- [The format may calcify too early] -> Version the IR document and runtime ABI independently,
  require feature flags, and reject unknown required features.
- [JavaScript number precision can corrupt NX integer semantics] -> Encode width-sensitive numeric
  literals and values explicitly where they cannot safely round-trip as JavaScript numbers.
- [State patches could be confused with NX-owned state transitions] -> Name the API around
  host-owned state normalization/patching and keep it pure; do not expose reducer/action semantics
  in v1.
- [Runtime preparation adds complexity] -> Keep the disk schema simple and isolate performance
  optimizations inside prepared runtime structures that do not affect artifact compatibility.

## Migration Plan

1. Add the IR model, serializer, and builder from `ProgramArtifact` without changing existing
   interpreter or codegen behavior.
2. Add Rust and CLI APIs for writing `.nxir.json` artifacts and golden tests for deterministic
   output.
3. Add the TypeScript runtime loader/preparer with function evaluation parity tests.
4. Add component descriptor, initialization, state update, and explicit-state evaluation support to
   the TypeScript runtime with parity tests.
5. Add native/.NET binding API exposure for artifact-first IR emission.
6. Keep existing JavaScript/TypeScript source generation as the default output. Rollback is limited
   to removing the new IR APIs and CLI format because existing runtime paths remain unchanged.

## Open Questions

- The exact package/file placement for the TypeScript IR runtime can be chosen during
  implementation to match repository conventions.
- Binary IR support is intentionally deferred; the JSON schema should still be designed so a binary
  representation can reuse it later.
