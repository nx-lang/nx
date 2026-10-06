## Why

The `@nx/agent` standard library (`add-agent-library`) lets an NX program declare an `Agent` with tools, but an
evaluated `Agent` is only a value: a host still has to derive each tool's model-facing name,
description and JSON Schemas, validate the set, run a `FunctionTool`'s function safely, and turn an
`HttpTool` into a request that cannot leave its connection. Every host would write that the same way,
and it is derivable from the language, so it belongs in NX. It does not belong inside
`@nx-lang/ir-runtime`, whose defining property is pure evaluation and which has a Rust twin that
would have to grow the same code.

The first consumer is ReachMe, which compiles a tenant's config in Node (where `@nx-lang/sdk-wasm`
is available), stores the result, and later executes tools in a Cloudflare Durable Object that has
only `@nx-lang/ir-runtime` and the linked IR program. The package is shaped around that split.

## What Changes

- Add a TypeScript package, `@nx-lang/agent` (name provisional, marked unstable), at
  `packages/agent`, built on `@nx-lang/ir-runtime` and published on the package release track.
- **Compile-time half** (`@nx-lang/agent/normalize`): turn an evaluated `Agent` value plus the
  `functionSchema` export of `add-declaration-schema-export` (the host passes its program artifact)
  into a normalized, JSON-serializable agent definition. Each tool becomes an MCP-shaped definition
  (`name`, `title?`, `description`, `inputSchema`, `outputSchema?`, `annotations`) with a `kind`
  (`function`, `http`, `provider`, or a host-registered kind) and a `config` holding the data its
  executor needs. Names default to snake_case
  (`findPlans` becomes `find_plans`), descriptions to the function's doc comment, annotations are
  fixed per tool and never authored, and parameters typed `ToolContext` (or a subtype) are left out
  of the input schema.
- Normalization reports diagnostics instead of producing a partial definition: duplicate tool names,
  a tool with no description, an invalid name, a parameter type JSON Schema cannot express, a
  context parameter the host cannot fill, an `HttpTool` whose connection or path is malformed, a `Tool` subtype no describer is registered for, and duplicate document titles.
- **Run-time half** (`@nx-lang/agent/execute`): turn a stored normalized definition plus a linked
  `NxPreparedProgram` into executable tools, each with `execute(input, context)`. Creation
  fails when a tool names a function the program does not hold. A function tool calls its function
  through `callFunction` with `maxOperations` set: the host's value, or 100,000 when the host
  gives none (the runtime itself has no default). The input of a call is bounded in two parts: the
  model's arguments by `maxArgumentsSize`, 1,000 by default, and the host's context by
  `maxContextSize`, also 1,000 by default. Failures are classified for the host as `invalid-input` (arguments of the
  wrong shape, or too large), `resource-limit` or `evaluation-failed`. A result carries what the
  call used, the operations and the input size the runtime reports. An HTTP tool evaluates its arguments function, builds a
  request description with structural validation (placeholder substitution, percent-encoding, final
  URL under `baseUrl`, method from the tool) and hands it, with the structured evaluated arguments
  and the `callId`, to a function the host injects. The evaluate-and-build step is also exported on
  its own, so a host can evaluate an arguments function without sending anything. A provider
  tool (`WebSearchTool`, named `web_search`) is described only; it has no `execute`.
- Host-supplied context: `execute` receives a `callId` the host chooses and the host's record of
  its concrete `ToolContext` subtype (`ToolContext` is abstract), and passes it to every context
  parameter. The model never supplies those parameters.
- The package's root entry exports the TypeScript types that `nxlang typegen @nx/agent` generates,
  because generated host code imports the library's types from `@nx-lang/agent`.
- An extension point in both halves, so a host registers a describer and an executor for its own
  `Tool` subtypes (ReachMe's later `RecordSearchTool`, for example).
- **AI SDK adapter** (`@nx-lang/agent/ai-sdk`): maps executable tools to Vercel AI SDK 7 tools. `ai`
  is an optional peer dependency that only this entry point imports.
- **One addition to the declaration schema export**, which the package needs to keep a host's
  context away from the model. A function's parameter entry gains `hostSuppliedWithin`: the listed
  host-supplied types that the parameter's type holds without being one, as under `+` or as a
  field of a record. The package rejects such a parameter. Without it nothing in the export's
  answer says so, and the context record's schema would be put to the model.
- The package never interprets a type: it stores the export's schemas as written, leaves validation
  to the runtime's boundary, and takes what it needs to know about a parameter's type from the
  export. NX is to gain more specialized types, constrained types first, and this is what lets
  them arrive with no change here.
- The package does NOT own the model loop, network policy (DNS and address blocking, redirects,
  timeouts, size caps, HTTPS-only egress), credentials, idempotency headers, rate limits, journaling
  or persistence. It gives the host what those need (the `callId`, the annotations, the request
  description) and stops there.

Depends on five other changes in this set: `add-function-reference-type` (the `Function` record a
`FunctionTool.function` or `HttpTool.arguments` field renders to), `add-declaration-schema-export`
(JSON Schemas and doc-comment descriptions), `add-ir-runtime-evaluation-budget` (`maxOperations`
on `callFunction`), stage 1 of `add-ir-runtime-input-limit-and-cost-tests` (`maxInputSize` and
`measureInputSize`) and `add-agent-library` (the `@nx/agent` types whose values the package reads and
whose generated TypeScript types it publishes). The
ReachMe changes `add-agent-function-tools` and `add-agent-http-tools` consume this package.

## Capabilities

### New Capabilities

- `agent-host-package`: The `@nx-lang/agent` TypeScript package: normalizing an evaluated `Agent`
  into stored tool definitions, executing those definitions against a linked IR program, building
  validated HTTP requests, host extension points, diagnostics, and the AI SDK adapter.

### Modified Capabilities

- `package-release-automation`: the list of workspace npm packages on the release track gains
  `@nx-lang/agent`.
- `declaration-schema-export`: a parameter's entry names the host-supplied types its type holds,
  and a parameter declared through a type alias is host-supplied, and named, as the type the alias
  denotes.

## Impact

- New workspace package `packages/agent` (`src/`, `test/`, `README.md`), picked up by the existing
  `packages/*` workspace glob and therefore by `scripts/workspace-packages.mjs`, `pnpm -r build`,
  `pnpm -r test` and `pnpm run verify:packages` with no script change.
- Dependencies: `@nx-lang/ir-runtime` (workspace dependency); `ai` `^7.0.0` as an optional peer
  dependency; `@nx-lang/sdk-wasm` and `ai` as dev dependencies for tests only. No JSON Schema
  validator and no HTTP client are added.
- The schema export gains one optional member of its answer: `crates/nx-api` (the walk and its
  tests) and the parameter entry of `@nx-lang/sdk-wasm` and `@nx-lang/sdk-node`. The export landed
  on this branch and is not on `main`. `@nx-lang/ir-runtime`, the Rust runtime, NX IR and the .NET
  SDK are not changed.
- Docs: `packages/agent/README.md`, the package list in `docs/deployment.md`, a host-integration
  page on the website, and the new member in the declaration schemas page and the two SDK READMEs.
- No existing API changes and nothing is removed. The package is unstable until the first ReachMe
  scenarios work, and makes no compatibility promise for its API or its stored definition format
  beyond the format version check.
