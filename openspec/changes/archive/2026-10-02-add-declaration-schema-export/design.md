## Context

See proposal.md for motivation and `specs/declaration-schema-export/spec.md` for the mapping. The
agreed product design is ReachMe's `docs/ReachMe-Cloudflare-AI-Agent-Architecture.md`, §5.5, §6.1
and §7.3; decision 7 in its §9 (NX owns the schema export) is taken as given.

What the code offers today:

- `@nx-lang/ir-runtime` exports `PreparedDeclaration`, `PreparedType`, `PreparedDeclaredParam` and
  `PreparedField` (`runtime/typescript/src/index.ts`), and its README says the prepared types are
  exported so a host can type what it holds, not as an API.
- The image those types are read from is lossy for this purpose. `nx-ir-format` erases record type
  parameters and type arguments to `object`. A type alias prepares to `{ tag: "typeAlias" }` with no
  target. A function carries its result type only when the source declares one. No doc comment is in
  the image: `doc-comments` requires that documentation not change generated IR executable content,
  and the archived `add-doc-comments` design states the IR "is runtime-only and is not read back for
  editor information, so docs do not need to be in it".
- The compiler has everything. A `ProgramArtifact` (`crates/nx-api/src/artifacts.rs`) holds, for
  every root module and every library module it links, a `ModuleArtifact` with the lowered module
  (declarations, each carrying `doc: Option<Doc>`, including `Param`, `RecordField`, `UnionCaseDef`
  and `UnionCaseField`), the checker's `type_env`, and the `prepared_module` through which a
  declaration's effective shape resolves across modules.
- `typegen` (`crates/nx-cli/src/typegen/model.rs`) already walks exported records, unions and
  aliases, renders documentation as Markdown with doc links as code spans (`typegen_doc`), and
  recognizes literal defaults (`ExportedLiteralDefault`). It does not cover functions, it lives in
  the CLI crate, and its model is organized around files to write.
- The `NxSchema` in `crates/nx-codegen/src/runtime.rs` is the validation vocabulary of the
  generated JavaScript runtime, not JSON Schema, and is not reused.
- Boundary validation is `normalizeValue`, `normalizeFields` and `normalizeNominalValue` in the
  TypeScript runtime and `crates/nx-ir-runtime/src/normalize.rs` in Rust. Both take a concrete
  record's field list from the declared type and so do not need `$type` there, require `$type` at
  an abstract record site and for a payload union case, read a constant case as a bare string,
  reject unknown fields, and reject `null` at an exactly-one site. `callFunction` in the TypeScript
  runtime and `call_function` in the Rust runtime take arguments keyed by parameter name.
- `record-type-inheritance` allows `extends` only from an abstract record, so a site typed by a
  concrete record admits exactly one shape. A union may also extend an abstract record, and its
  cases are then values of that record.
- The wasm module's ABI version is 5 (`bindings/wasm/native/src/lib.rs`); the README's ABI section
  still describes 4.
- `add-function-reference-type` and `add-agent-library` have landed. The checker gives the function
  reference type a variant of its own (`Type::AnyFunction`, `TypeRef::AnyFunction`), and
  `@nx/agent/agent.nx` is a compiler-carried library module every program that imports it links.

The consumer compiles in Node with `@nx-lang/sdk-wasm` and executes in a Cloudflare Durable Object
with `@nx-lang/ir-runtime` only. It resolves tool definitions when it compiles and stores them
beside the images; the Durable Object reads them and does not reflect over NX at run time.

## Goals / Non-Goals

**Goals:**

- One implementation of the mapping, in Rust, that every compiler SDK answers from.
- Schemas a host can hand to a model provider or an MCP client unchanged, and that are safe to
  trust: a value that passes the schema passes the runtime's boundary.
- Descriptions come from `///` comments with no second annotation mechanism.
- No change to the NX IR format, to either IR runtime's code, or to program meaning. The one change
  to emitted IR is which modules the entry image's module table lists (D14).

**Non-Goals:**

- Deriving schemas at run time from an image, in any runtime. A host that only executes stores the
  schemas it derived when it compiled.
- The .NET binding, the C FFI and a CLI command. They can be added over the same `nx-api` functions
  when a host needs them.
- A schema for a component's props or state, or for a component used as a type.
- Provider-specific reshaping: OpenAI strict mode (every property required, optional ones
  nullable) and property-name restrictions. Those belong to `add-agent-host-package` and its
  adapters. A result schema is returned as it is; MCP `2026-07-28` accepts any JSON Schema 2020-12
  as an `outputSchema`, so a non-object result needs no wrapping.
