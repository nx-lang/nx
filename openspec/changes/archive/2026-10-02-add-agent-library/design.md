## Context

See proposal.md for the motivation. The facts that shape the design:

- **The prelude is the only source the compiler carries.** `crates/nx-api/src/prelude.nx` is embedded
  with `include_str!` in `crates/nx-api/src/artifacts.rs`, built once per process by
  `build_prelude_library()` through `build_library_artifact_from_sources`, and held in a `OnceLock`
  (`prelude_library()`). It is an ordinary `LibraryArtifact`, so name binding, codegen, `typegen` and
  the language service read it with no case of their own. `apply_prelude_bindings` binds its exports
  last in every module. Its identity `@nx/prelude.nx` and the prefix `@nx/`
  (`nx_hir::PRELUDE_ROOT_PREFIX`) are constants in `crates/nx-hir/src/lib.rs`. Only the exact
  identity is refused today (`LogicalModuleGraph::from_modules` in `source_graph.rs`).
- **Host libraries load into a registry.** `LibraryRegistry::load_library_from_sources` takes a
  logical root, an optional version and modules. Imports between in-memory libraries are relative
  (`import "../question-flow"`) and resolve against loaded roots. `ProgramBuildContext` carries
  `visible_roots` and `implicit_imports`; an implicit import names a workspace module or a visible
  library root. A library module's NX IR image is named `<root>/<module>` and records the version
  the host gave the library (`module_version` in `crates/nx-codegen/src/ir.rs`); the prelude records
  `nx_hir::PRELUDE_VERSION` instead, because runtimes carry their own copy of its image.
- **Import paths are relative.** `normalize_workspace_import_identity` joins a path that does not
  start with `/` to the importer's directory. `import "@nx/agent"` in `app/main.nx` would mean
  `app/@nx/agent` today.
- **The consumer.** ReachMe's API (`apps/api/src/conversation/nx/`) loads two in-memory libraries,
  `nx-built-in/libraries/question-flow` and `nx-built-in/libraries/chat-link`, with a content hash as
  the version, compiles tenant source against them as implicit imports, emits one image per module,
  and stores them in a runtime bundle a Durable Object links with `@nx-lang/ir-runtime`. Its
  libraries will import the agent types and extend them.
- **The sketches were never compiled.** The design doc's §5.1, §5.2 and §5.7 are sketches, and §6.2
  lists three constructs to confirm. This design compiled them; the results are under
  "Verification" below.

## Goals / Non-Goals

**Goals:**

- One mechanism that carries a named NX library in the compiler and makes it importable from every
  kind of source, reusing the prelude's machinery instead of adding a second pipeline.
- The agent library's source, checked against the real grammar and compiler.
- Host libraries can import the agent library and extend its abstract types.
- No change to NX IR's schema, to either IR runtime, or to any SDK's API or ABI.
- A versioning rule that makes a stale library image impossible to link by accident.

**Non-Goals:**

- Behavior. The library has no functions; schema derivation, tool naming, validation and execution
  belong to `add-agent-host-package`.
- A general package manager, remote imports, or user-installable standard libraries. Git and HTTP
  import paths stay unresolved as `module-imports` already says.
- Shipping a standard library's image inside a runtime, as the prelude's is.
- Putting agent names in scope by default.
- The types the source design defers: `auth` on `HttpConnection`, `McpConnection`,
  `OpenApiConnection`, `approval`, `agents`, `skills`, `reasoning`, a `response` function on
  `HttpTool`, and an authored `output` type on `Agent`.
- ReachMe's types (`AssistantConfig`, `RecordSearchTool`, `MeetingSchedulerTool`, `AgentStep`,
  `FollowUpStep`, `ChatToolContext`, the model tier names).

## Decisions

### D1. A standard library is carried source, built like the prelude, imported like a library

`crates/nx-api/src/std/agent/agent.nx` is embedded with `include_str!`. A static table lists each
standard library: its name, its modules (library-relative identity and source) and its stability.
`standard_library(root)` builds the `LibraryArtifact` on first use through
`build_library_artifact_from_sources` with root `@nx/agent`, exactly as `build_prelude_library` does,
and keeps it in a per-library `OnceLock`. The prelude's bindings apply to a standard library's
modules, so `apply_prelude_bindings` must skip only the prelude itself rather than everything under
`@nx/`; the deadlock its comment warns about does not arise because a standard library is built
outside `prelude_library()`'s initialization.

