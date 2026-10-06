## Context

See proposal.md for motivation. The agreed design is
`reachme/docs/ReachMe-Cloudflare-AI-Agent-Architecture.md` (§5.5, §5.7, §6.4, §7.1, §7.3, §7.4); its
§6.4 gives the NX host package this scope: derive JSON Schemas, turn an `Agent` value into a
normalized, executable tool set, run function tools under the evaluation budget, build and validate
HTTP requests and call an injected `fetch`, validation diagnostics, and an AI SDK adapter.

What exists today, checked against the code:

- `@nx-lang/ir-runtime` (`runtime/typescript/src/index.ts`) exports `callFunction(program, value,
  args, options)`, which takes a `{ $type: "Function", module, name }` record and arguments keyed by
  parameter name, drops undeclared arguments, validates the rest against the declared parameter
  types at the boundary, and returns the canonical result. Failures are `NxIrRuntimeError` with
  `NxIrDiagnostic[]`. `NxRuntimeOptions` has `maxCallDepth` and `maxRangeLength` only.
- The runtime's README says the prepared types (`PreparedDeclaration` and friends) are exported for
  typing, and are not a supported way to read a program. The IR holds no doc comments.
- Canonical output renders a record as an object with a bare-name `$type`, a constant union case
  (such as `HttpMethod.get`) as the bare string `get`, and an empty optional as an empty array or
  an absent key.
- Workspace packages live under `packages/*`, build with `tsc -p tsconfig.json` to `dist/`, test
  with `node --test "dist/test/**/*.test.js"`, and are discovered for packing and publishing by
  `scripts/workspace-packages.mjs` from the workspace globs. `scripts/verify-package.mjs` installs
  the packed tarball into a scratch consumer, installs its non-`@nx-lang` peer dependencies, and
  imports every JavaScript entry point of `exports`.
- AI SDK 7 (`ai` 7.0.22 with `@ai-sdk/provider-utils` 5.0.7, as installed in ReachMe's
  `apps/conversation`) exports `dynamicTool({ description, inputSchema, execute })`, `jsonSchema()`
  (typed against `JSONSchema7`), and `ToolSet`. A tool's `execute(input, options)` receives
  `toolCallId`, `messages`, `abortSignal` and `context`. Provider tools are a separate shape
  (`type: 'provider'`, an `id` of the form `<provider>.<tool>`, `args`) that only a provider
  package constructs, for example `openai.tools.webSearch(...)`.

The package has two call sites in its first consumer, and they do not share a process:

1. **Compile time, Node (`apps/api`).** `@nx-lang/sdk-wasm` is present. The host builds the program,
   evaluates the config, and stores a JSON `normalized_config` in a runtime bundle.
2. **Run time, Cloudflare Durable Object.** No wasm compiler. The host has the stored definition and
   the linked IR program of the same bundle, and `@nx-lang/ir-runtime`.

Dependencies on the other changes in this set, with the exact identifiers taken from each:

- `add-function-reference-type` (N1): `FunctionTool.function` is typed `<function ... />: object*` and
  `HttpTool.arguments` is typed `<function ... />: HttpArguments`, and both render as the existing
  record `{ "$type": "Function", module, name }`. The compiler checks that an arguments function
  returns `HttpArguments`, so normalization does not. The runtimes do not re-check the result of
  a host-supplied record, so each call checks the `$type` of the value the function returned
  (decision 8). An
  image that uses the type requires the feature `function-reference-type-v1`, which the installed
  `@nx-lang/ir-runtime` must support; the package adds nothing for it.
