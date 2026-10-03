## Why

A host that exposes an NX function to a language model as a tool needs three things for it: a JSON
Schema for its arguments, a JSON Schema for its result, and the descriptions the author wrote. NX
has no supported way to give any of them. `@nx-lang/ir-runtime` exports `PreparedDeclaration`, but
its README says the prepared types are the runtime's, not an API, and the image they are read from
cannot answer the question anyway: it erases generic type arguments to `object`, records nothing for
a type alias, carries a function's result type only when the source declares one, and holds no doc
comments, because `doc-comments` forbids documentation from changing generated IR. Doc comments
themselves landed after 0.5.0 and have not been released, so no host can read them at all.

ReachMe's agent work (see `add-agent-host-package` in this repository and `add-agent-function-tools`
in ReachMe) needs this now. It compiles a tenant's NX in Node with `@nx-lang/sdk-wasm`, stores the
tool definitions it derives, and later runs the functions in a Cloudflare Durable Object that has
only `@nx-lang/ir-runtime`. Without an export from NX, ReachMe would have to rebuild the NX type
mapping from undocumented fields and could not get descriptions.

## What Changes

- Add a schema export to the compiler SDKs. From a built program artifact a host asks for the schema
  of a function, named by module identity and name (the same pair a `Function` record carries), and
  gets: a JSON Schema (draft 2020-12) object for its arguments keyed by parameter name, a JSON
  Schema for its result, the function's documentation, and one entry per parameter with its name,
  NX type, whether it is required, and its documentation. The answer also gives the source
  location of the function's declaration, so a host can point its own diagnostics at it.
- The same export answers for a declared type: a record (abstract ones and actions included), a
  union, a type alias, an update record or a property union.
- Define the mapping from every NX type form to JSON Schema: primitives, the `?`, `+` and `*`
  occurrences, records with required, optional and defaulted fields, constant unions, payload and
  mixed unions, abstract records and the concrete records that extend them, generic records and
  their applied types, type aliases, update records, property unions and `object`.
- The schema describes the canonical JSON value encoding, and agrees with what the runtimes accept
  at the host boundary: every value that is valid against an input schema is accepted by boundary
  validation, and every value an entry call returns is valid against the output schema. A schema
  for input asks for a `$type` discriminator only where the runtime needs one to choose a shape. A
  schema for output always carries it.
- `///` documentation becomes the `description` of the schema it documents: a function, a
  parameter, a record, a field, a union and a payload case. Doc links are written as their label in
  a code span, as `typegen` already writes them.
- A type with no JSON form is reported as a diagnostic, never as a wrong schema. That covers
  function types, the function reference type that `add-function-reference-type` adds, component
  types, an abstract record nothing concrete extends, and two shapes at one site that share a
  `$type`.
- A host can name types whose parameters it supplies itself (the agent library's `ToolContext` is
  the case that motivates it). Such a parameter is left out of the argument schema and reported as
  host-supplied.
- The export ships in `@nx-lang/sdk-wasm` and `@nx-lang/sdk-node`, over one implementation in
  `nx-api`. The wasm module's ABI version goes up by one.
- Release 0.6.0, the first after 0.5.0, carries doc comments and this export together, which is how
  doc comments reach hosts. Its release notes list both. The `@nx-lang/ir-runtime` README points hosts at the export in place of
  reading prepared declarations.

Nothing changes in the NX IR format, in either IR runtime, or in what a program means. The entry
module's image lists more modules in its module table: every module that declares a subtype of an
abstract record a function takes or returns, so a runtime that links from the entry accepts every
subtype a schema lists, even one declared in a module nothing references.

## Capabilities

### New Capabilities

- `declaration-schema-export`: what a host can ask a built program for (a function's schemas, a
  type's schema), the mapping from each NX type form to JSON Schema, how documentation becomes
  descriptions, the diagnostics for types with no JSON form, and the rule that the schemas agree
  with the canonical value encoding and boundary validation.

### Modified Capabilities

- `sdk-wasm`: a program artifact exports the schema of a function and of a type; the ABI version
  moves with the new exports.
- `nx-ir-format`: the entry module's artifact also lists the modules that declare a subtype of an
  abstract record a function of the program takes or returns.
- `sdk-node`: a program artifact exports the same schemas, equal to the wasm SDK's for the same
  program.

## Impact

- `crates/nx-api`: a new schema module over `ProgramArtifact`, reading each module's lowered
  declarations, documentation, inferred types and prepared interfaces. New public functions and
  serializable result types.
- `crates/nx-hir`, `crates/nx-cli`: the doc-to-Markdown rendering `typegen` uses
  (`typegen_doc` in `crates/nx-cli/src/typegen/model.rs`) moves to where both callers reach it.
- `bindings/wasm`: two native exports, ABI version bump, `functionSchema` and `typeSchema` on
  `NxProgramArtifact`, new types, README.
- `bindings/node`: the same two methods and types, README, parity tests.
- `src/vscode/CHANGELOG.md`: the 0.6.0 entries for doc comments and for this export.
- `runtime/typescript`: README only. A test that runs schema-valid arguments through `callFunction`
  and validates results lives with the wasm SDK's tests, which already depend on the runtime.
- Website: a reference page for the mapping.
- New dev dependency: a JSON Schema draft 2020-12 validator for the JavaScript tests.
- `crates/nx-codegen`: the entry image's module table lists the modules that declare a function's
  boundary subtypes; `docs/nx-ir-format.md` says so.
- Not affected: the NX IR format, `@nx-lang/ir-runtime` code, the Rust IR runtime, the .NET binding,
  the C FFI, the CLI.

**Dependencies on the other changes in this set**

- `add-function-reference-type` (N1): landed. This change reports the type it added,
  `<function ... />: R`, as not expressible.
- `add-ir-runtime-evaluation-budget` (N3): none.
- `add-agent-library` (N4): landed. No code dependency; the tests import the real `@nx/agent`. Its
  abstract `ToolContext`, in module `@nx/agent/agent.nx`, is the type hosts are expected to name as
  host-supplied, and its `FunctionTool` and `HttpTool` hold function reference fields, so a site
  typed by its `Tool` or `Agent` has no JSON form.
- `add-agent-host-package` (N5): depends on this change. It calls the export when a host compiles
  an agent and carries the result into its normalized tool definitions.
- ReachMe `add-agent-function-tools` (R2) and `add-agent-http-tools` (R3) depend on this change
  through N5. `add-agent-tool-loop` (R1) and `add-question-flow-agent-steps` (R4) do not.