Lookup is added at the two places a library root is resolved, not as a registry entry:
`LibraryRegistry::get_loaded_library` and `ProgramBuildContext::visible_library` (and the
logical-identity variants) answer for a root under `@nx/` from the process-wide table. Every registry
therefore sees the same snapshot, a context with limited `visible_roots` still sees it, and nothing
is "loaded". Dependency metadata, the program's library closure and IR emission then work unchanged,
because they only ever ask the registry for a root.

*Alternatives.* (a) Put the agent types in the prelude. Rejected: `Tool`, `Document`, `Agent` and
`Connection` are common names, the prelude is in every module's completions, and every program's
fingerprint would move. (b) Publish the `.nx` files in an npm package and have each host load them
into its registry under a root it picks. Rejected: two hosts would give the same types different
module identities, so a `$type`-qualified reference or a stored image would not be portable, and
the CLI, the LSP, `typegen` and the playground would each need the files handed to them. (c) Carry
it in the compiler but preload it into every registry. Rejected: registries are created per call in
the wasm crate, and analysis would be paid by programs that never import the library.

### D2. The import path is `@nx/<name>`, resolved before relative normalization

An import path whose first segment is `@nx` is a standard library specifier. The check happens
before `normalize_workspace_import_identity` and its library counterpart, so the path is the same
from any module at any depth and never escapes a root. Only the two-segment form names a library;
`@nx/agent/agent.nx` and `@nx/nope` get one diagnostic, `unknown-standard-library`, that lists the
libraries that exist. An implicit import identity of `@nx/agent` synthesizes `import "@nx/agent"`
verbatim instead of going through `workspace_import_path`, which would prepend `../`.

The `@nx/` prefix is already documented as reserved (`reference/syntax/modules.md`) and already a
constant. This change enforces the reservation: `LogicalModuleGraph::from_modules` refuses any
module identity under it, and `prepare_in_memory_library` and `load_library_from_directory` refuse a
root under it. The constant is renamed `NX_RESERVED_ROOT_PREFIX`.

*Alternatives.* A bare name (`import "agent"`) collides with relative directories, which is how
`import "../ui"`-style paths are already written without `./`. A URL scheme (`import "nx:agent"`)
would be a third spelling next to the prelude's `@nx/` identity, and the identity of a module should
be spelled the way it is imported. A relative path would be wrong at every depth but one.

This is breaking for a project that has a real directory named `@nx` beside a module and imports it
as `@nx/...`. Such a directory has to be renamed. No such project is known; the root has been
documented as reserved since the prelude landed.

### D3. Opt-in, not ambient

A standard library is in scope only where it is imported, by text or by a host's implicit imports.
The prelude stays the only ambient module. Consequences: a program that does not import the library
emits byte-identical IR; the language service indexes a standard library only when a snapshot
document or the snapshot's implicit imports name it; ordinary import rules (ambiguity on use,
duplicate import) apply with no special binding order.

What a host does, with ReachMe as the example:

- It loads nothing. There is no agent library source to stage and no registry call to make, and a
  load under `@nx/agent` is refused (D2). A library load order does not involve the standard
  library: `chat-link` and `question-flow` load into an empty registry and resolve it.
- It adds the identity `@nx/agent` to its implicit imports beside its own two roots, so authored
  configs need no import line.
- It writes `import "@nx/agent"` in each of its own library modules that names an agent type
  (`ChatLinkConfig.nx`, and the question-flow module that declares `AgentStep`), because a module of
  an implicitly imported library receives no implicit imports (`workspace-programs`).
- It emits `@nx/agent/agent.nx` with the other library images and stores it in its bundle (D4).
- It reads the library's version from that emitted image, as `prepareNxIrModule(bytes).version`.
  The emit metadata carries the module's identity and fingerprint and is left as it is, so no
  exchanged shape changes. A host that needs the
  version before any tenant compiles, as ReachMe does for its built-in libraries fingerprint,
  builds the one-line source `import "@nx/agent"` at startup and emits the library image from it.
  No listing API is added for this.

### D4. Images are emitted, never built in, and the version is content-derived