- `add-declaration-schema-export` (N2): `functionSchema(ref: NxDeclarationRef, options?: {
  hostSuppliedTypes })` on `NxProgramArtifact`, answering `NxFunctionSchema` with `module`, `name`,
  `description`, `summary`, `parameters` (`name`, `type` in NX spelling, `required`, `description`,
  `typeRef`, `hostSupplied`), `inputSchema`, `outputSchema`, `resultType` (NX spelling),
  `declaration` (an `NxTextSpan` in the module's source) and `diagnostics`. An
  absent schema comes with a `schema-inexpressible-type` diagnostic; an unknown function throws
  with `schema-unknown-declaration`. Host-supplied matching covers a listed type and every record
  that extends it, and does not cover a listed type under an occurrence. As landed, `typeRef` is
  answered only for a parameter declared with a bare or applied type name: under `+` it is absent,
  as `hostSupplied` is, so nothing in the answer said that a parameter holds a listed type without
  being one. This change adds that to the export (decision 13). Schemas exist only at compile
  time; nothing is derivable from the IR image.
- `add-ir-runtime-evaluation-budget` (N3): `NxRuntimeOptions.maxOperations`, opt-in and unlimited
  when unset; failure code `nx-ir-resource-limit` with `limit: { name: "maxOperations", value }`.
  One budget per public API call, no carry-over and no usage reporting.
- `add-ir-runtime-input-limit-and-cost-tests`, stage 1 (N5): `NxRuntimeOptions.maxInputSize`,
  opt-in and unlimited when unset, measured over everything the host passes to one call and
  failing with `nx-ir-resource-limit` and `limit: { name: "maxInputSize", value }` before
  anything is evaluated; `measureInputSize(value, limit?)`, which measures one value the same
  way; and `NxRuntimeOptions.usage`, a sink for the operations a call used.
- `add-agent-library` (N4): the standard library `@nx/agent`, one module `@nx/agent/agent.nx`,
  exporting `Agent`, `Document` (`title`, content `text`), `AgentLimits`, abstract `Tool`,
  `FunctionTool`, `WebSearchTool`, abstract `ToolContext` (`callId`), abstract `Connection`,
  `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments` (`pathParams`, `query`,
  `body:object`) and `HttpTool`. `nxlang typegen` imports these types from `@nx-lang/agent` in
  TypeScript, so this package publishes them (decision 12).
- ReachMe's `add-agent-function-tools` (R2) and `add-agent-http-tools` (R3) consume the package.
  `add-agent-tool-loop` (R1) owns the model loop, the journal and the call identifier
  (`<turnId>:<step>:<call>`), which the package receives as `callId` and never generates. R1's
  resolved tool entry shape (`name`, `title?`, `description`, `inputSchema`, `outputSchema`,
  `annotations`, `kind`, `config`) is the shape decision 4 uses.

Other active NX changes (`add-go-to-definition`, `add-ir-runtime-performance-harness`,
`infer-unannotated-return-types`, `retire-hir-interpreter`,
`support-stateful-component-value-evaluation`) do not touch `packages/agent` or the
`package-release-automation` requirement this change modifies. This change must land after N1 to N4
and has no ordering constraint with those five.

## Goals / Non-Goals

**Goals:**

- One function per half: `normalizeAgent` at compile time, `createAgentTools` at run time, with a
  JSON definition as the only thing that crosses between them, in the tool entry shape the first
  consumer stores.
- The run-time half needs nothing but `@nx-lang/ir-runtime`, the definition and the linked program.
- A model-supplied value can never change where an HTTP tool's request goes or what method it uses.
- A host can add tool types without forking the package.
- Failures are data: diagnostics at compile time, result values at run time.

**Non-Goals:**

- The model loop, streaming, structured output, message history, step and tool-call limits.
- Network policy: DNS and address filtering, HTTPS-only egress enforcement at call time, redirects,
  timeouts, response size caps, retries, credentials, idempotency headers, rate limits and
  `maxCallsPerConversation` enforcement.
- Persistence, journaling and replay of tool results.
- Size guards on instructions and documents, and validation of `model` (host policy).
- A Rust or .NET twin of the package.
- MCP or OpenAPI connections, human approval, subagents (future doc).
- Constrained types, such as an `int` between 1 and 10, and other more specialized types. They are
  a later change to the language. This design is written so that they need none here (decision 14).

## Decisions

### 1. A new package under `packages/agent`, not a module of the runtime

`@nx-lang/agent` is a workspace package with `@nx-lang/ir-runtime` as a `workspace:*` dependency.
The design doc settles that agent support sits on the runtime rather than in it, because the runtime
is pure and has a Rust twin. `packages/agent` matches where every other TypeScript package except the
runtime and the bindings lives, so the pack, verify and publish scripts pick it up from the existing
`packages/*` glob with no script change; only the spec's package list and `docs/deployment.md` name
packages one by one.

### 2. Four entry points, all Worker-safe

| Entry point | Holds | Imports |
| --- | --- | --- |
| `@nx-lang/agent` | The generated `@nx/agent` types, definition types, `AGENT_DEFINITION_FORMAT_VERSION`, `NX_AGENT_TOOL_CONTEXT`, `toSnakeCase`, `buildHttpRequest`, `NxAgentError`, `NxAgentToolError`, diagnostic types | nothing at run time |
| `@nx-lang/agent/normalize` | `normalizeAgent`, describer types, the `AgentSchemaSource` interface | the root entry |
| `@nx-lang/agent/execute` | `createAgentTools`, `checkAgentTools`, `evaluateHttpArguments`, executor types | the root entry, `@nx-lang/ir-runtime` |
| `@nx-lang/agent/ai-sdk` | `toAiSdkTools` | the root entry, `ai` |

All four are free of Node built-ins, wasm and `@nx-lang/sdk-wasm`, so all four can be imported in a
Cloudflare Worker. A Durable Object imports `/execute` (and `/ai-sdk` if it uses the adapter). The
compile-time host imports `/normalize`. `/normalize` is Worker-safe as code but only useful where a
schema source exists, which in practice is where the compiler runs.

`buildHttpRequest` is in the root entry because a host that makes HTTP calls in a different process
from the one that evaluates arguments (ReachMe's `apps/api`) needs it without the runtime.

Alternative considered: one entry point with tree-shaking. Rejected because a Worker bundle check
should not depend on a bundler's analysis, and because `ai` must not be imported by hosts that do
not install it.

### 3. Schemas come from the compiler at compile time and are stored

`normalizeAgent(agentValue, options)` takes `options.schemas`, an `AgentSchemaSource`:

```ts
interface AgentSchemaSource {
  functionSchema(
    ref: { module?: string; name: string },
    options?: { hostSuppliedTypes?: readonly { module?: string; name: string }[] },
  ): AgentFunctionSchema;   // the fields of N2's NxFunctionSchema that the package reads
}
```

This is N2's `functionSchema` signature, so a host passes the `NxProgramArtifact` itself, from
`@nx-lang/sdk-wasm` or `@nx-lang/sdk-node`. The package declares the interface structurally and does
not depend on either SDK, which keeps `/normalize` free of the wasm module. A host that queried the
schemas earlier and no longer holds the artifact passes an object that answers from what it kept.
The package always asks with `hostSuppliedTypes: [NX_AGENT_TOOL_CONTEXT]`, exported as
`{ module: "@nx/agent/agent.nx", name: "ToolContext" }` so such a host asks the same way. Tests use
`@nx-lang/sdk-wasm` as a dev dependency.

What the package takes from an answer: `inputSchema` and `outputSchema` unchanged, `description` as
the default tool description, `parameters[].hostSupplied`, `.hostSuppliedWithin` and `.typeRef` for
context parameters, `declaration` for diagnostics, and `diagnostics`. It reads no other field, and
of a parameter's type it reads only those three: never the spelling in `type`, and never a
schema. It does not rebuild or merge schemas:
N2 already leaves host-supplied parameters out and sets `required` and `additionalProperties`.
`typeSchema` is not used.

A source that throws an error whose diagnostics include `schema-unknown-declaration` becomes
`nx-agent-unknown-function`. Any other throw (a disposed artifact, a crashed wasm host) propagates,
because it is not a property of the agent.

The run-time half never derives a schema. The stored definition carries `inputSchema` and
`outputSchema`, which is what the design doc's §7.3 asks for ("the Durable Object reads ready-made
definitions from the pinned bundle; it never reflects over NX at run time") and what N2 establishes
is the only possibility.

Alternative considered: derive schemas at run time from `PreparedDeclaration`. Rejected: the image
erases generic arguments, has no doc comments, and records a result type only when declared.

### 4. The definition format

```ts
interface NormalizedAgent {
  formatVersion: 1;
  name: string;
  description?: string;
  model?: string;
  instructions: string;
  documents: { title: string; text: string }[];
  limits: { maxSteps?: number; maxToolCalls?: number };
  tools: NormalizedTool[];
}

interface NormalizedToolBase {
  name: string;
  title?: string;
  description: string;
  inputSchema: JsonSchema;
  outputSchema?: JsonSchema;
  annotations: { readOnlyHint: boolean; destructiveHint: boolean; idempotentHint: boolean; openWorldHint: boolean };
}

type NormalizedTool =
  | NormalizedToolBase & { kind: "function"; config: FunctionToolConfig }
  | NormalizedToolBase & { kind: "http"; config: HttpToolConfig }
  | NormalizedToolBase & { kind: "provider"; config: { provider: "web_search"; allowedDomains?: string[] } }
  | NormalizedToolBase & { kind: string; config: JsonObject };          // host kinds

interface FunctionToolConfig {
  function: { module: string; name: string };
  contextParameters: { name: string; type: { module: string; name: string } }[];   // N2's typeRef
}

interface HttpToolConfig {
  arguments: { module: string; name: string };
  contextParameters: { name: string; type: { module: string; name: string } }[];
  connection: { name: string; baseUrl: string };          // trailing slash removed
  method: "get" | "post" | "put" | "patch" | "delete";
  path: string;
  pathPlaceholders: string[];
  maxCallsPerConversation?: number;
}
```

Every kind keeps its executor data under `config`. That is the tool entry shape ReachMe's
`add-agent-tool-loop` stores in `resolved_agents_json`, so a host stores the package's tool
definitions without reshaping them, and a host kind such as ReachMe's `builtin` has the same layout
as the package's own. The agent-level fields are the package's; a host that stores more per agent
(a use-site `ref`, content hashes, a resolved model) wraps the definition.

`formatVersion` exists because the definition is stored and read later by a possibly newer package;
`createAgentTools` refuses a version it does not know rather than guessing. While the package is
unstable the version may change in any release.

`limits` and `model` are copied as authored. Capping limits and resolving the model are host policy.

**No output wrapping.** `outputSchema` is N2's output schema as answered and the output is the
function's canonical result as `callFunction` returns it: an array for `T*`, `null` for an empty
standalone `T?`, a bare string for a constant case. MCP `2026-07-28` allows `outputSchema` to be any
JSON Schema 2020-12 document, and the AI SDK takes any JSON value as a tool result, so nothing
requires an object. Wrapping would also break N2's guarantee that every returned value is valid
against the output schema. No wrapping remains for any other reason.

The design doc's §5.5 sketches `kind: 'function' | 'builtin' | 'provider'`. This design uses
`function`, `http` and `provider` for the package's own tools and leaves `builtin` (or any other
string) to host describers, because §7.1 distinguishes four runtime kinds and `http` needs its own
executor and its own stored data.

### 5. Names and descriptions

Default names follow the design doc: the function's name in snake_case for `FunctionTool`,
`web_search` for `WebSearchTool`. The doc does not say what an `HttpTool` defaults to; this design
uses the arguments function's name (`lookupOrder` becomes `lookup_order`) and its doc comment,
because §5.7 says the input schema "comes from the function, exactly as for `FunctionTool`" and its
example documents `lookupOrder` as the operation.

The conversion handles camelCase, PascalCase (element-style functions such as `PlanRow`) and
acronym runs (`HTTPStatusFor` becomes `http_status_for`), and is exported as `toSnakeCase` so a host
describer names its tools the same way.

Names must match `^[a-z][a-z0-9_]{0,63}$`. That is inside what OpenAI and Anthropic accept for
function names (`[a-zA-Z0-9_-]`, 64 characters) and inside MCP's recommendation. An authored name
that does not match is rejected rather than converted, so the name in the config is the name the
model sees and the name in traces. The pattern leaves `__` available for the future
`<connection>__<tool>` convention.

A missing description is an error, as §7.3 requires. A missing parameter description is not
reported: the design doc does not ask for it, and N2 supplies it when the author wrote one.

### 6. `ToolContext` parameters

`ToolContext` is abstract in N4 (the compiler allows extending only abstract records), and `@nx/agent`
declares no concrete subtype. A value passed to a context parameter must therefore be a record of a
concrete subtype the host declares, such as ReachMe's `ChatToolContext`, and only the host knows
which.

Compile time: N2 reports each parameter typed `ToolContext` or a subtype as `hostSupplied` and
leaves it out of `inputSchema`. The package stores its name and its `typeRef`, the declared type
by module and name, in `config`. `normalizeAgent` takes `options.toolContextType`, the
`{ module, name }` of the host's concrete subtype, and rejects a context parameter whose `typeRef`
is neither `ToolContext` nor that type, since no record the host supplies could satisfy it. With no
`toolContextType`, any context parameter is an error: the host has said it supplies no context.

A context type can also reach the model without being a parameter's type: under an occurrence
(`contexts:ChatToolContext+`), or as a field of a record the function takes. N2 does not treat
such a parameter as host-supplied, rightly, since the host cannot fill part of a value, so its
schema, `callId` and the host's fields included, would be put to the model. The export reports
this as `hostSuppliedWithin` on the parameter's entry (decision 13), and the package rejects any
parameter that carries it, naming the tool and the parameter. An author declares the context as a
parameter of its own.

The package decides all of this from three things the export answers for a parameter:
`hostSupplied`, `hostSuppliedWithin` and `typeRef`. It never parses the NX spelling in `type` and
never looks inside a schema. Which types are context types, which extend them and what a type
holds are questions about the program, and only the compiler can answer them by identity.

Run time: the call context carries `context`, the host's record with its `$type` and fields. The
package sets `callId` on a copy and passes it to every context parameter. It checks only that a
record with a `$type` was supplied when a tool needs one (`invalid-context`).

A key in the model's input that matches a context parameter's name is deleted before the call. This
is defence in depth: the input schema already has `additionalProperties: false`, but the package
does not validate against the schema (decision 7) and must not let a model-supplied identity
through.

Alternative considered: have the package build the record from loose values and a stored field
list. Rejected: it needs `typeSchema` of an abstract type at compile time and a second copy of the
type's shape in the definition, to save the host from writing `$type` once.

### 7. Execution, validation and results

`createAgentTools(definition, program, { runtime?, maxArgumentsSize?, maxContextSize?, request?, executors? })` returns `AgentTool[]`:
the definition's fields plus `execute(input, { callId, context?, signal? })`.

- **No JSON Schema validator.** `callFunction` validates and normalizes arguments against the
  declared parameter types, and N2 guarantees that a value valid against the input schema is
  accepted at the boundary. A second validator in the package would be a dependency in the Worker
  bundle and a second source of truth. The package checks only that the input is a JSON object.
- **Budget.** Three layers, which do not conflict. The runtime has no default: `maxOperations` is
  opt-in and unlimited when unset (N3). This package applies `NX_AGENT_DEFAULT_MAX_OPERATIONS`,
  100,000, when the host's `runtime` options do not set `maxOperations`, because it exists to run
  code its host did not write. The number is low on purpose. A realistic tool costs hundreds to a
  few thousand operations, its arguments checked and its result written included, so 100,000 is
  ample headroom. N3 charges every walk over a value (type checks, equality, the result written
  for the host) as well as every node, so that the time and memory of a call are proportional to
  its count: for a tool whose arguments are typed, 100,000 operations is a few milliseconds,
  during which a single-threaded Durable Object serves no other request. N3's own documents say
  that this bound is the intent of its rules and has been broken in review more than once through
  large values passed at `object`, so a host should keep its own limit as well; the Worker CPU
  limit is the one a Durable Object has. Raising a default that a tool outgrows
  is easy, since it fails with a `limit` that says so; lowering one that tools rely on breaks
  them. A host passes its own number when it has one; ReachMe passes 200,000
  per call (R2). A host cannot turn the budget off through this package. Each `execute` is one
  `callFunction` call and so gets the whole budget; N3 offers no shared budget, and the package
  adds none. N5 lets a host read what a call used, and the package puts it on the result (see
  *Results are values*).
- **Input.** The package bounds what goes into a call in two parts, because two parties supply
  it. The model's input object is measured with `measureInputSize` before the call and held to
  `maxArgumentsSize`, `NX_AGENT_DEFAULT_MAX_ARGUMENTS_SIZE`, 1,000, by default: a tool's
  arguments are tens to hundreds of values, and N3's review found every remaining way to make a
  cheap step do unbounded work needed a large value passed where no type checks it. Input over
  the cap never reaches the runtime. The host's context record has its own allowance,
  `maxContextSize`, `NX_AGENT_DEFAULT_MAX_CONTEXT_SIZE`, 1,000, by default. N5's task 6.1 set
  that number from a measured context, as four times its size, rounded up to a thousand, and
  1,000 at the least: the `ChatToolContext` record of the `agent-tool-context` corpus program,
  with `callId` set and its optional `contactEmail` supplied, measures 4 with `measureInputSize`
  (the record and its three strings), so the least applies. The allowance is separate because
  under one shared number a large context would eat the model's allowance and the model would be
  told its input was too big. The package measures the context too, as it is passed with `callId`
  set, and a context over its allowance fails before the call. Both parts being held, the call
  runs with `maxInputSize` set by the package to what a call within them can reach: the arguments
  cap; for each context parameter of the function, the context allowance (the same record goes
  to each) and what its name costs as a field name of the arguments record, one for every 64
  code units; and the size of the function record, all known at creation. That limit is a
  backstop a call within both checks cannot pass. A `maxInputSize` in the host's `runtime`
  options is not used for these calls: a smaller one would refuse input the package accepted and
  report the model's mistake as a resource limit. A host raises or lowers the limits through the
  two options, and cannot turn either off.
- **Results are values.** `execute` resolves to `{ ok: true, output, usage? }` or
  `{ ok: false, error: { code, message, diagnostics?, limit? }, usage? }`. `usage` is
  `{ operations, inputSize }`, what the runtime reports for the call (N5), and is present whenever
  the call reached the runtime: on a success, and on a failure of evaluation or of a limit, where
  it says how far the call got. A host that journals tool calls stores it with the result, logs
  what its tools cost, and chooses a budget from that. The package gives every call a sink of its
  own. The host's runtime options are passed through, but a `usage` in them would be shared by
  calls a model makes together, and after the first is awaited it would hold the numbers of
  whichever ended last; so the package replaces it and does not write to the host's. A failure
  the package finds before it calls the runtime (input that is not an object or is too large, a
  missing or oversized context, an abort) has no `usage`; one the runtime finds, a wrong-typed
  argument included, has it. For an HTTP tool the numbers are those of its arguments function,
  and a failure after that call, `invalid-request` or `request-failed`, carries them too; a host
  executor's or a provider's tool has none. The AI SDK adapter returns the
  output alone to the SDK and hands the whole result, `usage` included, to the host's `onResult`
  (decision 10). Codes: `invalid-input`,
  `invalid-context`, `evaluation-failed`, `resource-limit`, `invalid-request`, `request-failed`,
  `aborted`, or a code a host function threw in an `NxAgentToolError`. A host that journals tool
  calls stores the result object as is. Throwing is reserved for configuration errors at
  `createAgentTools` time (`NxAgentError`).
- **The package classifies runtime failures so hosts do not parse IR codes.** An
  `NxIrRuntimeError` whose diagnostics are all boundary failures is `invalid-input`: the model sent
  arguments the function's parameters do not admit and can correct them. A boundary failure is
  `nx-ir-arguments` or a code that begins `nx-ir-boundary-`, which today is `nx-ir-boundary-type`
  and `nx-ir-boundary-field`. The family is matched, not the two names, so that a check a runtime
  adds at the boundary later, a value outside a constrained type for one, reaches the model as
  input to correct and not as a failure of the tool (decision 14). Input over `maxArgumentsSize` is `invalid-input` too, found by the package before the
  call, with `limit: { name: "maxArgumentsSize", value }` and no diagnostics: the model sent it
  and can send less, where `resource-limit` would tell it the tool ran too long. A context over
  `maxContextSize` is `resource-limit`, also found before the call, with
  `limit: { name: "maxContextSize", value }`: the host supplied it. Any
  `nx-ir-resource-limit` diagnostic makes it `resource-limit`, with the diagnostic's
  `limit` copied to the error (`limit.name` is `maxOperations` for the budget, another name for
  call depth, nesting or range length). Everything else is `evaluation-failed`.
- **Evaluation is synchronous.** The abort signal is checked before evaluation and passed to the
  HTTP request function and host executors. A running evaluation is bounded by the budget, not by
  the signal.
- **Functions are checked against the program at creation.** For each `function` and `http` tool,
  `createAgentTools` looks the named module up in `program.modulesByIdentity` and the name in that
  module's `declarationsByName`, and requires a function declaration. A miss throws
  `NxAgentError` with `nx-agent-unknown-function`; a missing tool is never silently dropped and
  never left to fail at its first call. This is the package's one read of the runtime's prepared
  types, limited to two exported maps and the declaration's kind tag.
- **`checkAgentTools(definition, program)`** runs the format-version and function checks alone and
  returns diagnostics. A compile-time host calls it after linking, when it has no executors and no
  request function (R2's bind check).
- **Misuse of the options is a plain error, not an `NxAgentError`.** The list of diagnostic codes
  is fixed and has none for these, and neither is a property of the definition: an executor
  registered for `function`, `http` or `provider` is a `TypeError`, since ignoring it would hide a
  host's mistake, and a limit (`maxArgumentsSize`, `maxContextSize`, `runtime.maxOperations`) that
  is not a non-negative safe integer is a `RangeError`, which is what keeps `Infinity` from turning
  one off. Both are thrown at creation.
- **A host executor's own error** that is not an `NxAgentToolError` fails the call as
  `evaluation-failed`, the nearest of the seven result codes.
- **A member of an options object, or of a call context, may be `undefined`** as well as absent, so
  a host passes on a value it may not have. What the package produces keeps the rule: absent,
  never `undefined` or `null`.

### 8. HTTP: a pure builder plus an injected request function

`buildHttpRequest(tool, httpArguments)` is pure and returns
`{ ok: true, request: { method, url, headers, body? } }` or a structured failure. The rules are in
the spec. The reasoning behind the less obvious ones:

- **Percent-encode everything outside the unreserved set**, for path values and for query names and
  values. This is `encodeURIComponent` plus `! ' ( ) *`. A value therefore cannot contain `/`, `?`,
  `#`, `&` or `=` in the output.
- **Reject `.` and `..` segments after substitution.** They are unreserved characters, so encoding
  leaves them alone, and a URL parser would then remove them and walk out of the fixed path.
- **Compare the parsed URL with what was built.** After building, the package parses the string
  with the standard `URL` and requires that `href` is the string it built, that `origin` equals
  the base URL's, that there is no user information, that `pathname` is the base path followed by
  the path built, that `search` is the query built, and that there is no fragment. If any of it
  differs, something got through that the rules did not anticipate, and the request is refused.
  This is the structural form of "the final URL must stay under `baseUrl`". The parts are
  compared, and not the text and the prefix alone as first designed, because a `?` or a `#` that
  should have been encoded leaves the text exactly as it was and ends the path early: with the
  encoder broken, `/accounts/{a}/orders` and a value of `X?` would have been requested as
  `/accounts/X`. The request's URL is the one this check answers, so the check cannot be removed
  and a URL still come out. No input reaches it with something to refuse while the other rules
  hold, so it is tested on its own, one condition at a time.
- **A base URL is written the way a parser writes it.** Because the built text must survive
  parsing unchanged, a base URL the parser itself would rewrite (`https://API.example.com`, a
  default port, a dot segment in its path) would fail every call. Normalization refuses it, naming
  the spelling to write, so the config compile fails and not the first call.
- **Dot segments in every spelling.** A URL parser reads `%2e` as a dot, so `.%2E` walks up a
  directory as `..` does. The path template refuses a literal segment in any of those spellings,
  and the check after substitution reads a literal `%2e` beside a value the same way. A value's
  own `%2e` is data: its percent sign is encoded.
- **`$type` is removed from the body.** `HttpArguments.body` is typed `object` (N4), so it is any
  one value, usually a record the program declares, and its canonical form carries `$type` at every
  record. A third-party API does not expect NX discriminators. The cost is that a payload union
  case in a body loses its tag; see Open Questions.
- **A body is refused on `get` and `delete`.** `post`, `put` and `patch` carry one.
- **Headers.** Only `content-type: application/json` when there is a body. `Idempotency-Key`,
  `Authorization`, `Accept` and anything else are the host's.

Static checks on `baseUrl` and the path template happen in `normalizeAgent`, so a bad connection
fails the config compile, not the first call. `https` is required there because the design doc's
`HttpConnection` is "https only" and N4 leaves `baseUrl` checks to the host package; `allowInsecureBaseUrl` exists so a host's tests can point
at a local server. The scheme is checked again by the builder's origin comparison only in the sense
that it cannot change; enforcing HTTPS at egress remains host policy.

The injected function is not `fetch` itself:

```ts
type AgentHttpRequestFunction = (call: {
  request: { method: string; url: string; headers: Record<string, string>; body?: string };
  arguments: { pathParams: { name: string; value: string }[]; query: { name: string; value: string }[]; body?: JsonValue };  // as evaluated; body keeps `$type`
  callId: string;
  toolName: string;
  connection: string;
  annotations: ToolAnnotations;
  signal?: AbortSignal;
}) => Promise<{ status: number; body: JsonValue; [key: string]: JsonValue }>;
```

The design doc says "calls a `fetch` the host passes in". A raw `fetch` signature would force the
package to decide how to read the response (size, content type, streaming), which is policy, and
would give the host no `callId`. With this shape a simple host wraps `fetch` in a few lines (the
README shows it), and ReachMe's Durable Object forwards `toolName`, `arguments` and `callId` to
`apps/api`, which loads the tool definition from the bundle and calls `buildHttpRequest` itself
(§7.4: the API "does not trust configuration sent by the Worker"). Both processes run the same
builder over the same stored definition.

`evaluateHttpArguments(tool, program, input, { callId, context? }, { runtime?, maxArgumentsSize?, maxContextSize? })` is exported from
`/execute` and does the first two steps of `execute` only: it evaluates the arguments function and
runs the builder, and returns `{ ok: true, arguments, request }` or the failure. It takes no request
function and sends nothing. R3 uses it for its compile-time sample evaluation, which compares the
returned path parameter names with `config.pathPlaceholders`.

Normalization does not check the arguments function's result type. N1 makes the compiler check it:
`HttpTool.arguments` is `<function ... />: HttpArguments`, so a program cannot bind a function of
another result there, including one that returns its own record named `HttpArguments`. What is
left is an `Agent` value that did not come from evaluating a checked program, or a stored
definition run against a program that has since changed. Both are caught where the value is used:
`evaluateHttpArguments` fails with `invalid-request`, naming the function, when the value the
function returned is not a record whose `$type` is `HttpArguments`, before the builder reads it. A
normalization check on N2's `resultType` spelling would cover only the first case, and less
exactly than the compiler does.

A non-2xx status is a successful tool call whose output says so. The output is the request
function's whole result, so a host can add fields such as R3's `bodyNote`; the stored
`outputSchema` requires `status` and leaves the object open. Deciding that a timeout is an
"unknown outcome" is the host's: it throws `NxAgentToolError` with its own code and the package
passes it through.

### 9. Extension point

Compile time: `options.toolTypes`, a map from a `Tool` subtype's `$type` to a describer
`(value, context) => ToolDescription | ToolDescription[] | { diagnostics }`. Returning an array
covers the design doc's `MeetingSchedulerTool`, which resolves to two model-facing tools. The
context gives the describer `schemas`, `toSnakeCase` and the tool's path for diagnostics.

Run time: `options.executors`, a map from `kind` to
`(tool, input, context) => Promise<JsonValue>`.

The package's own types and kinds cannot be overridden through either map. A host that wants to
intercept every call (journaling, tracing, limits) wraps `execute` on the returned tools, which are
plain objects.

Tool values are matched by their bare `$type`, because that is all the canonical encoding carries.
Two libraries that each declare a record named `FunctionTool` cannot both be used in one agent; see
Risks.

### 10. AI SDK adapter

`toAiSdkTools(tools, options)` returns a `ToolSet`:

- An executable tool becomes `dynamicTool({ description, inputSchema: jsonSchema(tool.inputSchema),
  execute })`. `dynamicTool` is the SDK's form for tools not known at development time, and avoids
  inventing static input and output types. The schema is passed to `jsonSchema` with a cast: the
  SDK types it as `JSONSchema7`, the package produces draft 2020-12, and the SDK forwards the
  object to the provider without interpreting it.
- `execute` maps `options.toolCallId` to `callId` unless the host gives `callId(info)`. The design
  doc requires `callId` to be stable across retries; a provider's tool call id is not stable across
  a re-run of the model step, so a host with retries supplies its own (ReachMe's is
  `<turnId>:<step>:<call>`). `options.context(info)` supplies the tool context record.
- A failure is thrown as `NxAgentToolError`, which the SDK turns into a tool error the model sees.
  What the model sees of it depends on whose mistake it was. `invalid-input` is the model's, so
  the error carries the failure's message, diagnostics and limit, and the model can correct the
  call. The package's other codes (`invalid-context`, `evaluation-failed`, `resource-limit`,
  `invalid-request`, `request-failed`, `aborted`) are the host's, the author's or the network's:
  the model can do nothing with the reason, and the reason can hold what it should not be shown,
  such as the text a request function threw, the runtime's account of a function, or a
  connection's base URL, which the library promises the model never sees. For those the error
  carries a fixed sentence that names the tool and the kind of failure. A code a host function
  chose keeps the host's message, since the host wrote it for this. The host reads every failure
  whole in `onResult`, and can throw from there to tell the model something else. What the model
  is sent, in `ai` 7.0.127, is the thrown error's name and message and nothing else of it (for a
  tool the host runs the SDK calls `getErrorMessage`, which is `toString()`), so the message is
  the whole of what this decides; the diagnostics and limit on an `invalid-input` error are for
  the host's own handlers. A host that throws its own `NxAgentToolError` is therefore writing to
  the model, and the README's request function shows it done with a fixed sentence and the caught
  error logged, not `String(error)`. The `request-failed` sentence says the tool "did not get an
  answer", not that the request did not happen, since a write that timed out may have. One cost is
  accepted: an `invalid-request` the model caused, an empty path parameter or a `..` segment,
  is reported to it in general terms ("could not make a request from these arguments"). Telling
  those apart would need the builder's rule on the result, and a host that wants it has
  `onResult`.