- Validating values against the schemas. The runtime's boundary validation remains the authority at
  run time; the host package may add a validator.
- Tool naming, missing-description policy and duplicate detection, which `add-agent-host-package`
  owns.
- Module documentation (`//!`), which does not exist yet.

## Decisions

### D1. Export at compile time, from the program artifact, not from the image

The export is two functions in `nx-api` over `&ProgramArtifact`, surfaced as two methods on each
compiler SDK's `NxProgramArtifact`.

The consumer already has the artifact in hand at the moment it needs schemas: it builds the tenant
workspace, emits images, and normalizes tools in the same request. The artifact is the only place
that has documentation, alias targets, type arguments and inferred result types together.

*Alternative: carry documentation and full types in the image, in an optional section like the
debug section, and derive schemas in `@nx-lang/ir-runtime`.* It would let an execute-only host
derive schemas, which no host needs: ReachMe stores normalized definitions in the bundle so a
conversation's tool set cannot change. It would grow every image a Durable Object fetches, require
the same derivation in the Rust runtime or let the two drift, need un-erased types in a format that
deliberately erases them, and sit against the `doc-comments` rule. Rejected. If an execute-only
host ever needs it, an optional non-executable section is the way, and nothing here blocks it.

*Alternative: a TypeScript library that derives schemas from `PreparedDeclaration`.* It cannot
produce descriptions, alias targets, applied type arguments or inferred results, and it would turn
the prepared types into an API. Rejected.

*Alternative: extend `typegen` with a JSON Schema target.* `typegen` writes files for exported
types of a library and has no notion of a function. A host would have to run a CLI and read files
during a request. Rejected, though the two share the documentation rendering (D8).

### D2. A function is named by module identity and name

The key is the pair a canonical `Function` record carries (`function-values`), so the host package
can go from an evaluated `FunctionTool.function` value straight to its schema. The module defaults
to the entry module. Any function declaration qualifies, exported or not, entrypoint or not, because
a `Function` record can name any of them. A library module is named by its identity in the program,
`<root>/<module>`, which is what the record carries for a library function.

### D3. One answer holds both schemas, parameter entries and diagnostics

```ts
interface NxDeclarationRef { readonly module?: string; readonly name: string }

interface NxFunctionSchemaOptions {
  /** Types whose parameters the host fills in; see D7. */
  readonly hostSuppliedTypes?: readonly NxDeclarationRef[];
}

interface NxFunctionSchema {
  readonly module: string;
  readonly name: string;
  readonly description?: string;   // the function's documentation
  readonly summary?: string;       // its first Markdown block
  readonly parameters: readonly NxParameterSchema[];
  readonly inputSchema?: NxJsonSchema;   // absent with an error diagnostic
  readonly outputSchema?: NxJsonSchema;  // absent with an error diagnostic
  readonly resultType: string;     // NX spelling, declared or inferred
  /** The whole declaration's span in `module`'s source; absent when the artifact has no source. */
  readonly declaration?: NxTextSpan;
  readonly diagnostics: readonly NxDiagnostic[];
}

interface NxParameterSchema {
  readonly name: string;
  readonly type: string;           // NX spelling
  readonly required: boolean;
  readonly description?: string;
  /** The record or union the parameter is declared with, ignoring its `?` mark. */
  readonly typeRef?: { readonly module: string; readonly name: string };
  /** The listed host-supplied type this parameter matched. */
  readonly hostSupplied?: { readonly module: string; readonly name: string };
}

interface NxTypeSchemaOptions { readonly direction?: "input" | "output" }  // default "output"

interface NxTypeSchema {
  readonly module: string;
  readonly name: string;
  readonly description?: string;
  readonly summary?: string;
  readonly schema?: NxJsonSchema;
  readonly diagnostics: readonly NxDiagnostic[];
}

// on NxProgramArtifact, in both SDKs
functionSchema(ref: NxDeclarationRef, options?: NxFunctionSchemaOptions): NxFunctionSchema;
typeSchema(ref: NxDeclarationRef, options?: NxTypeSchemaOptions): NxTypeSchema;
```

`NxJsonSchema` is a plain JSON object type. The Rust side builds `serde_json::Value` documents
with an order-preserving map and serializes once; both bindings pass the JSON through, which is
what makes the Node and wasm answers identical text.