The prelude's image ships inside `@nx-lang/ir-runtime` (`runtime/typescript/src/prelude-image.ts`)
and in `crates/nx-ir-runtime/src/prelude.nxir`, and its version is a hand-bumped contract number so a
runtime's copy can be compared with what an image expects. A standard library does not follow that
model. Its image is emitted from the program that links it, like any library module's, and the host
stores and resolves it. For ReachMe that is one more image in the runtime bundle, fetched on demand
like the others.

Because nothing carries a second copy, the version does not need to be a contract number. It is the
16-digit lowercase hex of a 64-bit FNV-1a hash over, for each module in identity order, the
library-relative identity, a zero byte, the source text and a zero byte. FNV-1a is what the module
fingerprint already uses (`docs/nx-ir-format.md`), so it is platform-stable and adds no dependency.
`module_version` returns it for a module under a standard root. A comment edit changes the version.
That is accepted, not needed: the image holds no documentation (`doc-comments` keeps it out of the
IR, and `add-declaration-schema-export` reads it from the program artifact), but hashing the source
is the one rule that needs no judgment about which edits matter, and the entry and library images
of one bundle always come from the same compiler.

The version, the module fingerprint and the source positions in an image all depend on the exact
bytes of the embedded source. `.gitattributes` already checks every `.nx` file out with LF
(`*.nx eol=lf`), so the embedded bytes are the same on every platform, and a test pins the version
of the shipped library so a change to those bytes cannot pass unnoticed.

*Alternatives.* The package version (`0.6.0`) as the library version would give two releases with
identical library source different versions, which is harmless, and would give two local builds
with different source the same version, which is not. A hand-bumped number repeats the prelude's
maintenance burden for no benefit.

*Why not ship the image in the runtime.* It would put host-facing vocabulary inside the runtime,
which the source design keeps pure, it would have to be done twice (TypeScript and Rust), and a
pinned ReachMe bundle must keep the exact image it was compiled with regardless of the runtime
version that later evaluates it.

### D5. `unstable` is a documented contract, not a compiler feature

The stability lives in the static table, in the header comment of `agent.nx`, on the website page
and in release notes. The compiler does not warn on import and there is no opt-in flag: every
consumer of an unstable library would have to set it, and the warning would land in tenant-facing
diagnostics in ReachMe. What `unstable` means for versioning:

- NX packages share one version (`package-release-automation`). An unstable library may change
  incompatibly in any release, including a patch release. It does not hold back or force the
  packages' version number.
- A host gets a change only by moving its pin of the NX packages. Until then its declarations and
  image version are fixed.
- A stored image is unaffected by a later release; it links against the library image stored with
  it. A stored *source* may need migrating when a type changes, which is the host's job.
- Every release that changes the library lists the change under `@nx/agent` in the release notes
  (the repository's one changelog today is `src/vscode/CHANGELOG.md`; the GitHub release notes carry
  the package side).
- The library becomes `stable` by a later change, once the first ReachMe scenarios work. From then an
  incompatible change needs a breaking-version release.

### D6. The library source

One module, because the types refer to each other, a host fetches one image instead of several, and
thirteen short declarations read well in one file. This is the file to add, with one substitution
described under "Verification".