- `options.onResult({ name, callId, result })` is called once for every executable tool call, with
  the whole result as `execute` resolved it, before the output is returned or the error thrown.
  The SDK is given the output alone, and a thrown tool error has no place for numbers, so without
  this a host on the adapter, which is the usual way to run tools, would never see a call's
  `usage`. It is also where such a host journals a result, which in a Durable Object is
  asynchronous, so the adapter awaits what the function returns before it hands the output to
  the SDK: unawaited, the model would go on before the journal was written, and a rejection
  would be caught by nobody. The adapter does not catch what the function throws, and a rejected
  promise is a throw: it is the host's code, and the SDK reports it as the tool's error.
- A `provider` tool is mapped by `options.providerTool(tool)`, which returns the provider package's
  tool (for example `openai.tools.webSearch({ filters: { allowedDomains } })`) or `null`. The
  package cannot construct provider tools without depending on a provider package, and silently
  dropping web search would be worse than an error.

The functions a host gives for `callId` and `context` are passed `{ toolName, toolCallId, input,
options }`, the last being the SDK's own execution options. A provider tool with no mapping is a
`TypeError`, for the reason decision 7 gives. Checked against `ai` 7.0.127 with
`@ai-sdk/provider-utils` 5.0.53 when the adapter was written: the four names are where this
decision says, and the tests run the tool set through both `generateText` and `streamText`.