Input and output are separate documents, each with its own `$defs`, because they are handed to
different consumers (MCP's `inputSchema` and `outputSchema`) and because a record is written
differently in each direction (D5).

`declaration` is cheap: `Function` in HIR stores the span of its whole syntax node, the artifact
keeps each module's source (`ProgramArtifact::source_text`), and `nx-api` already converts a range
and source to the `NxTextSpan` a diagnostic label carries (`text_range_to_span` in
`crates/nx-api/src/diagnostics.rs`). The wasm SDK already uses the same field name and type for a
value node's declaration. The module identity beside it says which document the span is in.

`typeRef` lets the host package ask `typeSchema` for a host-supplied parameter's own type, which may
be a host subtype such as `ChatToolContext`, without parsing `type`.

A type with no JSON form comes back as data, not as a thrown error, following `validateWorkspace`,
which answers diagnostics as data. A host compiling a config wants every problem in one pass, and
wants the input schema even when the result is not expressible. An unknown reference throws: it is
a caller bug, not a property of the program.

### D4. The schema describes the canonical encoding and nothing wider

The contract is one-directional agreement: schema-valid implies boundary-accepted for input, and
runtime-returned implies schema-valid for output. The schema is allowed to be narrower than the
boundary. The boundary's leniencies (a single value at a `+` site, `null` or `[]` for an absent
optional, a one-element array at a `?` site, a non-integral number at an `int` site in the
TypeScript runtime, an argument the function does not declare) are not described.

Consequences:

- `int` and `int64` are `integer`. The TypeScript runtime checks only `typeof number` at those
  sites, so `integer` is narrower and safe. `int32` adds its bounds, which both runtimes enforce.
- An optional field or parameter is "absent", never `null`: the property is left out of `required`
  and its schema does not admit `null`. A standalone `T?` result is the one place the output admits
  `null`, because `entryResult` returns it.
- Objects are closed (`additionalProperties: false`). The runtime rejects unknown record fields, and
  drops unknown arguments; a closed argument object is the narrower, clearer statement.
- `object` is `{ "not": { "type": "null" } }`: any value but `null`, in both directions. A runtime
  takes an array or object at an `object` site whole, but reads `null` as no value and refuses it
  where exactly one value is expected, so the empty schema `{}` would be wider than the boundary.

*Alternative: describe everything the boundary accepts* (`anyOf` of item and array, nullable
optionals). It makes schemas larger and vaguer for a model, and no consumer wants to produce the
non-canonical spellings. Rejected.

### D5. `$type` is asked for only where the runtime needs it, and always promised

In the output direction every record and payload case has `$type` as a required `const`, because
every runtime writes it. In the input direction a concrete record at a site typed by that record
has no `$type` property at all, because the runtime takes the fields from the declared type there
and a model that need not write a discriminator makes fewer mistakes. The discriminator is required
in input only at an abstract-record site and for a payload union case, where the runtime uses it to
choose a shape. If one document mentions a record both ways, it gets one definition with `$type`
required; that is still narrower than the boundary.

Leaving `$type` out of single-shape input also keeps `$`-prefixed property names out of most tool
input schemas, which some providers restrict.

*Alternative: require `$type` everywhere, in one schema per type.* Simpler, and a type would have
one schema. It makes every nested argument record carry a constant the runtime ignores. Rejected,
at the cost of the `direction` option on the type query.

### D6. Polymorphism is closed over the program

An abstract-record site is `anyOf` the concrete records and union cases that extend it in any
module of the program artifact, library modules included. That is the set a value can come from at
run time for a pinned bundle, and with D14 it is what `resolveSubtype` accepts in a program linked
from its entry. A program compiled later with another descendant has a different schema, which is
correct: the host re-derives when it compiles.

Two descendants with the same `$type` at one site are refused with
`schema-ambiguous-discriminator`, mirroring the runtime's "Ambiguous subtype" diagnostic, instead of
emitting an `anyOf` no value can satisfy unambiguously.

`anyOf` is used, not `oneOf`: the branches are disjoint by their `$type` constants (or by being a
string versus an object), so the two are equivalent for valid values, and `anyOf` is cheaper to
validate and more widely supported by model providers. No `discriminator` keyword is emitted; it is
OpenAPI vocabulary, not JSON Schema 2020-12.

### D7. Host-supplied parameter types are an option, matched in the compiler

