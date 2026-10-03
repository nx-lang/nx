---
title: 'The agent library'
description: 'Reference for @nx/agent, the standard library of types for declaring an AI agent, its documents and its tools.'
sidebar:
  label: '@nx/agent'
---

`@nx/agent` is a [standard library](/reference/syntax/modules#standard-libraries): NX source the
compiler carries. It declares the types for an AI agent, the reference documents it reads and the
tools it can call. Import it by name:

```nx
import "@nx/agent"

let assistant = <Agent name="support">Be brief.</Agent>
```

:::caution[Unstable]
`@nx/agent` is **unstable**. Its declarations may change incompatibly in any NX release, including
a patch release, with no deprecation period. A change reaches you only when you move to a new
release of the NX packages, and every release that changes the library lists the change under
`@nx/agent` in its release notes. NX IR you have already compiled keeps working, because it links
against the library image stored with it. Source you have stored may need updating when a type
changes.
:::

The library holds types only. It has no functions and runs nothing: what a tool does, how its
schemas are derived and how a run proceeds are up to the host that reads these values.

## An agent

An `Agent` is a record written as an element. Its body is its instructions, the system prompt:

```nx
import "@nx/agent"

let root() = {
  <Agent:markdown name="support" model="balanced" limits={ <AgentLimits maxSteps=6 /> }>
    You are the support assistant. Answer briefly.
  </Agent>
}
```

```nx output
<Agent
  instructions="You are the support assistant. Answer briefly."
  limits=<AgentLimits maxSteps=6 />
  model="balanced"
  name="support"
/>
```

| Type | Fields | Notes |
| --- | --- | --- |
| `Agent` | `name`, `description?`, `model?`, `documents?: Document+`, `tools?: Tool+`, `limits?: AgentLimits`, content `instructions` | `instructions` is required. NX does not interpret `model`; the host resolves it. |
| `Document` | `title`, content `text` | Reference text sent to the model on every run, after the instructions. |
| `AgentLimits` | `maxSteps?`, `maxToolCalls?` | A host applies its own caps too, and the lower value wins. |

## Tools

`Tool` is abstract. Each concrete tool type says what a call does, and every tool takes an optional
model-facing `name` and `description`.

| Type | Fields | Notes |
| --- | --- | --- |
| `Tool` (abstract) | `name?`, `description?` | A host library extends it to add tool types of its own. |
| `FunctionTool` | `function` | Exposes an NX function. Its parameters are the tool's input and its result the output. |
| `WebSearchTool` | `allowedDomains?: string+` | Web search run by the model provider. |
| `HttpTool` | `connection`, `method`, `path`, `arguments`, `maxCallsPerConversation?` | One operation on a connection. |
| `ToolContext` (abstract) | `callId` | Values the host supplies to a tool's function. |

`FunctionTool.function` is typed `<function ... />: object*`, so it takes any function and nothing
else. A string naming a function is rejected, and so is a record that only looks like a function
value:

```nx invalid
import "@nx/agent"

let findPlans(teamSize:int): string* = { "Team" }
let tool = <FunctionTool function="findPlans" />
```

A function parameter typed `ToolContext`, or a type that extends it, is filled in by the host. It is
left out of the tool's input, so the model never supplies it.

## HTTP tools

An `HttpTool` is a fixed method and path on a connection. Only the arguments vary per call, and an
NX function computes them:

| Type | Fields | Notes |
| --- | --- | --- |
| `Connection` (abstract) | `name`, `description?` | An external service a tool can reach. |
| `HttpConnection` | `baseUrl` | A service reached over HTTPS. The model never sees the address. |
| `HttpMethod` | `get`, `post`, `put`, `patch`, `delete` | `get` makes the tool read-only. |
| `HttpParam` | `name`, `value` | One named value. The host encodes it. |
| `HttpArguments` | `pathParams?: HttpParam+`, `query?: HttpParam+`, `body?: object` | What an arguments function returns. |

`HttpTool.arguments` is typed `<function ... />: HttpArguments`: any parameters, but the function
must return `HttpArguments`. A function with another result is a compile error. `body` is `object`,
so it holds one value of any type, usually a record you declare. To send a list, wrap it in a
record.

## A worked example

```nx
import "@nx/agent"

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

Evaluated, `root` is a plain record. Each function field is a function value, which a host can
call by name with arguments it has checked against the function's parameters.

## Extending the library

`Tool`, `ToolContext` and `Connection` are abstract so that a host library can extend them. A host
library imports `@nx/agent` like any module, and its subtypes are accepted wherever the base type
is expected:

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
export external component <AgentStep extends FlowStep
  agent?: Agent
  content prompt: string
/>
```

`nxlang typegen @nx/agent --language typescript --output <dir>` generates the library's types.
Generated code for a library that references them imports from the package `@nx-lang/agent`
(TypeScript) or qualifies them with the namespace `NxLang.Agent` (C#).

## What the host decides

The library says what an agent is, not how it runs. A host decides:

- which models `model` may name, and what an omitted `model` means;
- each tool's default `name` and `description`, and its input and output schemas;
- how tool calls are validated, executed, retried and limited;
- how a `ToolContext` is filled in, and how an `HttpConnection` is authenticated.

## The library source

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

## See also
- Reference: [Modules](/reference/syntax/modules#standard-libraries)
- Reference: [Function types](/reference/syntax/functions#functions-as-values)