`ai` is declared as `peerDependencies: { "ai": "^7.0.0" }` with `peerDependenciesMeta.ai.optional`.
`verify-package.mjs` installs peers into its probe consumer, so the `/ai-sdk` entry point is
exercised by the existing packaging check.

### 11. Diagnostics

Normalization diagnostics reuse the runtime's `severity`, `code`, `message` fields and add `path`
and `tool`. Codes: `nx-agent-not-an-agent`, `nx-agent-duplicate-tool-name`,
`nx-agent-invalid-tool-name`, `nx-agent-missing-description`, `nx-agent-inexpressible-parameter`,
`nx-agent-inexpressible-result` (warning), `nx-agent-unknown-function` (also thrown by
`createAgentTools`), `nx-agent-unsupported-format`, `nx-agent-invalid-definition` (a stored
definition that does not have the shape the package wrote, reported by `checkAgentTools` and
`createAgentTools` with the path of the member, so a damaged definition is a diagnostic and never a
`TypeError`), `nx-agent-context-parameter`, `nx-agent-unknown-tool-type`, `nx-agent-http-base-url`,
`nx-agent-http-path`, `nx-agent-http-arguments-type`, `nx-agent-duplicate-document-title`,
`nx-agent-describer`, and at creation `nx-agent-missing-executor` and
`nx-agent-missing-request-function`. Diagnostics from N2 for an inexpressible type are wrapped, keeping N2's
message as the reason. Every diagnostic about a tool's function carries `declaration`, copied from N2's
answer: the function's module identity and the `NxTextSpan` of its declaration in that module's
source, so a host can point an editor at the function. N2 leaves the span out only when the
artifact holds no source for the module; the diagnostic then has `path` and `tool` only.