`add-agent-host-package` must leave a `ToolContext`-typed parameter out of the model-facing schema
and fill it from host context, including for a host's own subtype (`ChatToolContext`). Deciding
"is this parameter's type `ToolContext` or a record that extends it" needs the base chain across
modules, which the compiler has and a host does not. So the function query takes
`hostSuppliedTypes`, and the answer marks each matching parameter and leaves it out of
`inputSchema`. The export knows nothing about the agent library; the names come from the caller.

A listed type the program does not declare matches nothing and is not an error, so a host passes
one constant list for every tenant, whether or not the tenant's program links the agent library.

*Alternative: report each parameter's nominal type and base chain and let the host decide and
rebuild the input schema.* More surface, and every host would re-implement the subset logic and the
`required` list. Rejected.

### D8. Descriptions are the documentation's Markdown, links as code spans

The same rendering `typegen` uses for TypeScript: `Doc::replace_links(|link| Some(link.code_span()))`.
`typegen_doc` moves from `nx-cli` to a method on `nx_hir::Doc` so both callers share it. The full
text is the `description`; the function's and type's `summary` (first Markdown block, as
`doc-comments` defines it) is returned beside it for hosts that want the short form. The export
does not truncate, strip Markdown, or apply length limits; models read Markdown well and limits are
provider policy.

JSON Schema has no place for a description per `enum` value. Rather than switch a constant union to
`oneOf` of `const` schemas, which is larger and which some strict modes reject, documented constant
cases are appended to the union's description as a Markdown list. A union with no documented case
keeps exactly its own documentation.

An inherited field carries the documentation of the declaration that declared it, which `Doc`
already shares across copies.

### D9. Defaults are written only when they are literals

A function evaluates its own defaults and a default may read earlier parameters
(`function-parameters`), so a default is an expression, not a value. The export writes the JSON
Schema `default` keyword for a string, numeric or boolean literal and for a reference to a constant
case of the site's union, and writes nothing otherwise. It never evaluates. In every case the
member is simply not `required` in input, which is the part that matters; `default` is annotation.
This is the same literal set `typegen` supports (`ExportedLiteralDefault`) plus constant cases.

### D10. Generic records are described per instantiation

IR treats a type-parameter field as `object`, but the artifact knows the argument at each applied
type. Substituting it gives a narrower schema that the checker guarantees for output and that the
boundary accepts for input. Each instantiation is its own `$defs` entry keyed `<Record>_<arg>`.
Asking for a generic record by name, with no arguments, maps parameter-typed fields as `object`.
A record that applies itself to a growing argument, `inner?:<Box T=<Box T=T /> />`, would need one
entry per depth, and so would one that grows its argument through another generic record,
`inner?:<Box T=<Pair L=T R=T /> />`. Once an instantiation's arguments apply the record twice, or
nest applied types more than four deep, it is described as the record named with no arguments
instead; the depth bound alone guarantees the walk ends, since a program has finitely many shapes
of bounded depth. Its parameter fields then take the `object` schema, which is
what the runtime checks there, since IR erases record type arguments, so the agreement holds.

`add-agent-library` does not use this for `HttpArguments`: its `body` field is typed `object`, so an
arguments function's output schema gives `body` the schema of `object` (D4) and the record has one
`$defs` entry.

### D11. Types with no JSON form

One code, `schema-inexpressible-type`, error severity, labeled at the parameter, field or result
annotation that holds the type, with a message naming the type in NX spelling and the path from the
root of the schema (`Catalog.rows.template`). The cases:

- A function type. Its wire form is the `Function` record, which names code rather than describing
  data; a model cannot produce one, and a host should not be led to accept one from a model.
- The function reference type `<function ... />: R` from `add-function-reference-type`, whatever
  its result, for the same reason. The message names the type as written,
  `<function ... />: object*` for a field any function satisfies. The export reads the checker's type, where it is its
  own type and not `object`, so such a field is reported and an `object` field is not. In an image
  the two are also distinct (type kind `anyFunction`, numbered 6, with the result as its operand,
  behind the required feature `function-reference-type-v1`), but the export never reads an image.
- A component used as a type. A rendered component may hold action handlers, and boundary
  validation passes such a value through unchecked. Markup, an element such as `<div />` or the
  built-in `Element`, has no JSON form for the same reason; an unannotated function that returns
  markup is reported as returning markup.
- An abstract record with no concrete descendant in the program. No value exists.
- A result type the checker left as an unresolved placeholder. The message says to annotate the
  result. `infer-unannotated-return-types` shrinks this case.