```nx
// The NX agent library: product-neutral types for declaring an AI agent, its reference documents
// and its tools. Import it with `import "@nx/agent"`.
//
// UNSTABLE. These declarations may change incompatibly in any NX release, including a patch
// release. See https://nxlang.org/reference/libraries/agent.
//
// The library holds types only. What a tool does, how its schemas are derived and how a run
// proceeds are the host's business.

/// Limits on one run of an agent. A host applies its own caps as well, and the lower value wins.
export type AgentLimits = {
  /// The most model calls one run may make.
  maxSteps?: int
  /// The most tool calls one run may make.
  maxToolCalls?: int
}

/// Reference text sent to the model on every run, kept separate from the agent's instructions.
export type Document = {
  /// Names the document in traces and citations. Unique among one agent's documents.
  title: string
  /// The document's text, written as the element body.
  content text: string
}

/// Something the model can call during a run. Each concrete tool type says what the call does.
export abstract type Tool = {
  /// The model-facing name, in snake_case. Each tool type has a default.
  name?: string
  /// The model-facing description. Each tool type has a default.
  description?: string
}

/// A reusable definition of an agent: who answers, with what reference text and which tools.
export type Agent = {
  /// A stable identifier for the agent, shown in traces.
  name: string
  /// What the agent is for. Shown to operators and not sent to the model.
  description?: string
  /// The model to run. NX does not interpret it; the host resolves it and may restrict it.
  model?: string
  /// Reference text sent to the model on every run, after the instructions.
  documents?: Document+
  /// The tools the model may call.
  tools?: Tool+
  /// Limits on one run.
  limits?: AgentLimits
  /// How the agent behaves: the system prompt, written as the element body.
  content instructions: string
}

/// Exposes an NX function to the model. The tool's input schema comes from the function's
/// parameters and its output schema from the function's result type.
export type FunctionTool extends Tool = {
  /// The function the tool calls. Its name and doc comment are the tool's default name and
  /// description.
  function: <function ... />: object*
}

/// Web search run by the model provider. The default name is `web_search`.
export type WebSearchTool extends Tool = {
  /// The only domains the search may draw on. Without it the search is unrestricted.
  allowedDomains?: string+
}

/// Values the host supplies to a tool's function. A function parameter of this type, or of a type
/// that extends it, is filled in by the host and left out of the tool's input schema, so the model
/// never supplies it. A host extends this type with the values it provides.
export abstract type ToolContext = {
  /// Identifies this tool call and stays the same when the call is retried, so it can serve as
  /// an idempotency key.
  callId: string
}

/// The method of an HTTP request.
export type HttpMethod = get | post | put | patch | delete

/// An external service a tool can reach. The model never sees a connection's address.
export abstract type Connection = {
  /// Identifies the connection in traces.
  name: string
  /// What the service is. Shown to operators and not sent to the model.
  description?: string
}

/// A service reached over HTTPS.
export type HttpConnection extends Connection = {
  /// The URL every request to the service starts with. It must use `https`.
  baseUrl: string
}

/// One named value of an HTTP request. The host encodes the value.
export type HttpParam = {
  /// The parameter's name.
  name: string
  /// The parameter's value, not encoded.
  value: string
}

/// The variable parts of one HTTP call, returned by an [HttpTool]'s `arguments` function.
export type HttpArguments = {
  /// Values for the `{name}` placeholders in the tool's path.
  pathParams?: HttpParam+
  /// Query string parameters, in order.
  query?: HttpParam+
  /// The request body, which the host sends as JSON. Usually a record the program declares.
  body?: object
}

/// One operation on a connection: a fixed method and path. Only the arguments vary per call.
export type HttpTool extends Tool = {
  /// The service the tool calls.
  connection: HttpConnection
  /// The request method. `get` makes the tool read-only; any other method is a write.
  method: HttpMethod
  /// The path under the connection's base URL. It may hold `{name}` placeholders.
  path: string
  /// A function that returns the call's [HttpArguments]. Its parameters are the tool's input
  /// schema, and its name and doc comment are the tool's default name and description.
  arguments: <function ... />: HttpArguments
  /// The most calls to this tool one conversation may make. A host applies its own cap as well.
  maxCallsPerConversation?: int
}
```

Notes on the source:

- Plain `//` comments form the file header. A `///` block there would attach to `AgentLimits`.
- `HttpConnection` declares no `auth`. The name is reserved by the absence of any other use; a
  record cannot reserve a field name in the language.
- No field has a default. Defaults that a host applies (tool names, descriptions, limit caps) are
  not NX defaults, because they depend on the host or on the referenced function.
- The doc comments are the model- and operator-facing descriptions that
  `add-declaration-schema-export` exports, so they are written as descriptions, not as notes to
  maintainers.

### D7. `ToolContext` is abstract (deviation from the sketch)

The sketch declares `export type ToolContext = { callId: string }` and has ReachMe write
`ChatToolContext extends ToolContext`. The compiler rejects that: "Record 'ChatToolContext' extends
'ToolContext', but only abstract records may be extended" (`record-type-inheritance`). `ToolContext`
is therefore `abstract`. A host that fills a context parameter supplies a record of a concrete
subtype it declares; a function parameter typed `ToolContext` accepts any of them. The library adds
no concrete subtype, since the brief fixes the type list; see Open Questions.

`Tool` and `Connection` are abstract in the sketch already, for the same reason.

### D8. `HttpArguments.body` is `object`

`object` is NX's existing top type (`PrimitiveType` in `nx-grammar.md`; "every applied type
satisfies `object`" in `reference/syntax/types.md`). A field typed `object` accepts any one value: a
record of any type, a string or a number.