Two uses the requirements leave open. `nx-agent-http-arguments-type` is in the list and, with the
result type checked by the compiler (decision 8), nothing reports it; it is used for an `HttpTool`
whose `arguments` field is not a `Function` record, as `nx-agent-unknown-function` is for a
`FunctionTool` whose `function` field is not one. And when N2 answers no input schema, every one of
its diagnostics becomes an `nx-agent-inexpressible-parameter` error, the result's among them when
there is no output schema either: the answer does not say which diagnostic is about what, and the
package does not read messages to find out. An `Agent` value that a checked program could not have
produced is reported with the nearest code and never thrown on.

Duplicate document titles are checked here because the design doc's §5.6 states the rule as a
property of an agent, not of ReachMe. The size guard in the same section is a host limit and is not.

### 12. The package publishes the library's TypeScript types

N4's `typegen` makes a host library that references `Tool` or `ToolContext` import them from
`@nx-lang/agent`. The root entry therefore exports the types `nxlang typegen @nx/agent` generates,
under their NX names (`Agent`, `Document`, `AgentLimits`, `Tool`, `FunctionTool`, `WebSearchTool`,
`ToolContext`, `Connection`, `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments`,
`HttpTool`). The generated files are checked in at `packages/agent/src/generated/` and a test
regenerates them with the CLI and fails on any difference, the way the prelude image is pinned.
One change is made to the generator's output, by the script that writes the files and that the
test runs: each relative import gains `.js`. As generated, a consumer compiled with
`moduleResolution: nodenext` could not import the types at all, because Node's resolution wants
the extension; a bundler's does not. That `nxlang typegen` writes extensionless imports is the
generator's and is left alone here. The
package's own types are named so they do not collide (`NormalizedAgent`, `NormalizedTool`,
`AgentTool`). `normalizeAgent`'s parameter is typed with the generated `Agent`.