The walk collects every such member before answering, so one call reports them all.

One consequence for the agent library: `FunctionTool.function` and `HttpTool.arguments` are function
reference fields, and a site typed by the abstract `Tool` is `anyOf` every concrete tool in the
program (D6), so a function that takes or returns a `Tool` or an `Agent` has no JSON form on that
side. That is intended. Tools and agents are configuration a host evaluates, not values a model
supplies or reads; the schemas a host needs are those of the functions the tools name, whose
parameters and results are ordinary data.

### D12. Where the code goes

- `crates/nx-api/src/schema.rs`: `program_artifact_function_schema(&ProgramArtifact, module, name,
  &FunctionSchemaOptions)` and `program_artifact_type_schema(...)`, returning serializable structs.
  The walk resolves a `TypeRef` through the declaring module's `prepared_module`, so a type written
  in a library module is resolved in that module's namespace, the way IR generation and typegen
  resolve it. It does not touch `nx-interpreter`, so `retire-hir-interpreter` does not affect it.
- `bindings/wasm/native`: `nx_wasm_program_function_schema(handle, ptr, len)` and
  `nx_wasm_program_type_schema(handle, ptr, len)`, JSON in and JSON out like the other program
  operations. The ABI version goes up by one from whatever it is when this lands.
- `bindings/node`: two napi methods on the native program artifact, returning the JSON text the
  wrapper parses.
- TypeScript types are declared once per binding with identical names, as the two bindings already
  do for diagnostics and IR metadata.

### D13. The agreement is tested, not assumed

A corpus of NX sources, one per mapping in the spec, lives with the wasm SDK's tests, which already
depend on `@nx-lang/ir-runtime`. For each corpus function the test (a) validates hand-written
canonical arguments and generated minimal arguments (every optional property left out) against the
input schema with a draft 2020-12 validator, calls `callFunction` with them, and requires no
boundary diagnostic; (b) validates every returned value against the output schema. Rust unit tests
in `nx-api` hold golden schema documents for the same corpus, so a mapping change is visible in
review. The Node and wasm parity test compares the two bindings' answers as JSON text.

Ajv (its 2020-12 build) is added as a dev dependency of the wasm SDK package for this. It is not a
runtime dependency of any published package.

### D14. The entry image links every module that declares a boundary subtype

A runtime links the modules images reference: each image's module table lists the modules its code
names, and linking follows those tables from the entry. A module that only declares a subtype of an
abstract record, such as `type X extends Base` in a module the entry imports but never uses, is
referenced by nothing and is never linked, so the runtime refuses `{ "$type": "X" }` at a site typed
`Base`, while D6 lists `X` in the schema. Inside NX this never happens, since constructing an `X`
references its module; only a value from a host can name such a shape, and that is exactly what a
tool argument is.

So the entry image's module table also lists, after the modules it references, every module that
declares a concrete record or union extending an abstract record reachable from the parameter or
result type of a function of a module the entry reaches through its imports. A function of any
other module is never linked from the entry, so a host cannot call it, and counting it would make
the entry image depend on modules it has nothing to do with. The walk visits each declaration once,
whatever its type arguments, and visits the arguments on their own, so a recursive generic record
ends. `nx-api` computes the set
(`program_artifact_boundary_subtype_modules`) with the same walk and the same descendant search the
export uses, so the schema and the link agree by construction, and `nx-codegen` adds the modules to
the entry's table. Only the entry image changes: a library module's image must stay the same bytes
whichever program emitted it, and a host links from the entry.

*Alternative: compute the linked set in the export and narrow the schema to it.* It would duplicate
codegen's reference rules in `nx-api` and leave a host unable to supply a shape the program
declares. Rejected. *Alternative: document that a host must link every module.* Linking follows
module tables, so a host has no way to ask for more. Rejected.

## Risks / Trade-offs

- [A schema drifts from boundary validation when either changes] → The agreement tests in D13 run
  on every build over the full mapping; a new type form without a mapping fails the golden tests.
- [The agreement is tested against the TypeScript runtime only] → It is the runtime every consumer
  of the export runs. The Rust runtime also calls by name and normalizes the same canonical
  encoding, but checking the schemas against it waits for a host that runs it; the corpus and the
  argument fixtures carry over unchanged.