*Why not a generic record* (`type HttpArguments = { TBody:type ... body?:TBody }`):

- A type parameter has no default, so every arguments function for a request with no body, which is
  most reads, would have to write `<HttpArguments TBody=string/>` as its return type and at the
  construction site.
- `HttpTool.arguments` is a function reference of any signature, so the checker never relates the
  tool to one instantiation. The parameter would buy no checking at the tool.
- The IR erases type arguments: a parameter-typed field is `object` at run time
  (`reference/syntax/types.md`), so the host sees the same thing either way.
- The body's static type is still available to a host that wants it: it is the type of the
  expression in the function, and the value carries its `$type`.

*Why not a new top type for records only:* it needs a language change, and it would forbid a JSON
string or number body for no benefit.

The cost is that `object` admits exactly one value, so a top-level JSON array body is not
expressible; a program wraps the list in a record. That is acceptable for the first scenarios and
noted for `add-agent-host-package`, which owns serialization.

### D9. The function reference type comes from `add-function-reference-type`

`FunctionTool.function` is typed `<function ... />: object*` and `HttpTool.arguments` is typed
`<function ... />: HttpArguments`. What this library takes from that change, by its final text:

- `<function ... />: R` is a function type whose parameters are not stated: a function of any
  parameters satisfies it when its result satisfies `R`. Every result type satisfies `object*`, so
  every function satisfies `<function ... />: object*`. That is the type of a function tool's
  function, whose result may be a list or optional; `<function ... />: object` would reject those.
  The type has no name, so `agent.nx` writes it out, once.
- Only a function value satisfies either. `function="findPlans"` is rejected, and so is
  `function={ <Function module="main.nx" name="findPlans" /> }`: NX source cannot forge the record
  a host later calls. Both are `agent-library` scenarios.
- The checker requires an arguments function to return `HttpArguments`: `arguments={findPlans}`,
  where `findPlans` returns `string*`, is rejected as a result mismatch. That is an
  `agent-library` scenario too.
- Occurrences and defaults are ordinary there, but the library uses neither: both fields are
  required, exactly-one and carry no default. A tool with no function is not a tool.
- The value is opaque in NX and evaluates to `{ "$type": "Function", module, name }`, which is what
  the verification build already produced.
- A module whose type table holds the type lists the required feature
  `function-reference-type-v1`. `@nx/agent/agent.nx` therefore lists it, and every runtime that
  links the library image must support it. An entry image lists it only if its own type table
  holds the type. This is why ReachMe's `add-agent-tool-loop` adds the feature to its flow runtime's
  supported set with the pin bump.
- The runtimes do not re-check the result of a host-supplied `Function` record, so
  `add-agent-host-package` checks at each call that the arguments function returned an
  `HttpArguments` record.

`add-function-reference-type` has landed (archived 2026-10-02), and the source in D6 was compiled
against it as written; see "Verification".

### D10. `typegen` and the published types

`nxlang typegen @nx/agent` generates from the carried source, so the TypeScript types that
`add-agent-host-package` publishes in `@nx-lang/agent` are generated, not hand-written, and a test
there can pin them to the compiler's source. A host library that references a standard library type
imports it from a fixed target held in the static table: `@nx-lang/agent` for TypeScript,
`NxLang.Agent` for C#. No C# package is published by this set of changes; a C# host generates the
contracts itself with `nxlang typegen @nx/agent --language csharp`, whose namespace defaults to the
table's `NxLang.Agent` so the contracts bind to what host libraries' generated code references.

Today's output for a library that imports a directory named `agent` is
`import type { Agent, Tool, ToolContext } from "agent";` with a warning that the package name was
assumed. The fixed target removes both the guess and the warning.