### 13. The export says where a host-supplied type is held

The parameter entry of N2's `functionSchema` gains one optional member:

```ts
interface NxParameterSchema {
  // ...
  hostSupplied?: NxDeclarationName;          // as landed: the parameter is one, and is left out
  hostSuppliedWithin?: NxDeclarationName[];  // new: its type holds these, and it is not left out
}
```

`hostSuppliedWithin` names each listed type that the parameter's type reaches without being one:
the listed type, or a record that extends it, found under an occurrence, as a field of a record or
of a union case, through a type alias or as a type argument, at any depth. It is absent when there
is none, and on a parameter that is itself `hostSupplied`. Nothing else about the answer changes:
such a parameter stays in the input schema, as N2 already requires, because whether a held context
is an error is the consumer's to say. This package says it is.

It is computed in `crates/nx-api/src/schema.rs` by a walk over the parameter's resolved type, the
`Shape` the schema is written from, with a set of the declarations it has entered so that a record
that holds itself ends. It is a walk of its own, not a by-product of writing the schema: the
schema writer describes each declaration once and refers to it afterwards, so a second parameter
of the same record would never be looked into.

Alternatives considered:

- *Answer `typeRef` under an occurrence.* Ten lines, and it covers a list only, not a field. It
  also changes what `typeRef` means, since a caller could no longer tell `T` from `T+`.
