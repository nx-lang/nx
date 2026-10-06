# @nx-lang/agent

> **Unstable.** The API and the stored definition format may change incompatibly in any release,
> a patch release included. The package name is provisional. Pin an exact version.

Turns an evaluated NX `Agent` into tools a host can give to a model. An NX program that imports
the [`@nx/agent`](https://nxlang.org/reference/libraries/agent) library declares an agent and its
tools; evaluated, that agent is only a value. This package does the part every host would
otherwise write the same way:

- it derives each tool's model-facing name, description and JSON Schemas, and reports what is
  wrong with an agent before anything runs;
- it runs a `FunctionTool`'s function through [`@nx-lang/ir-runtime`](../../runtime/typescript)
  under a budget, and tells the host why a call failed without making it read IR diagnostics;
- it turns an `HttpTool`'s arguments into a request that cannot leave its connection.

It does not run a model, and it owns no network policy, credentials or storage. Those are the
host's, and the package hands over what they need.

## Two halves

A host usually compiles a config in one place and runs tools in another, which has no compiler.
The package is split the same way, and a JSON definition is the only thing that crosses.

**Where the compiler is**, `normalizeAgent` turns the evaluated `Agent` and the program artifact
into a definition, or into diagnostics:

```ts
import type { Agent, NormalizedAgent } from "@nx-lang/agent";
import { checkAgentTools } from "@nx-lang/agent/execute";
import { normalizeAgent } from "@nx-lang/agent/normalize";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import type { NxHost } from "@nx-lang/sdk-wasm";

export function compileAgent(host: NxHost, source: string): { definition: string; images: Map<string, Uint8Array> } {
  const artifact = host.buildWorkspaceArtifact({ modules: [{ identity: "main.nx", source }], entry: "main.nx" });
  try {
    // One image for each module of the program, the agent library's among them.
    const images = new Map(artifact.generateNxIr({ modules: [] }).map((image) => [image.identity, image.bytes]));
    const prepared = new Map([...images].map(([identity, bytes]) => [identity, prepareNxIrModule(bytes)]));
    const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });

    // The artifact is the schema source as it stands.
    const result = normalizeAgent(evaluateFunction(program, "root") as unknown as Agent, { schemas: artifact });
    if (!result.ok) {
      throw new Error(result.diagnostics.map((diagnostic) => `${diagnostic.path}: ${diagnostic.message}`).join("\n"));
    }
    const definition: NormalizedAgent = result.definition;
    // Every function the definition names is in the program it will run with.
    const problems = checkAgentTools(definition, program);
    if (problems.length > 0) {
      throw new Error(problems.map((problem) => problem.message).join("\n"));
    }
    return { definition: JSON.stringify(definition), images };
  } finally {
    artifact.dispose();
  }
}
```

**Where the tools run**, `createAgentTools` turns the stored definition and the linked program
into tools with `execute`:

```ts
import type { NormalizedAgent } from "@nx-lang/agent";
import { createAgentTools, type AgentHttpRequestFunction, type AgentTool } from "@nx-lang/agent/execute";
import { linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

export function toolsOf(definition: string, images: Map<string, Uint8Array>, request: AgentHttpRequestFunction): AgentTool[] {
  const prepared = new Map([...images].map(([identity, bytes]) => [identity, prepareNxIrModule(bytes)]));
  const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });
  // Throws an NxAgentError, before any tool runs, when the definition cannot run with this program.
  return createAgentTools(JSON.parse(definition) as NormalizedAgent, program, { request });
}

export async function callTool(tools: AgentTool[], name: string, input: unknown, callId: string): Promise<unknown> {
  const tool = tools.find((candidate) => candidate.name === name);
  if (tool?.execute === undefined) {
    throw new Error(`No tool named '${name}' can be executed.`);
  }
  // A failure of the tool is a result, never a rejected promise.
  const result = await tool.execute(input, { callId });
  return result.ok ? result.output : { error: result.error.code, message: result.error.message };
}
```

## Entry points

| Entry point | Holds | Loads at run time | In a Worker |
| --- | --- | --- | --- |
| `@nx-lang/agent` | The `@nx/agent` types, the definition types, `toSnakeCase`, `buildHttpRequest`, `NxAgentError`, `NxAgentToolError` | Nothing | Yes |
| `@nx-lang/agent/normalize` | `normalizeAgent`, the describer types, `AgentSchemaSource` | Nothing | Yes, though it is only useful where a schema source is, which is where the compiler runs |
| `@nx-lang/agent/execute` | `createAgentTools`, `checkAgentTools`, `evaluateHttpArguments` | `@nx-lang/ir-runtime` | Yes |
| `@nx-lang/agent/ai-sdk` | `toAiSdkTools` | `ai` | Yes |

No entry point imports a Node built-in, a WebAssembly module or `@nx-lang/sdk-wasm`, and a test
holds each to the column above. `ai` is an optional peer dependency that only
`@nx-lang/agent/ai-sdk` imports, so a host that does not use the AI SDK installs nothing for it.

## The library's types

The root entry exports the TypeScript types `nxlang typegen @nx/agent` generates, under their NX
names: `Agent`, `Document`, `AgentLimits`, `Tool`, `FunctionTool`, `WebSearchTool`, `ToolContext`,
`Connection`, `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments` and `HttpTool`.
Generated code for a host library that references the agent library imports them from here:

```ts
import type { Tool, ToolContext } from "@nx-lang/agent";

export interface RecordSearchTool extends Tool {
  $type: "RecordSearchTool";
  recordKind: string;
}

export interface ChatToolContext extends ToolContext {
  $type: "ChatToolContext";
  conversationId: string;
}
```

The files are checked in under `src/generated` and a test regenerates them and fails on any
difference. One thing is changed in the generator's output: each relative import gains its `.js`
extension, without which a consumer compiled with `moduleResolution: nodenext` cannot import the
types.

## The definition

A `NormalizedAgent` is plain JSON. `JSON.stringify` it, store it, parse it back: the result equals
what was stored, and the same value and schemas always give the same text.

| Field | |
| --- | --- |
| `formatVersion` | `1`. `createAgentTools` refuses a version it does not know, naming both. |
| `name`, `description?`, `model?`, `instructions` | As authored. `model` is an opaque string the host resolves. |
| `documents` | `{ title, text }` in authored order. Titles are unique. |
| `limits` | `{ maxSteps?, maxToolCalls? }` as authored. Capping them is the host's. |
| `tools` | One definition for each authored tool, or several when a describer returns several. |

Each tool is in the [MCP](https://modelcontextprotocol.io) tool shape, with a `kind` and a
`config` that holds what its executor needs:

| Field | |
| --- | --- |
| `name` | Matches `^[a-z][a-z0-9_]{0,63}$`. The authored `name`, or the function's name in snake_case (`findPlans` is `find_plans`, `HTTPStatusFor` is `http_status_for`); `web_search` for a `WebSearchTool`. An authored name that does not match is refused, not converted. |
| `title?` | Only a host describer sets one. |
| `description` | The authored `description`, or the function's `///` documentation. Required. |
| `inputSchema` | JSON Schema draft 2020-12 of an object: the function's parameters, without the ones the host fills in. |
| `outputSchema?` | The schema of the output as it is. An array result is an array, not an object that holds one. |
| `annotations` | The four MCP hints, fixed by the tool's type and never authored. |
| `kind`, `config` | See below. |

| `kind` | From | `config` | Annotations |
| --- | --- | --- | --- |
| `function` | `FunctionTool` | `{ function, contextParameters }` | Read-only, idempotent, closed-world |
| `http` | `HttpTool` | `{ arguments, contextParameters, connection: { name, baseUrl }, method, path, pathPlaceholders, maxCallsPerConversation? }` | Open-world. `get` is read-only and idempotent; every other method is a destructive write, and `put` and `delete` are idempotent |
| `provider` | `WebSearchTool` | `{ provider: "web_search", allowedDomains? }` | Read-only, idempotent, open-world |
| anything else | A host describer | Whatever the describer stored | The describer's |

`function` and `arguments` are `{ module, name }`, the pair a canonical `Function` record carries.
An optional field with no value is absent, never `null`.

## Diagnostics

`normalizeAgent` answers `{ ok: true, definition, diagnostics }` or `{ ok: false, diagnostics }`.
It reports everything it can find in one pass, and with one error there is no definition. Each
diagnostic has a `severity`, a `code`, a `message`, a `path` into the agent such as `tools[1]`,
the tool's name when one was resolved, and, when it is about a tool's function, `declaration`: the
function's module and the span of its declaration.

| Code | |
| --- | --- |
| `nx-agent-not-an-agent` | The value is not an `Agent`, or a field of it is not what an `Agent` holds. |
| `nx-agent-duplicate-document-title` | Two documents share a title. |
| `nx-agent-duplicate-tool-name` | Two tools share a name, however each came by it. |
| `nx-agent-invalid-tool-name` | A name, authored or derived, does not match the pattern. |
| `nx-agent-missing-description` | A tool has no description, or an empty one. |
| `nx-agent-inexpressible-parameter` | A parameter's type has no JSON form, such as a function type. |
| `nx-agent-inexpressible-result` | A warning, the only one: the result type has no JSON form, so the tool has no `outputSchema`. |
| `nx-agent-unknown-function` | A tool names a function the program does not declare. Also thrown by `createAgentTools`. |
| `nx-agent-context-parameter` | A context parameter the host cannot fill, or a parameter that holds a context without being one. See below. |
| `nx-agent-unknown-tool-type` | A tool value of a type with no describer, or one that is not well formed. |
| `nx-agent-describer` | A host describer refused a tool, or returned something that cannot be stored. |
| `nx-agent-http-base-url`, `nx-agent-http-path` | An `HttpTool`'s connection or path is malformed. |
| `nx-agent-http-arguments-type` | An `HttpTool`'s `arguments` is not a function. |

`createAgentTools` throws an `NxAgentError`, whose `diagnostics` hold every problem it found, with
`nx-agent-unsupported-format`, `nx-agent-invalid-definition`, `nx-agent-unknown-function`,
`nx-agent-missing-executor` or `nx-agent-missing-request-function`. `checkAgentTools` answers the
first three as diagnostics and needs neither executors nor a request function.

A stored definition is JSON the host kept, so the package checks what it reads of one before any
tool runs: `nx-agent-invalid-definition` names the place, such as `tools[2]`, of a definition, a
tool list or a tool that does not have the shape `normalizeAgent` wrote. A damaged tool is
reported beside whatever is wrong with the others. An unsupported format version is reported
alone, since nothing else in such a definition can be read with any confidence.

## Results

`execute(input, { callId, context?, signal? })` resolves to `{ ok: true, output, usage? }` or
`{ ok: false, error: { code, message, diagnostics?, limit? }, usage? }`. The `callId` is the
host's: it identifies the call, stays the same when the call is retried, and is never made up,
changed or remembered by the package.

| Code | |
| --- | --- |
| `invalid-input` | The model sent arguments the function's parameters do not admit, or too many. It can correct them. |
| `invalid-context` | The call has no `callId`, or a tool that needs the host's context record was given none, or one that does not fit the function's context type. The diagnostics say which. |
| `evaluation-failed` | The function failed for a reason other than its arguments or a limit: in its body, in a default it fills in, or in its result. |
| `resource-limit` | A limit was reached. `limit.name` says which: `maxOperations`, `maxContextSize`, or another of the runtime's. |
| `invalid-request` | An `http` tool's request could not be built, or its function did not return `HttpArguments`. |
| `request-failed` | The host's request function threw. |
| `aborted` | The call's signal was already aborted. Nothing was evaluated. |
| any other | The code of an `NxAgentToolError` a host's request function or executor threw. |

A call's arguments are checked by the IR runtime against the function's parameter types, and
the package has no validator of its own. A refusal there is a diagnostic whose code is
`nx-ir-arguments` or begins `nx-ir-boundary-`, and whose `argument` names the parameter the
refused value was passed for. The package reads that name and never the message:

- an argument the model sent is `invalid-input`;
- a context parameter, which the host filled in, is `invalid-context`, and so is a call that
  fails in both, since the model cannot make it succeed while the host's record is wrong;
- a refusal that names no argument is `evaluation-failed`. It was not found in what the call was
  given: a record's field default whose value does not fit its field is one.

The family of codes is matched and not a list of them, so a check the runtime gains later is
reported the same way. The schemas are stored and passed on exactly as the compiler wrote them,
keywords the package has never seen included.

## Budget

Three layers, which do not conflict:

1. **The IR runtime has no default.** `maxOperations` is unlimited unless it is set.
2. **This package sets 100,000**, `NX_AGENT_DEFAULT_MAX_OPERATIONS`, when the host's `runtime`
   options do not, because it exists to run code its host did not write. The number is low on
   purpose. A realistic tool costs hundreds to a few thousand operations, so there is ample room,
   and 100,000 operations is a few milliseconds during which nothing else is served. A tool that
   outgrows it fails with `resource-limit` and a `limit` that says so, which is easy to act on;
   lowering a default that tools have come to rely on breaks them.
3. **A host passes its own** as `runtime: { maxOperations }`, and it is used as given.

Every call gets the whole budget, and a host cannot turn the budget off through this package.
Evaluation is synchronous, so a running call is bounded by its budget and not by its abort signal.
Keep a limit of your own on time as well: in a Worker, the CPU limit.

## The two input limits

What goes into a call comes from two parties, and each has its own limit, measured as the
runtime's `measureInputSize` measures a value:

| Option | Default | Over it | The party at fault |
| --- | --- | --- | --- |
| `maxArgumentsSize` | 1,000 | `invalid-input`, `limit.name` `maxArgumentsSize` | The model, which can send less |
| `maxContextSize` | 1,000 | `resource-limit`, `limit.name` `maxContextSize` | The host, whose context record it is |

Neither failure calls the function. The limits are separate so that a large context cannot use up
the model's allowance and have the model told that its input was too big. A `maxInputSize` in
the host's `runtime` options is not used: the package sets the runtime's limit to what a call
within both limits can reach, as a backstop. Both limits can be raised or lowered, and neither
can be turned off.

## What a call used

A result carries `usage`, `{ operations, inputSize }`, whenever the call reached the runtime: on a
success, and on a failure of evaluation or of a limit, where it says how far the call got. A
failure the package finds before that, such as input that is too large, has none. For an `http`
tool the numbers are those of its arguments function. Each call reports its own numbers, so calls
a model makes together do not share them, and a `usage` object in the host's `runtime` options
is left alone.

Log `usage.operations` with each result. What a host's tools cost in practice is what to choose a
budget from.

## Tool context

A function can take values from the host that the model never supplies: who is asking, in which
conversation. `ToolContext` is abstract, so a host declares a concrete subtype in a library of its
own:

```nx
import "@nx/agent"

/// What a chat host supplies to a tool's function.
export type ChatToolContext extends ToolContext = {
  conversationId: string
  contactEmail?: string
}

/// Looks an order up for the conversation that asks.
let lookupOrder(orderId: string, context: ChatToolContext): string = {
  orderId + " in " + context.conversationId
}
```

A parameter declared `ChatToolContext`, or `ToolContext`, or a type alias of either, is left out of
the tool's input schema and recorded in its `config`. The host names its type when it normalizes and supplies the record,
with its `$type`, at each call:

```ts
import type { Agent } from "@nx-lang/agent";
import type { AgentTool } from "@nx-lang/agent/execute";
import { normalizeAgent, type AgentSchemaSource } from "@nx-lang/agent/normalize";

export function normalizeForChat(agent: Agent, schemas: AgentSchemaSource) {
  // The identity of the module that declares the type, and its name.
  return normalizeAgent(agent, { schemas, toolContextType: { module: "libraries/chat/Chat.nx", name: "ChatToolContext" } });
}

export function lookUp(tool: AgentTool, conversationId: string) {
  // The record is passed to every context parameter, with `callId` set to this call's.
  return tool.execute!({ orderId: "A1" }, { callId: "turn_7:1:0", context: { $type: "ChatToolContext", conversationId } });
}
```

The record can be a value of the type `nxlang typegen` generates for the host's context type, with
its `callId` or without it (`Omit<ChatToolContext, "callId">`), since the package sets `callId` at
every call. A member that is `undefined` is left out, as an optional field with no value is, so
`{ ...context, contactEmail: maybeEmail }` is safe to pass. That holds for the record itself,
whatever built it, and for every plain object inside it. Write a nested part of the context as a
plain object: the record is read as the IR runtime reads any host value, so an instance of a class
below the top level, a `Date` or anything else that is not plain data is refused, with code
`invalid-context` and a diagnostic that names where it is.

A key of the model's input that names a context parameter is dropped. With no `toolContextType`,
a function that declares a context parameter is an error, as is one declared with another subtype
than the host's.

A record that does not fit the function's context type (a required field missing, a field of the
wrong type, a `$type` that names another type) fails the call with `invalid-context`, like a
record that was not passed at all. The mistake is the host's, so the model is not asked to correct
it; the failure's `diagnostics` name the field, and their `argument` is the context parameter. A
context built from your generated type fits.

Only a parameter declared with the context type itself is a context parameter. One declared as a
list of it (`contexts: ChatToolContext+`), or with a record that holds one as a field at any
depth, is an error: the host cannot fill in part of a value, so the model would be asked for the
context. Declare the context as a parameter of its own. A type derived from the context type,
`ChatToolContext.Update` or `ChatToolContext.Property`, is another type: a parameter declared with
one is an argument the model supplies, like any other.

The package learns all of this from the schema source and never reads a type's spelling. A source
that is not a program artifact answers, for each parameter, `hostSupplied` when the parameter is a
context, `hostSuppliedWithin` when its type holds one, and `typeRef`.

## HTTP tools

An `http` tool evaluates its arguments function, builds a request and hands it to the function
the host gave as `request`. `buildHttpRequest(tool, httpArguments)`, in the root entry, is the
builder on its own: pure, with no I/O. Its rules:

- The method, scheme, host, port and the fixed part of the path come from the tool. Nothing in the
  arguments can change them.
- A value fills a `{name}` placeholder or a query parameter with every byte outside
  `A-Z a-z 0-9 - . _ ~` percent-encoded, so it cannot hold a `/`, `?`, `#`, `&` or `=`.
- A placeholder with no path parameter, a path parameter with no placeholder, one given twice and
  an empty value are refused. So is a path segment that comes out as `.` or `..`.
- The finished URL is parsed, and refused unless the parser reads it exactly as it was built: the
  base URL's origin, no user information, the base path followed by the path, the query, and no
  fragment.
- A body is sent as JSON with every `$type` key removed and `content-type: application/json`, the
  only header the package sets. A body on `get` or `delete` is refused.

A connection's `baseUrl` must be `https`, with no query, fragment or user information, written the
way a URL parser writes it (`https://api.example.com/v1`, not `https://API.example.com:443/v1`).
`allowInsecureBaseUrl` accepts `http`, for a test against a local server.

**The host still enforces everything about the network.** The package does not resolve or filter
addresses, follow or refuse redirects, set timeouts, cap sizes, add credentials or idempotency
headers, retry, or count calls against `maxCallsPerConversation`. A request function over `fetch`:

```ts
import { NxAgentToolError, type JsonValue } from "@nx-lang/agent";
import type { AgentHttpRequestFunction } from "@nx-lang/agent/execute";

export function requestWith(tokenFor: (connection: string) => string): AgentHttpRequestFunction {
  return async ({ request, connection, callId, annotations, signal }) => {
    const headers: Record<string, string> = { ...request.headers, authorization: `Bearer ${tokenFor(connection)}` };
    if (!annotations.readOnlyHint) {
      // The callId stays the same when the call is retried.
      headers["idempotency-key"] = callId;
    }
    let response: Response;
    try {
      response = await fetch(request.url, {
        method: request.method,
        headers,
        redirect: "error",
        signal: signal === undefined ? AbortSignal.timeout(10_000) : AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
        ...(request.body === undefined ? {} : { body: request.body }),
      });
    } catch (error) {
      // What `fetch` threw is for the host's log. It can hold an address, so it is not put in
      // the message of an error with a code of the host's own, which the AI SDK adapter gives
      // the model as written.
      console.error(connection, callId, error);
      if (annotations.readOnlyHint) {
        // Thrown on as it is, this fails the call as `request-failed`: `onResult` reads the
        // reason, and the model is told only that the tool got no answer.
        throw error;
      }
      // A write that may or may not have happened is the host's to name.
      throw new NxAgentToolError("unknown-outcome", "The request was sent, and whether it was carried out is not known.");
    }
    const text = (await response.text()).slice(0, 64_000);
    let body: JsonValue = text;
    try {
      body = JSON.parse(text) as JsonValue;
    } catch {
      // Not JSON: the text is the body.
    }
    // A status that is not 2xx is an output the model reads, not a failure.
    return { status: response.status, body };
  };
}
```

The function is also handed the arguments in structured form, `{ pathParams, query, body? }`, and
the tool's name. A host that makes its requests in another process sends those and the `callId`,
and that process calls `buildHttpRequest` with its own copy of the definition and gets the same
request. `evaluateHttpArguments` runs the first two steps alone and sends nothing, for a host
that wants to try an arguments function when it compiles.

## Tool types of the host's own

A host library extends `Tool`, and the host registers a describer for the type where it
normalizes and an executor for the kind where it runs:

```ts
import type { Agent, NormalizedAgent } from "@nx-lang/agent";
import { createAgentTools } from "@nx-lang/agent/execute";
import { normalizeAgent, type AgentSchemaSource, type ToolDescriber } from "@nx-lang/agent/normalize";
import type { NxPreparedProgram } from "@nx-lang/ir-runtime";

export const toolTypes: Record<string, ToolDescriber> = {
  // Keyed by the `$type` of the tool value. Answers one tool, several, or `{ diagnostics }`.
  RecordSearchTool: (value, { toSnakeCase }) => {
    const { recordKind } = value as unknown as { recordKind: string };
    return {
      name: toSnakeCase(`search ${recordKind} records`),
      description: `Searches ${recordKind} records.`,
      inputSchema: { type: "object", properties: { query: { type: "string" } }, required: ["query"], additionalProperties: false },
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
      kind: "builtin",
      config: { recordKind },
    };
  },
};

export function describe(agent: Agent, schemas: AgentSchemaSource) {
  return normalizeAgent(agent, { schemas, toolTypes });
}

export function toolsWithBuiltins(definition: NormalizedAgent, program: NxPreparedProgram, search: (kind: string, query: string) => Promise<string[]>) {
  return createAgentTools(definition, program, {
    executors: {
      // Keyed by `kind`. What it returns is the output; what it throws fails the call.
      builtin: async (tool, input) => search((tool.config as { recordKind: string }).recordKind, (input as { query: string }).query),
    },
  });
}
```

A described tool is held to the same rules as the package's own: the name pattern, a description,
unique names and plain JSON. The authored `name` and `description` override those of a tool value
that is one tool. The package's own types (`FunctionTool`, `HttpTool`, `WebSearchTool`) and kinds
(`function`, `http`, `provider`) cannot be registered. To see every call, for a journal or a
trace, wrap `execute` on the tools `createAgentTools` returns: they are plain objects.

## The AI SDK

`toAiSdkTools` maps the tools to a [Vercel AI SDK](https://ai-sdk.dev) 7 tool set:

```ts
import type { ToolSet } from "ai";
import { toAiSdkTools } from "@nx-lang/agent/ai-sdk";
import type { AgentTool } from "@nx-lang/agent/execute";

export function toolSetFor(tools: AgentTool[], turnId: string, conversationId: string, webSearch: ToolSet[string] | null): ToolSet {
  let calls = 0;
  return toAiSdkTools(tools, {
    // A provider's tool call id changes when a step is run again. A host that retries derives its own.
    callId: () => `${turnId}:0:${calls++}`,
    context: () => ({ $type: "ChatToolContext", conversationId }),
    // The provider's own tool, such as `openai.tools.webSearch(...)`, or null to leave it out.
    providerTool: () => webSearch,
    // The whole result, `usage` included. Awaited before the output goes to the model.
    onResult: async ({ name, callId, result }) => {
      console.log(name, callId, result.ok ? "ok" : result.error.code, result.usage?.operations);
    },
  });
}
```

A tool with `execute` becomes a tool defined at run time, with the definition's description and
input schema and nothing else set. A success gives the SDK the output alone; a failure is thrown
as an `NxAgentToolError`, which the SDK reports to the model as a tool error. The model is told
why only when it can correct the call. The SDK sends the model the thrown error's message. For
an `invalid-input` failure that is the failure's own message. For the package's other codes it
is a fixed sentence, such as `Tool 'lookup_order' did not get an answer to its request.`,
because the reason can hold what a model should not see: the text your request function threw,
or a connection's address. `onResult` is given every failure whole, and what it throws is what
the model is told instead.

One thing still reaches the model as written: the message of an `NxAgentToolError` your own
request function or executor throws with a code of its own, such as `unknown-outcome` above. You
are writing to the model there, so do not build that message from an error you caught. A
`provider` tool is mapped by `providerTool`, and one with no mapping is an error, not an omission. The schemas
are draft 2020-12 and are passed to the provider as they are; a provider that refuses a construct
such as `$ref` will say so.