`typegen` renders `object` as `unknown` (TypeScript) and `object` (C#) today, so
`HttpArguments.body` is `unknown` in the published types. `add-function-reference-type` renders a
member of a function reference type, whatever its result, as `NxFunctionRef` in TypeScript, emitted once in the helper module, and as
`NxLang.Nx.NxFunctionRef` in C#, so `FunctionTool.function` and `HttpTool.arguments` are typed that
way.

### D11. Language service

`WorkspaceDeclarations` already indexes the prelude and the context's visible libraries. It also
indexes each standard library that a snapshot document imports or that the snapshot's implicit
imports name, found from the analyzed modules' imports. Hover uses a label parallel to the prelude's
"built-in type": "standard library `@nx/agent`". Completions need no new rule, since a name is
offered when it is in scope. Go-to-definition (`add-go-to-definition`) lists locations in the prelude
and in libraries as a non-goal, which covers standard libraries: there is no target yet.

### D12. Worked example

This is the example agent from the source design, corrected to what compiles. It assumes
`import "@nx/agent"` or the host's implicit import.

```nx
type Plan = { name:string seats:int monthlyPrice:int }
type NewTicket = { subject:string priority:int }

let companyName = "Example"

/// Finds the plans that fit a team.
let findPlans(
  /// Number of people who need a seat.
  teamSize: int,
  /// Upper price limit per month, in dollars.
  maxMonthlyPrice?: int
): Plan* = { <Plan name="Team" seats={teamSize} monthlyPrice=20 /> }

let refundPolicy =
  <Document:markdown title="Refund policy">
    Refunds are available within 30 days of purchase.
  </Document>

let shop = <HttpConnection name="shop" baseUrl="https://api.example.com" />

/// Looks up one order and returns its status.
let lookupOrder(
  /// The order number from the customer's confirmation email.
  orderId: string
): HttpArguments =
  <HttpArguments pathParams={ <HttpParam name="orderId" value={orderId} /> } />

/// Opens a support ticket for the customer.
let openTicket(
  /// A one-line summary of the problem.
  subject: string
): HttpArguments =
  <HttpArguments body={ <NewTicket subject={subject} priority=2 /> } />

let supportTools: Tool+ = {
  <WebSearchTool allowedDomains={ "docs.example.com" } />
  <FunctionTool function={findPlans} />
  <HttpTool connection={shop} method={HttpMethod.get}
            path="/orders/{orderId}" arguments={lookupOrder} />
  <HttpTool connection={shop} method={HttpMethod.post}
            path="/tickets" arguments={openTicket} maxCallsPerConversation=1 />
}

let supportAgent =
  <Agent:markdown name="support" model="balanced"
      documents={ refundPolicy } tools={supportTools}
      limits={ <AgentLimits maxSteps=6 /> }>
    You are the support assistant for @{companyName}.
    Answer from the documents and the documentation site. Offer a call when you cannot help.
  </Agent>

let root(): Agent = { supportAgent }
```

Evaluated by `@nx-lang/ir-runtime`, `root` is a plain record: `$type` `Agent`, `instructions` the two
dedented lines with `Example` interpolated, `documents` one `Document` whose `text` is the body, and
`tools` four records. Each function field is `{ "$type": "Function", "module": "main.nx", "name":
"findPlans" }`, the existing function value shape. `callFunction(program, tool.function, { teamSize:
4 })` returns the `Plan` list, and `callFunction` on `openTicket` returns `{ "$type":
"HttpArguments", "body": { "$type": "NewTicket", ... } }`.

A host library extends the types like this, which was compiled alongside:

```nx
import "@nx/agent"

export type RecordSearchTool extends Tool = {
  recordKind: string
  maxResults: int = 5
}

export type ChatToolContext extends ToolContext = {
  conversationId: string
  contactEmail?: string
}

export abstract external component <FlowStep id:string />
export abstract external component <AgentStep extends FlowStep
  agent?: Agent
  content prompt: string
/>
```

### Verification

The library and both examples were compiled and run on `main` (49f00fd) outside the repository, with
the built `nxlang` binary, the built `@nx-lang/sdk-wasm` module and the built `@nx-lang/ir-runtime`.
Two substitutions were needed at the time: the library was loaded as an in-memory host library under
the root `std/agent`, and the function reference type was written `object`.

After `add-function-reference-type` landed (39d9932), the source in D6 and both examples were
compiled again exactly as written, with the library as a workspace module `agent/agent.nx`. The only
substitution left is the root. The rows marked "re-run" are from that build.

| Check | Result |
|---|---|
| Library builds, with doc comments and doc links `[HttpTool]`, `[HttpArguments]` | No errors or warnings |
| §6.2: content string property plus record- and sequence-valued attributes | Works |
| §6.2: typed text on that element (`<Agent:markdown ...>`, `<Document:markdown ...>`) with `@{}` | Works; text is dedented and trimmed |
| §6.2: default on a field of an abstract type's subtype (`maxResults: int = 5`) | Works, across libraries |
| §6.2: content string on an external component that also takes `agent?: Agent` | Works (ReachMe's `AgentStep` shape) |
| A second library imports the agent library and extends `Tool` | Works |
| Extending a non-abstract `ToolContext` | **Rejected**; fixed by D7 |
| `body={ <NewTicket .../> }` at `body?: object` | Works |
| A function value at an `object`-typed field | Works; evaluates to the `Function` record |
| Re-run: library, D12 example and host library with `<function ... />: object*` and `<function ... />: HttpArguments` | No errors or warnings |
| Re-run: `function="findPlans"` | **Rejected**: "expects `<function ... />: object*`, found string" |
| Re-run: `function={ <Function module="main.nx" name="findPlans" /> }` | **Rejected**: "expects `<function ... />: object*`, found Function" |
| Re-run: `arguments={findPlans}` where `findPlans` returns `string*` | **Rejected**: "the result `string*` is not `HttpArguments`" |
| Re-run: `HttpMethod.head`, `auth="token"`, `<Tool name="x" />`, `<Agent name="support" />` | Each **rejected** as the `agent-library` scenarios say |
| Re-run: emitted library image | 7.3 KB; `nxlang ir explain` prints `requires function-reference-type-v1`. The entry image does not list it |
| Re-run: `nxlang typegen` for the library | `FunctionTool.function` and `HttpTool.arguments` are `NxFunctionRef` (TypeScript, from the helper module `_nx.ts`) and `global::NxLang.Nx.NxFunctionRef` (C#) |
| Emit entry and library images, link and evaluate in the TypeScript IR runtime, call both functions with `callFunction`, pass a `ChatToolContext` argument | Works |
| A host library exports `let surveyAgent: Agent = <Agent:markdown ...>` and an external component in another module of it declares `agent: Agent = {surveyAgent}` (ReachMe's `add-question-flow-agent-steps`) | Works in the TypeScript IR runtime: the omitted property evaluates to the library's agent. `evaluateNx()` **fails** with "Undefined variable: surveyAgent" |
| `nxlang typegen` for the library, TypeScript and C# | Works; C# warns that abstract `ToolContext` has no exported concrete descendant, which is expected |
| `evaluateNx()` (the HIR interpreter) on a `Tool+` holding a subtype declared in another module | **Fails**: "Type mismatch in record field 'tools': expected Tool, got RecordSearchTool" |

The two `evaluateNx()` failures are defects in `crates/nx-interpreter`, not in the library or in the
host library: the default-value case is the interpreter resolving a library default in the wrong
module's scope. For the last row, the type checker accepts
the program and the IR runtime evaluates it. It occurs whenever the subtype and the abstract base
are declared in different modules and the value sits in a sequence-typed record field, so it would
affect `nxlang run`, `evaluateNx()` and the playground for any program that puts a host tool type in
`Agent.tools`. `retire-hir-interpreter` replaces that interpreter with the Rust IR runtime. This
change adds the case as a source-level test and sequences after that change (see Risks).

## Dependencies on the other changes

- **`add-function-reference-type`**: the type `<function ... />: R`,
  its satisfaction rule, the rendered
  record `{ "$type": "Function", module, name }` and the required feature
  `function-reference-type-v1` (D9). Landed and archived.
- **`add-declaration-schema-export`**: nothing is needed from it, and it needs nothing changed
  here. Its mapping fits the library as written: `object` exports as the empty schema `{}`, so
  `HttpArguments.body` is any JSON value; an abstract site (`Tool`, `Connection`) exports as `anyOf`
  the concrete descendants in the program; a host names `ToolContext` in `hostSuppliedTypes` and
  every parameter typed by it or by a subtype is left out of `inputSchema`; a field of a function
  reference type is reported as not expressible, which only matters if a host asks for the schema of `FunctionTool`
  or `HttpTool` themselves; and the doc links `[HttpTool]` and `[HttpArguments]` export as code
  spans. Schemas come from the program artifact at compile time, never from the library image.
- **`add-ir-runtime-evaluation-budget`**: independent.
- **`add-agent-host-package`**: reads values of these types by the field names in D6, exactly.
  `Document`'s content field is `text`. It exports the TypeScript types generated by
  `nxlang typegen @nx/agent` from `@nx-lang/agent` (D10). It checks the result type of
  `HttpTool.arguments`. Because `ToolContext` is abstract (D7), the context record it builds must
  carry the `$type` of a concrete host subtype, including for a parameter typed `ToolContext`
  itself.
- **ReachMe `add-agent-tool-loop`**: the import and storage steps listed under D3, and
  `function-reference-type-v1` in its flow runtime's supported features.
- **ReachMe `add-agent-http-tools`**: `ChatToolContext extends ToolContext` and
  `HttpTool.maxCallsPerConversation`, both as declared in D6.
- **ReachMe `add-question-flow-agent-steps`**: `agent: Agent = {surveyAgent}` on an external
  component with a library-level `Agent` value, which was compiled here (Verification).

## Risks / Trade-offs

- **[The reserved import form shadows a real `@nx` directory]** → Documented as reserved since the
  prelude; the directory is renamed; listed as breaking in the proposal.
- **[An unstable library in a shared compiler means every host moves together]** → Hosts pin NX
  packages. ReachMe pins a compiled bundle per conversation, so running conversations are unaffected,
  and stored sources are migrated when ReachMe bumps the pin.
- **[A comment edit changes the library's version and so every bundle's library image]** → Intended
  (D4). The cost is one small image (4.5 KB stripped and 9.8 KB with its debug section, as pinned in the conformance corpus) re-stored per version.
- **[A checkout with other line endings would record a different version]** → `.gitattributes`
  already pins `.nx` files to LF, and a test pins the version of the shipped library.
- **[The HIR interpreter rejects cross-module subtypes in a sequence field, and a library default
  that names a value of another module]** → If
  `retire-hir-interpreter` has landed, the regression test runs on the IR runtime and passes. If it
  has not, the test is added for the IR runtimes and the interpreter case is recorded as a known
  failure in that change rather than fixed in code that is about to be deleted.
- **[`body?: object` cannot hold a top-level array, and admits a string or number]** → Wrap a list in
  a record. A host may narrow further: ReachMe's `add-agent-http-tools` accepts only a JSON object
  body. Revisit with the host package if a real API needs an array.
- **[A runtime that predates `function-reference-type-v1` refuses the library image]** → Intended.
  The host bumps its runtime with its compiler, and the refusal names the feature.
- **[A host-supplied `Function` record at `HttpTool.arguments` is not checked for its result by the
  runtimes]** → The checker covers every value the program produces. The host package checks the
  value an arguments function returned at each call.
- **[Typegen targets a package this change does not publish]** → `@nx-lang/agent` is created by
  `add-agent-host-package`. Until it exports the generated types, a host library's generated
  TypeScript will not resolve the import. The two changes release together.
- **[A library module of a host does not receive implicit imports]** → ReachMe writes
  `import "@nx/agent"` in its library modules. Tenant source needs no import line.

## Migration Plan

1. `add-function-reference-type` has landed.
2. Land this change. Programs that do not import `@nx/agent` are unaffected, and their emitted IR is
   byte-identical.
3. Land `add-agent-host-package`, which generates and exports the library's TypeScript types.
4. Release the packages together. ReachMe's `add-agent-tool-loop` bumps its pin, adds `@nx/agent` to
   its implicit imports and to the images it stores in a runtime bundle, and imports the library in
   its own two libraries.

Rollback is a revert: no stored format changes, and no image compiled before this change refers to a
standard library.

## Open Questions

- **A concrete `ToolContext` in the library.** With `ToolContext` abstract, a host that has nothing to
  add must still declare a subtype to fill a context parameter, and the host package cannot build a
  context record whose `$type` is `ToolContext`. Default taken: no concrete type, to
  keep the fixed type list; ReachMe declares `ChatToolContext`. A later change can add one without
  breaking anything.
- **`HttpParam.value` as `string` only.** A numeric or boolean query value must be converted in the
  arguments function. Default taken: `string`, as sketched. Whether NX has the conversions an author
  needs for this is a question for the first HTTP scenarios.
- **Design doc §9, decision 11 (opaque `model`).** Taken as given: `model` is a `string`, so the
  editor cannot complete a host's tier names.
- **Design doc §9, decision 7 (NX owns schema export).** Taken as given; this library only supplies
  the doc comments and the types that export reads.
- **Whether `unstable` should later become a compiler-visible attribute**, for example to dim
  completions. Default taken: documentation only (D5).
- **A second standard library.** The mechanism supports several, but the table holds one. Naming and
  admission rules for more are left until one is proposed.