- *Compare in the package*, by the spelling in `type` or by the keys of `$defs`. Both are names,
  not identities: a key is a bare name with a fallback on a collision, and a spelling has to be
  parsed again by every consumer and for every form of type the language gains.
- *Have the export refuse the function*, answering no input schema. Rejected: N2 describes a
  program and leaves policy to its consumers, and another consumer may want such a parameter.

A type derived from a listed type is not that type. Every record has a derived `.Update` record
and `.Property` union, and a parameter declared `ChatToolContext.Update` is neither host-supplied
nor counted as holding a context: it stays in the input schema and normalizes with no diagnostic.
The rule exists because an author who writes a context expects the host to fill it, and the host
cannot fill part of a value. No one can expect that of a patch. The host has no
`ChatToolContext.Update` to supply, so nothing it supplies is asked of the model, and a function
that takes one from the model asks for it as plainly as one with a `conversationId:string`
parameter does. `Request.Update`, where `Request` has a context field, is still named, because
the field's value is the context itself. The alternative, counting the derived types of a listed
type as holding it, is a few lines in the walk and was not taken: it would refuse a function for a
value the host was never going to supply. A derived type is a declaration of its own, with its own
`$type`, which is what tells it apart from a form that wraps or narrows another type (decision
14): a value of a constrained `ChatToolContext` is a `ChatToolContext`, and a value of
`ChatToolContext.Update` is not.

One more thing the export gets right with this change. A type alias denotes its target in the
schemas and to a runtime, but the host-supplied match and `typeRef` stopped at the alias's name,
so `context:Context` with `type Context = ChatToolContext` was not host-supplied and its record
went to the model. Both now look through an alias of a named type, through any number of aliases:
the parameter is host-supplied and `typeRef` names the record the alias denotes, which is what
the package compares with the host's type. The spelling in `type` stays the author's. An alias of
an occurrence denotes no single declaration and is held, not supplied.

The .NET SDK does not expose `functionSchema`, so the change is the Rust crate and the two
JavaScript bindings. No runtime and no IR changes.

### 14. Types the language gains later

NX will gain more specialized types. The first is expected to be a constrained type, an `int`
between 1 and 10 for one, and more will follow. The package is built so that such a type needs no
change here. The rule that gets it there: **the package never interprets a type.** Everything it
needs to know about one, the compiler answers or the runtime enforces.

- **Schemas pass through.** `inputSchema` and `outputSchema` are stored and forwarded as the export
  wrote them. Whatever keywords a constrained type becomes (`minimum`, `maximum`, `minLength`,
  `pattern`), they reach the model untouched, and a test holds the package to that with keywords
  it has never heard of.
- **Validation is the runtime's.** The package has no validator (decision 7), so it has no second
  copy of the rules to bring up to date. A model that sends 11 where the parameter admits 1 to 10
  is refused where a model that sends a string is refused today: at the runtime's boundary.
- **A refusal at the boundary is `invalid-input`** whatever its code, as long as the code is of
  the boundary family (decision 7).
- **Context types are found by the compiler** (decision 13). The walk over a type lives beside the
  definition of what a type can be. It matches every form of `Shape` by name, with no catch-all
  arm, so adding a form to the language does not compile until someone has said what the walk
  does with it. A form that narrows or wraps another type, as a constraint does, is walked into.
- **Identity is a declaration's module and name.** `typeRef`, `toolContextType` and
  `config.contextParameters` compare by it. A named constrained type is a declaration like any
  other, and an inline constraint has no name to compare and needs none.

What this rests on, which the change that adds constrained types has to hold, and should be
checked against when it is written:

1. The export writes a constraint into the schemas. N2's requirement *Schemas agree with the
   runtimes' boundary* already demands it: a schema may be narrower than what a runtime accepts
   and never wider, so once a runtime refuses 11, a schema that admits it is wrong.
2. Both runtimes check a constraint on every value a host passes in, and report a violation with a
   code that begins `nx-ir-boundary-`. A violation inside a function, of a value the program
   computed, is not the model's to correct and keeps a code outside that family.
3. A parameter declared as a constrained form of a context type is still reported `hostSupplied`,
   with the context type as its `typeRef`.

What may want a second look then, and none of it is needed now:

- A provider may refuse a schema keyword, as it may refuse `$ref` today (see Risks). The adapter
  does not rewrite schemas.
- The `@nx/agent` library could state some of what this package checks as constraints of its own
  types: the pattern of a tool's `name`, a positive `maxSteps`. The compiler would then report
  them at the author's source. The package's checks stay, because a name is often derived and not
  authored, and because an `Agent` value does not have to come from a checked program. The test
  that regenerates the library's TypeScript types will show the change when it comes.