- [Two mappings from NX types exist: typegen's host types and this one] → They answer different
  questions and share the documentation rendering and literal-default rules. A later change can move
  typegen onto the `nx-api` walk; not needed now.
- [`int64` values beyond 2^53 are not exact as JSON numbers] → Existing property of the canonical
  JSON encoding. The schema says `integer`; the website page notes the limit.
- [A large library makes an abstract site's `anyOf` large] → It lists only descendants in the
  program, and each is one `$ref`. Hosts that find a schema too large for a provider see that at
  compile time and can reject the config.
- [Providers accept different JSON Schema subsets: no `$ref`, no `$defs`, no `anyOf` at the root,
  required-everything strict modes] → Out of scope here by design; `add-agent-host-package` adapts.
  Emitting standard 2020-12 with `$defs` keeps the export honest and lets recursion work.
- [Descriptions carry Markdown and can be long] → Passed through as written; `summary` is offered
  for short forms. A size guard is the host's.
- [The wasm ABI bump collides with `add-go-to-definition`, which also bumps it] → Each takes "one
  more than current" at the time it lands, as that change's design already says.
- [Closed-world polymorphism means a stored schema is stale after a library adds a descendant] → A
  stored schema belongs to the bundle it was derived with, and a conversation pins its bundle.

## Migration Plan

Additive. No existing API, image or behavior changes.

1. Land the `nx-api` module with golden tests.
2. Land the wasm and Node bindings and the parity and agreement tests.
3. Update the READMEs and the website.
4. Release: 0.6.0 carries doc comments (already on `main`) and this export, and its changelog lists
   both. Hosts move their pin to that version; the wasm loader refuses an older module by ABI
   version.

Rollback is removing the two methods; nothing persists that depends on them inside NX.

Sequencing with other changes in this repository:

- `add-function-reference-type` (N1): landed. This change reports the type it added with
  `schema-inexpressible-type` (task 3.6).
- `add-agent-library` (N4): landed; no code dependency. The tests import the real `@nx/agent`
  rather than a copy of its declarations: the module identity `@nx/agent/agent.nx`, the abstract
  record `ToolContext` with `callId:string`, and `HttpArguments` with `body?:object`. A host names
  `{ module: "@nx/agent/agent.nx", name: "ToolContext" }` in `hostSuppliedTypes`. Because
  `ToolContext` is abstract, a parameter typed by it directly would have no JSON form in a program
  with no subtype; D7's rule that a host-supplied parameter is not checked covers that. A site typed
  by its `Tool` or `Agent` has no JSON form (D11).
- `add-agent-host-package` (N5): lands after this change and calls
  `functionSchema({ module, name }, { hostSuppliedTypes })` and `typeSchema(ref, { direction })`,
  reading `description`, `summary`, `parameters[].{ name, type, required, description, typeRef,
  hostSupplied }`, `inputSchema`, `outputSchema`, `resultType`, `declaration` and `diagnostics`.
- ReachMe `add-agent-function-tools` (R2): calls `artifact.functionSchema` in `apps/api` while the
  artifact is alive, naming `ToolContext` as host-supplied.
- `add-ir-runtime-evaluation-budget` (N3): unrelated.
- `infer-unannotated-return-types`: independent; it improves the inferred result types this export
  reads.
- `retire-hir-interpreter`, `support-stateful-component-value-evaluation`,
  `add-ir-runtime-performance-harness`: no overlap.

## Open Questions

- **Should `additionalProperties: false` be optional?** Default taken: always closed, matching the
  runtime's rejection of unknown record fields. A host that wants open objects can post-process.
- **Should a component used as a type get a loose object schema?** Default taken: no JSON form.
  ReachMe's `AgentStep` output is fixed by the step type and does not need it.
- **Should the answer also carry a fingerprint tying it to the emitted image?** Default taken: no.
  The host derives schemas and emits images from one artifact in one request and stores them
  together.
- **Design doc §9 decision 7** (does NX ship the export): taken as yes, per the brief.

## Deviations from the design doc

- §6.1 says the IR "already carries full types" and that `@nx-lang/ir-runtime` exports them. The
  image erases generic arguments, has no alias targets, no inferred result types and no
  documentation, so the export cannot be built on it. It is built on the compiler's program
  artifact instead, which the doc allows ("NX ships a supported schema export").
- §5.2's `findPlans` sketch is consistent with the grammar as far as this change is concerned:
  paren-style parameters are comma-separated, and `doc-comments` lists function parameters as
  documentable, so a leading `///` above each parameter becomes that parameter's description.
