---
title: 'Running an agent in a host'
description: 'How a JavaScript host turns an evaluated @nx/agent Agent into tools a model can call, with the @nx-lang/agent package.'
sidebar:
  label: 'Agent hosts'
---

An NX program that imports [`@nx/agent`](/reference/libraries/agent) declares an agent and its
tools. Evaluated, that agent is a value: NX runs nothing. A host reads the value, gives a model
the tools, and runs each tool the model calls.

[`@nx-lang/agent`](https://www.npmjs.com/package/@nx-lang/agent) is the npm package that does the
part every JavaScript host would otherwise write the same way. It derives each tool's name,
description and JSON Schemas, runs a tool's function under a budget, and builds an HTTP tool's
request so that it cannot leave its connection. It does not run a model, and network policy,
credentials and storage stay the host's.

:::caution[Unstable]
`@nx-lang/agent` is **unstable**, as the library it reads is. Its API and the format of the
definition it stores may change incompatibly in any release, including a patch release. Pin an
exact version, and compile your stored configs again when you upgrade. The package first ships
in release 0.6.0.
:::

```bash
npm install @nx-lang/agent @nx-lang/ir-runtime
```

## The program

This is the program the rest of the page runs. One tool calls an NX function, one calls an HTTP
API, and one is web search run by the model's provider:

```nx
import "@nx/agent"

type Plan = { name:string seats:int monthlyPrice:int }

/// Finds the plans that fit a team.
let findPlans(
  /// Number of people who need a seat.
  teamSize: int
): Plan* = { <Plan name="Team" seats={teamSize} monthlyPrice=20 /> }

/// Looks an order up by its identifier.
let lookupOrder(orderId: string): HttpArguments = {
  <HttpArguments pathParams=<HttpParam name="orderId" value={orderId} /> />
}

let shop = <HttpConnection name="shop" baseUrl="https://api.example.com/v1" />

let root(): Agent = {
  <Agent name="support" model="balanced" tools={
    <FunctionTool function={findPlans} />
    <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />
    <WebSearchTool allowedDomains={ "docs.example.com" } />
  }>
    You are the support assistant. Answer from the documentation site.
  </Agent>
}
```

The model sees three tools: `find_plans`, `lookup_order` and `web_search`. Each name is the
function's name in snake_case, each description is the function's `///` comment, and each input
schema comes from the function's parameters. An author can set `name` and `description` on a tool
to override them.

## Where the compiler is

A host usually compiles a config in one place and runs tools in another, which has no compiler.
Where the compiler is, the host evaluates the agent and normalizes it into a *definition*: plain
JSON, with every tool in the [MCP](https://modelcontextprotocol.io) tool shape. The program
artifact supplies the [declaration schemas](/reference/concepts/declaration-schemas):

```ts
import type { Agent } from "@nx-lang/agent";
import { checkAgentTools } from "@nx-lang/agent/execute";
import { normalizeAgent } from "@nx-lang/agent/normalize";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import type { NxHost } from "@nx-lang/sdk-wasm";

export function compileAgent(host: NxHost, source: string): { definition: string; images: Map<string, Uint8Array> } {
  const artifact = host.buildWorkspaceArtifact({ modules: [{ identity: "main.nx", source }], entry: "main.nx" });
  try {
    const images = new Map(artifact.generateNxIr({ modules: [] }).map((image) => [image.identity, image.bytes]));
    const prepared = new Map([...images].map(([identity, bytes]) => [identity, prepareNxIrModule(bytes)]));
    const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });

    const result = normalizeAgent(evaluateFunction(program, "root") as unknown as Agent, { schemas: artifact });
    if (!result.ok) {
      throw new Error(result.diagnostics.map((diagnostic) => `${diagnostic.path}: ${diagnostic.message}`).join("\n"));
    }
    const problems = checkAgentTools(result.definition, program);
    if (problems.length > 0) {
      throw new Error(problems.map((problem) => problem.message).join("\n"));
    }
    // Store both: the definition as text, and one image for each module of the program.
    return { definition: JSON.stringify(result.definition), images };
  } finally {
    artifact.dispose();
  }
}
```

Normalization reports everything it finds in one pass, as diagnostics with a code and a path such
as `tools[1]`: a tool with no description, two tools with one name, a parameter whose type has no
JSON form, a base URL that is not `https`. With one error there is no definition.

## Where the tools run

Where the tools run, the host needs the stored definition, the stored images and
`@nx-lang/ir-runtime`. This half loads no compiler and no WebAssembly, so it runs in a Cloudflare
Worker:

```ts
import type { NormalizedAgent } from "@nx-lang/agent";
import { createAgentTools, type AgentHttpRequestFunction } from "@nx-lang/agent/execute";
import { linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

export async function findPlans(definition: string, images: Map<string, Uint8Array>, request: AgentHttpRequestFunction) {
  const prepared = new Map([...images].map(([identity, bytes]) => [identity, prepareNxIrModule(bytes)]));
  const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });
  const tools = createAgentTools(JSON.parse(definition) as NormalizedAgent, program, { request });

  const tool = tools.find((candidate) => candidate.name === "find_plans")!;
  // The host chooses the callId, and keeps it the same when it retries the call.
  const result = await tool.execute!({ teamSize: 5 }, { callId: "turn_7:1:0" });
  if (!result.ok) {
    // A failure is a result with a code, never a rejected promise.
    return { error: result.error.code, message: result.error.message };
  }
  // [{ $type: "Plan", name: "Team", seats: 5, monthlyPrice: 20 }], and what the call used.
  return { output: result.output, operations: result.usage?.operations };
}
```

Each kind of tool runs its own way:

| Tool | What `execute` does |
| --- | --- |
| `FunctionTool` | Calls the function through the IR runtime, which checks the model's arguments against the parameter types. |
| `HttpTool` | Calls the arguments function, builds the request, and hands it to the `request` function the host supplied. |
| `WebSearchTool` | Nothing: it has no `execute`. The model's provider runs it, and the host maps it to the provider's own tool. |
| A host's own `Tool` subtype | Calls the executor the host registered for it. |

A failed call says why with one of a few codes, so a host does not read IR diagnostics:
`invalid-input` when the model sent arguments of the wrong shape and can correct them,
`resource-limit` when a limit was reached, `evaluation-failed` when the function itself failed.

## Limits

A tool's function is code the host did not write, so every call runs under limits:

- **A budget.** A call may cost 100,000 operations unless the host sets its own `maxOperations`.
  A realistic tool costs hundreds to a few thousand. One that goes over fails with
  `resource-limit`.
- **The model's input**, held to a size of 1,000 by default. Over it, the call fails with
  `invalid-input` and the function is not called.
- **The host's context**, held to the same size separately, so a large context does not use up
  the model's allowance.

A result carries `usage`, the operations the call cost and the size of its input. Logging it is
how a host learns what budget its tools need.

## Values the host supplies

A function often needs to know who is asking. Those values come from the host, never from the
model. A host declares a subtype of the abstract `ToolContext` in a library of its own, and a
function takes it as a parameter:

```nx
import "@nx/agent"

/// What a chat host supplies to a tool's function.
export type ChatToolContext extends ToolContext = {
  conversationId: string
}

/// Says which conversation is asking.
let whoIsAsking(context: ChatToolContext): string = { context.conversationId }

let root(): Agent = {
  <Agent name="support" tools={ <FunctionTool function={whoIsAsking} /> }>Be brief.</Agent>
}
```

The parameter is left out of the tool's input schema, so the model is never asked for it. The host
names its type as `toolContextType` when it normalizes, and passes the record as `context` at each
call. The package sets the record's `callId` to the call's.

The context has to be a parameter of its own. A function that takes a list of contexts, or a
record with a context among its fields, is refused when the agent is normalized, because the
model would be asked for the part the host cannot fill in.

## HTTP tools

An `HttpTool` fixes the method and the path, and its arguments function supplies only what varies.
The package builds the request so that nothing a model sends can change where it goes: values are
percent-encoded, a path segment of `.` or `..` is refused, and the finished URL is refused unless a
URL parser reads it exactly as it was built, under the connection's base URL.

The package never sends the request. It hands the request, the tool's name and the `callId` to the
host's `request` function, and everything about the network is that function's to enforce: which
addresses may be reached, redirects, timeouts, response sizes, credentials, idempotency headers
and retries.

## With the AI SDK

`@nx-lang/agent/ai-sdk` maps the tools to a [Vercel AI SDK](https://ai-sdk.dev) tool set. `ai` is
an optional peer dependency, installed only by a host that uses this entry point.

A failed call reaches the model as a tool error. The model is told why for `invalid-input`, which
it can correct, and for a failure the host's own function gave a code of its own, whose message
the host wrote. For the package's other codes it is told what kind of failure it was, and the
host reads the reason in its `onResult` function. That keeps the text of a network error and a
connection's address from the model, as long as the host does not put them in a message of its
own.

## More

The package's [README](https://github.com/nx-lang/nx/tree/main/packages/agent) has the whole API:
the definition format, every diagnostic and result code, the HTTP rules, a request function over
`fetch`, describers and executors for a host's own tool types, and the AI SDK adapter.