- `hostSuppliedWithin` is a list for a like reason: an answer shaped for one listed type would have
  to change shape the day a host lists two and a parameter holds both.

## Risks / Trade-offs

- [The interfaces of N1 to N4 are taken from their change text, not from landed code] → Checked
  against the landed code in section 1 of the tasks. Every identifier is as written here, and a
  program artifact satisfies `AgentSchemaSource` as it stands. One difference was found: N2 answered nothing for
  a parameter that holds a context type under an occurrence. Decision 13 adds what was missing.
- [A host context record that does not satisfy a function's declared context type fails at the
  runtime boundary and is classified `invalid-input`, as if the model had erred, and since
  `invalid-input` is the code whose message the adapter shows, the model is sent the runtime's
  sentence, which names the field of the context that is wrong] → The compile-time
  `toolContextType` check rules out a mismatched declared type. A record built from the host's
  generated type has the right fields, with one hole the package closes: TypeScript accepts an
  optional field set to `undefined` unless `exactOptionalPropertyTypes` is on, and the runtime
  would refuse that member, so `execute` leaves out a member that is `undefined`: of the record
  itself, whatever built it, and of every plain object below it. It does not look into an
  instance of a class below the top level, because the runtime's input measure counts one as a
  single value and the copy has to enter exactly what the measure enters; the runtime's check
  does enter one at a field typed as a record, so an `undefined` member there is still refused,
  and the README says to write a nested part of the context as a plain object. The cause is in
  the runtime, which has no single answer to what a host value is; the follow-on change
  `treat-host-values-as-json` gives it one and removes this package's copy. A record of
  another type, one with a field the type does not declare, or one missing a required field can
  still be passed by a host that does not type it, and is reported this way. The diagnostics name
  the context parameter, so the cause is visible in the host's trace. The proper fix is in the runtimes, which
  report which argument failed only in a message: the follow-on change
  `name-the-argument-in-boundary-diagnostics` adds that as data and has the package report
  `invalid-context`.
- [This change now touches the schema export, which it first said it would not] → The addition is
  one optional member of an answer, the export landed on this branch and is not on `main`, and no
  runtime or IR changes. The proposal and the `declaration-schema-export` delta say so.
- [A boundary code that is not about what the host passed in would be reported to the model as its
  mistake] → The family is the runtime's name for a value that does not fit at the boundary, and
  decision 14 states it as a condition on the change that adds the next code. It is not always
  the host's value, though. The runtime fills in a record's field defaults while it checks the
  argument that holds the record, and checks each default's value against the field's type; the
  compiler does not type-check a default that reads an earlier field (`specs/future.md`), so
  `type Req = { n:int label:string = { n } }` compiles and a call with an argument that fits
  fails with `nx-ir-boundary-type` for `req.label`, which is `invalid-input` here. The follow-on
  change `name-the-argument-in-boundary-diagnostics` makes a failure that is not in a value the
  host passed `evaluation-failed`.
- [`@nx/agent` has no concrete `ToolContext` subtype, so a host that declares none cannot run a
  function with a context parameter] → `normalizeAgent` rejects such a tool at compile time instead
  of letting it fail at run time. See Open Questions.
- [The function check reads `modulesByIdentity` and `declarationsByName`, which the runtime exports
  without a stability promise] → Both packages are released together at one version, and the
  package's tests fail if the fields move.
- [Bare `$type` matching can confuse a host type with a library type of the same name] → The
  package's three type names are reserved and documented; a describer for one of them is refused.
  Exact identity would need N4 or the canonical encoding to carry a module, which is out of scope.
- [No JSON Schema validation of model input in the package] → The runtime's boundary validation is
  the authority and N2 guarantees agreement; the test suite runs schema-valid and schema-invalid
  inputs through both to hold the guarantee.
- [A provider may reject a draft 2020-12 construct N2 emits, such as `$ref` into `$defs`, `anyOf`, `not` or `const`, or a
  keyword a constrained type will add] →
  Out of this package's hands; the adapter does not rewrite schemas. Recorded so the ReachMe
  changes test real schemas against the provider they use.
- [`destructiveHint: true` for every non-GET method overstates some POSTs] → The hints are
  conservative by MCP's own default, and an author cannot be trusted to declare a write harmless
  (design doc §5.2).
- [Synchronous evaluation cannot be aborted] → The budget bounds it; the default is finite.
- [The stored definition format will change while the package is unstable] → `formatVersion` makes
  a mismatch an explicit error; hosts that pin compiled bundles recompile on upgrade.
- [AI SDK major versions break often] → The adapter is one small entry point with its own tests
  against the pinned `ai` dev dependency; the rest of the package has no AI SDK types.

## Migration Plan

Additive. Nothing depends on the package until ReachMe adopts it. It ships in the first release
after N1 to N4 land; rollback is not publishing it, or deprecating the version on npm.

## Open Questions

- **Is `@nx-lang/agent` the final name?** The design doc calls it provisional. Default taken: keep
  it. A rename before the first ReachMe release costs one manifest and the docs.
- **Should a request body keep `$type` for payload union cases?** Default taken: strip every
  `$type`. If an operator needs a tagged union in a body they declare the tag as an ordinary field.
  Depends on how N4 types `HttpArguments.body`; it does not change the builder's other rules.
- **Should `put` and `delete` be `idempotentHint: true`?** Default taken: yes, following HTTP
  semantics. A host that treats the hint as permission to retry should confirm with the API's owner;
  the package never retries.
- **Should the summary paragraph or the whole doc comment be the description?** Default taken:
  N2's `description`, the whole documentation. N2 also answers `summary`; switching is one line.
- **Should `@nx/agent` or this package offer a concrete `ToolContext` for hosts with nothing to
  add?** Default taken: no, because N4 fixes the type list. A host that wants only `callId` declares
  a one-line empty subtype in its own library and names it as `toolContextType`.
- **Should a host with no context type be able to run a function whose context parameter is
  optional?** Default taken: no. Every context parameter is filled at every call, and a call with
  no record fails with `invalid-context`, which is right for a host that has a context. The case
  of a host that has none is recorded in `specs/future.md` (*Agent Host Package: What
  `add-agent-host-package` Left For Later*), with the other things this change leaves: provider
  acceptance of the schemas, and an `invalid-request` the model caused.
- **Decision 4 in the design doc's §9 (where the operator-visible tool trace is stored)** does not
  touch this package: it returns results and the host stores them.
