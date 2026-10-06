## Purpose

The `@nx-lang/agent` TypeScript package, which sits on top of `@nx-lang/ir-runtime` and turns an
evaluated NX `Agent` value into tools a host can give to a model: normalized, storable tool
definitions in the MCP tool shape at compile time, and executable tools over a linked IR program at
run time. It owns schema assembly, naming, validation, function-tool execution and HTTP request
construction. It does not own the model loop, network policy, credentials or persistence.

## ADDED Requirements

### Requirement: The agent host package is published with separate compile-time and run-time entry points
The repository SHALL publish `@nx-lang/agent`, a TypeScript package that depends on
`@nx-lang/ir-runtime` and carries its type declarations. It SHALL expose four entry points:
`@nx-lang/agent` (the TypeScript types generated from the `@nx/agent` library, the definition types,
constants, naming helpers and the HTTP request builder),
`@nx-lang/agent/normalize` (the compile-time half), `@nx-lang/agent/execute` (the run-time half)
and `@nx-lang/agent/ai-sdk` (the AI SDK adapter). No entry point SHALL import `@nx-lang/sdk-wasm`, a
WebAssembly module or a Node built-in module, so each can be loaded in a Cloudflare Worker. Only
`@nx-lang/agent/ai-sdk` SHALL import `ai`, which SHALL be an optional peer dependency. The package
SHALL be marked unstable in its README and manifest description.

#### Scenario: A Worker imports the run-time half
- **WHEN** a Cloudflare Worker bundle imports `@nx-lang/agent/execute` and `@nx-lang/agent`
- **THEN** the bundle SHALL contain no WebAssembly module, no Node built-in import and no code from
  `@nx-lang/sdk-wasm` or `ai`

#### Scenario: A host that does not use the AI SDK installs the package
- **WHEN** a project installs `@nx-lang/agent` without `ai` and imports every entry point except
  `@nx-lang/agent/ai-sdk`
- **THEN** every import SHALL resolve and installation SHALL report no missing required peer

#### Scenario: The library's types are importable from the package
- **WHEN** a host library that references `Tool` and `ToolContext` is run through `nxlang typegen`
  for TypeScript, and the generated file imports those types from `@nx-lang/agent`
- **THEN** the import SHALL resolve to types named `Tool` and `ToolContext`
- **AND** the package's exported library types SHALL equal what `nxlang typegen @nx/agent` generates
  from the library source of the same release, with each relative import given its `.js` extension
  so that they resolve under Node's module resolution as well as a bundler's, which a test SHALL
  check

#### Scenario: The compile-time half takes schemas as data
- **WHEN** a host imports `@nx-lang/agent/normalize`
- **THEN** the import SHALL load no compiler, and the host SHALL supply declaration schemas through
  an option rather than the package loading them

### Requirement: The public API has fixed names
The package SHALL export the following under these names, and hosts SHALL be able to rely on them.

From `@nx-lang/agent`: the generated `@nx/agent` types under their NX names; the types
`NormalizedAgent` and `NormalizedTool`; `AGENT_DEFINITION_FORMAT_VERSION`; `NX_AGENT_TOOL_CONTEXT`,
the reference `{ module: "@nx/agent/agent.nx", name: "ToolContext" }`; `toSnakeCase(name)`;
`buildHttpRequest(tool, httpArguments)`; and the error classes `NxAgentError` (configuration
errors, carrying diagnostics) and `NxAgentToolError` (a tool failure with a `code`).

From `@nx-lang/agent/normalize`: `normalizeAgent(agentValue, options)`, whose options are `schemas`
(required: the program artifact, or any object with the export's `functionSchema(ref, options)`),
`toolContextType` (the `{ module, name }` of the host's concrete `ToolContext` subtype), `toolTypes`
(host describers by `$type`) and `allowInsecureBaseUrl`. The package SHALL call
`schemas.functionSchema` with `{ hostSuppliedTypes: [NX_AGENT_TOOL_CONTEXT] }`.

From `@nx-lang/agent/execute`: `createAgentTools(definition, program, options)`, whose options are
`runtime` (the IR runtime's options, including `maxOperations`), `maxArgumentsSize` and
`maxContextSize` (the limits on a call's input), `request` (the HTTP request function) and
`executors` (host executors by kind); `checkAgentTools(definition, program)`, which
returns diagnostics; and `evaluateHttpArguments(tool, program, input, context, options)`, whose
options are `runtime`, `maxArgumentsSize` and `maxContextSize`. A tool's
`execute(input, context)` takes a context of `callId`, optional `context` (the host's tool context
record) and optional `signal`.

From `@nx-lang/agent/ai-sdk`: `toAiSdkTools(tools, options)`, whose options are `callId`, `context`,
`providerTool` and `onResult`.

A `NormalizedAgent` SHALL have `formatVersion`, `name`, `description?`, `model?`, `instructions`,
`documents` (`{ title, text }`), `limits` (`{ maxSteps?, maxToolCalls? }`) and `tools`. A
`NormalizedTool` SHALL have `name`, `title?`, `description`, `inputSchema`, `outputSchema?`,
`annotations`, `kind` and `config`. For `function`, `config` SHALL be `{ function, contextParameters }`;
for `http`, `{ arguments, contextParameters, connection: { name, baseUrl }, method, path,
pathPlaceholders, maxCallsPerConversation? }`; for `provider`, `{ provider: "web_search",
allowedDomains? }`; `function` and `arguments` SHALL be `{ module, name }`, and each context
parameter `{ name, type: { module, name } }`. In every definition, diagnostic and result the package
produces, an optional field that has no value SHALL be omitted and SHALL never be `null`.

Normalization and creation diagnostics SHALL use exactly these codes: `nx-agent-not-an-agent`,
`nx-agent-duplicate-document-title`, `nx-agent-duplicate-tool-name`, `nx-agent-invalid-tool-name`,
`nx-agent-missing-description`, `nx-agent-inexpressible-parameter`, `nx-agent-inexpressible-result`
(the only warning), `nx-agent-unknown-function`, `nx-agent-context-parameter`,
`nx-agent-unknown-tool-type`, `nx-agent-describer`, `nx-agent-http-base-url`, `nx-agent-http-path`,
`nx-agent-http-arguments-type`, `nx-agent-unsupported-format`, `nx-agent-invalid-definition`,
`nx-agent-missing-executor` and `nx-agent-missing-request-function`. A failed `execute` SHALL carry exactly one of the result codes
`invalid-input`, `invalid-context`, `evaluation-failed`, `resource-limit`, `invalid-request`,
`request-failed` or `aborted`, or the code of an `NxAgentToolError` a host function threw.

#### Scenario: A host passes its artifact and context type
- **WHEN** a host calls `normalizeAgent(value, { schemas: artifact, toolContextType: { module: "host/Chat.nx", name: "ChatToolContext" } })`
- **THEN** the package SHALL query `artifact.functionSchema` with `hostSuppliedTypes` holding
  `NX_AGENT_TOOL_CONTEXT`, and SHALL accept context parameters typed `ToolContext` or `ChatToolContext`

#### Scenario: An absent optional field is omitted
- **WHEN** an agent has no `description` and a tool has no `title` and no `outputSchema`
- **THEN** the serialized definition SHALL contain none of those keys, and no `null`

#### Scenario: Creation errors carry their codes
- **WHEN** `createAgentTools` is given a definition with an unsupported `formatVersion`, a tool that
  is not the shape the package writes, a missing function, a kind with no executor, or an `http`
  tool with no `request` option
- **THEN** the thrown `NxAgentError` SHALL carry `nx-agent-unsupported-format`,
  `nx-agent-invalid-definition`, `nx-agent-unknown-function`, `nx-agent-missing-executor` or
  `nx-agent-missing-request-function` respectively

### Requirement: An evaluated Agent normalizes to a storable definition
`normalizeAgent` SHALL accept the canonical value of an `@nx/agent` `Agent` record, as
`@nx-lang/ir-runtime` renders it, together with a source of function schemas, and SHALL return
either a normalized agent definition or error diagnostics, never a partial definition. The schema
source SHALL be anything with the `functionSchema(ref, options)` query that
`declaration-schema-export` defines on a program artifact, so a host passes the artifact it built.
The definition SHALL carry a `formatVersion`, the agent's `name`, `description`, `model` (verbatim,
as an opaque string), `instructions`, `documents` (each with `title` and `text`, in authored order),
`limits` (as authored, uncapped) and `tools`, one or more normalized tool definitions per authored
tool, in authored order. The definition SHALL be plain JSON: serializing it with `JSON.stringify`
and parsing it back SHALL yield an equal definition that the run-time half accepts. Normalization
SHALL be deterministic: the same value and schemas SHALL produce the same definition, with object
keys in a stable order. An optional field that is empty in the value SHALL be absent from the
definition, and an agent with no tools SHALL normalize to an empty `tools` array.

#### Scenario: An agent with a function tool normalizes
- **WHEN** a program declares `/// Finds the plans that fit a team.` above `let findPlans(teamSize:int, maxMonthlyPrice?:int): Plan* = ...` and an `Agent` named `support` whose `tools` hold `<FunctionTool function={findPlans} />`
- **AND** the host evaluates the agent and passes the value and the program artifact to `normalizeAgent`
- **THEN** the result SHALL be a definition whose `name` is `support` and whose `tools` hold one
  definition of `kind` `function` named `find_plans`

#### Scenario: The definition survives storage
- **WHEN** a host stores `JSON.stringify(definition)` and later parses it
- **THEN** the parsed value SHALL deep-equal the original and SHALL be accepted by `createAgentTools`

#### Scenario: An invalid agent yields no definition
- **WHEN** normalization reports at least one error diagnostic
- **THEN** the result SHALL carry the diagnostics and no definition

#### Scenario: A value that is not an Agent is refused
- **WHEN** `normalizeAgent` is given a value whose `$type` is not `Agent`
- **THEN** it SHALL return an error diagnostic naming the `$type` it received

#### Scenario: Duplicate document titles are rejected
- **WHEN** an agent's `documents` hold two documents titled `Refund policy`
- **THEN** normalization SHALL fail with a diagnostic naming the title

### Requirement: A normalized tool follows the MCP tool shape
Every normalized tool definition SHALL carry `name`, `description`, `inputSchema`, `annotations`,
`kind` and `config`, and MAY carry `title` and `outputSchema`. `config` SHALL hold everything the
tool's executor needs, as plain JSON whose shape depends on `kind`. `inputSchema` and `outputSchema`
SHALL be JSON Schema draft 2020-12. `inputSchema` SHALL always describe a JSON object;
`outputSchema` SHALL be the schema of the tool's output as it is, of any JSON type, and the package
SHALL NOT wrap an output or its schema in an object. `annotations` SHALL carry the four boolean
hints `readOnlyHint`, `destructiveHint`, `idempotentHint` and `openWorldHint`. `kind` SHALL be
`function`, `http`, `provider` or a kind a host describer supplies. The package's own tool types
SHALL NOT set `title`.

#### Scenario: A definition has the MCP fields
- **WHEN** the `find_plans` tool above is normalized
- **THEN** its definition SHALL have `name`, `description`, `inputSchema`, `outputSchema`,
  `annotations`, `kind: "function"` and a `config` naming the function, and no `execute`

#### Scenario: An array result is not wrapped
- **WHEN** a function tool's function returns `Plan*`
- **THEN** the tool's `outputSchema` SHALL describe an array, and its output SHALL be that array

### Requirement: Model-facing tool names are snake_case and unique
A tool's name SHALL be its authored `name` when one is given, used verbatim. Otherwise a
`FunctionTool` SHALL be named after its function and an `HttpTool` after its arguments function,
converted to snake_case: a boundary SHALL be placed before an upper-case letter that follows a
lower-case letter or digit, and before the last upper-case letter of a run that is followed by a
lower-case letter; every character that is not an ASCII letter or digit SHALL become a boundary;
boundaries SHALL be written as single underscores and letters lower-cased. A `WebSearchTool` SHALL
default to `web_search`. Every name, authored or derived, SHALL match
`^[a-z][a-z0-9_]{0,63}$`, and a name that does not SHALL be an error diagnostic. Two tools of one
agent with the same name SHALL be an error diagnostic naming the name and both tools' positions.

#### Scenario: A camelCase function name is converted
- **WHEN** a `FunctionTool` with no `name` references `findPlans`
- **THEN** the tool's name SHALL be `find_plans`

#### Scenario: Acronyms and element-style names are converted
- **WHEN** tools with no `name` reference functions named `HTTPStatusFor`, `PlanRow` and `lookupOrder2`
- **THEN** their names SHALL be `http_status_for`, `plan_row` and `lookup_order2`

#### Scenario: An authored name is kept
- **WHEN** a `FunctionTool` has `name="plans"`
- **THEN** the tool's name SHALL be `plans`

#### Scenario: An authored name that is not snake_case is rejected
- **WHEN** a `FunctionTool` has `name="Find Plans"`
- **THEN** normalization SHALL fail with a diagnostic naming `Find Plans` and the pattern

#### Scenario: Duplicate names are rejected
- **WHEN** an agent's tools hold two `FunctionTool` elements that both resolve to `find_plans`
- **THEN** normalization SHALL fail with one diagnostic naming `find_plans` and the positions of both

### Requirement: Tool descriptions come from the author or the function's doc comment
A tool's description SHALL be its authored `description` when one is given. Otherwise a
`FunctionTool` SHALL take the documentation of its function and an `HttpTool` the documentation of
its arguments function, as the declaration schema export reports it. A tool that has neither, or
whose description is empty after trimming white space, SHALL be an error diagnostic naming the tool.
A parameter's documentation SHALL become the `description` of its property in the input schema; a
parameter with no documentation SHALL NOT be a diagnostic.

#### Scenario: The doc comment is the default description
- **WHEN** `findPlans` is documented `/// Finds the plans that fit a team.` and its tool has no `description`
- **THEN** the tool's description SHALL be `Finds the plans that fit a team.`

#### Scenario: An authored description wins
- **WHEN** the same tool has `description="Search plans."`
- **THEN** the tool's description SHALL be `Search plans.`

#### Scenario: A tool with no description is rejected
- **WHEN** a `FunctionTool` references an undocumented function and has no `description`
- **THEN** normalization SHALL fail with a diagnostic naming the tool

#### Scenario: Parameter documentation reaches the schema
- **WHEN** `findPlans` documents `teamSize` as `/// Number of people who need a seat.`
- **THEN** the input schema's `teamSize` property SHALL carry that text as its `description`

### Requirement: Tool schemas come from the declaration schema export
For a `FunctionTool` and an `HttpTool`, the package SHALL ask the schema source for the schema of
the function the tool's `Function` record names, by that record's `module` and `name`, listing
`ToolContext` of `@nx/agent/agent.nx` as a host-supplied type. `inputSchema` SHALL be the input
schema the export answers, unchanged: an object schema with one property per model-supplied
parameter in declaration order, `required` listing each parameter that has neither the `?` mark nor
a default, and `additionalProperties` `false`. A `FunctionTool`'s `outputSchema` SHALL be the output
schema the export answers, unchanged. When the export answers no input schema, each of its
diagnostics SHALL become an error diagnostic naming the tool and keeping the export's message, which
names the parameter and its type. When the export answers no output schema for a `FunctionTool`,
`outputSchema` SHALL be absent and normalization SHALL report a warning. When the export reports
`schema-unknown-declaration`, normalization SHALL report an error diagnostic naming the module and
the function. A diagnostic about a tool's function SHALL carry the function's module identity and
the declaration span the export answers, so a host can point an editor at the function; it SHALL
omit the span only when the export does.

The package SHALL store and forward a schema as the export wrote it. It SHALL NOT read a schema,
rewrite one or leave a keyword out, so that whatever the export writes for a type, the keywords of
a constrained type that a later NX adds included, reaches the stored definition and the model
unchanged.

#### Scenario: Required and optional parameters
- **WHEN** `findPlans(teamSize:int, maxMonthlyPrice?:int): Plan*` is normalized
- **THEN** the input schema's `properties` SHALL be `teamSize` then `maxMonthlyPrice`, `required`
  SHALL be `["teamSize"]`, and `additionalProperties` SHALL be `false`
- **AND** the output schema SHALL describe an array of `Plan`

#### Scenario: A parameter with a default is not required
- **WHEN** a function declares `limit:int = 10`
- **THEN** `limit` SHALL be a property and SHALL NOT be in `required`

#### Scenario: A function-typed parameter is rejected
- **WHEN** a `FunctionTool` references `let apply(f: <function n:int />: int, n:int): int = ...`
- **THEN** normalization SHALL fail with a diagnostic naming the tool and the parameter `f`

#### Scenario: A function the program does not declare is rejected
- **WHEN** a tool's `Function` record names `main.nx` and `findPlans`, and the schema source reports
  `schema-unknown-declaration` for it
- **THEN** normalization SHALL fail with a diagnostic naming `main.nx` and `findPlans`

#### Scenario: A schema keyword the package does not know is kept
- **WHEN** a schema source answers an input schema whose `teamSize` property is
  `{ "type": "integer", "minimum": 1, "maximum": 10 }` and an output schema that holds a `pattern`
- **THEN** the tool's `inputSchema` and `outputSchema` SHALL equal what the source answered
- **AND** the AI SDK adapter SHALL give the SDK that input schema unchanged

### Requirement: ToolContext parameters are filled by the host and hidden from the model
A function parameter that the schema export reports as host-supplied by `ToolContext`, which is one
whose declared type is the abstract `ToolContext` of `@nx/agent` or a record that extends it, SHALL
be a context parameter. A context parameter SHALL NOT appear in `inputSchema`, and the tool's
`config` SHALL record each context parameter's name and its declared type, by module identity and
name as the export's parameter entry gives it. `ToolContext` is abstract, so the host supplies a
record of one concrete subtype it declares. `normalizeAgent` SHALL accept that subtype, by module
identity and name, as an option, and SHALL report an error diagnostic naming the tool and the
parameter when a context parameter's declared type is neither `ToolContext` nor that subtype, or
when the host named no subtype.

A parameter that is not a context parameter and whose type holds `ToolContext` or a record that
extends it, as under a `+` occurrence or as a field of a record the function takes, SHALL be an
error diagnostic naming the tool and the parameter, because the model would otherwise be asked for
the host's context. The package SHALL learn this from the export, which reports on a parameter's
entry each host-supplied type its type holds.

The package SHALL decide which parameters are context parameters, and which hold one, only from
what the export reports of a parameter: that it is host-supplied, the host-supplied types it holds,
and its type reference. It SHALL NOT read the type's spelling or look inside a schema, so that a
form of type the language gains later, a constrained type for one, needs no change to the package.

#### Scenario: A context parameter is left out of the schema
- **WHEN** a host library declares `type ChatToolContext extends ToolContext = { conversationId:string }`, the host names `ChatToolContext` as its context type, and a tool's function is `let lookupOrder(orderId:string, context:ChatToolContext): HttpArguments = ...`
- **THEN** the input schema's only property SHALL be `orderId`
- **AND** the tool's `config` SHALL record a context parameter named `context` of type
  `ChatToolContext`

#### Scenario: A context parameter declared through an alias is a context parameter
- **WHEN** a program declares `type Context = ChatToolContext` and a tool's function declares
  `context:Context`
- **THEN** the input schema SHALL NOT have the property `context`
- **AND** the tool's `config` SHALL record a context parameter named `context` of type
  `ChatToolContext`

#### Scenario: A function with only a context parameter takes no input
- **WHEN** a tool's function is `let whoAmI(context:ToolContext): string = ...`
- **THEN** the input schema SHALL be an object schema with no properties

#### Scenario: A context parameter the host cannot fill is rejected
- **WHEN** a tool's function declares `context:ChatToolContext` and the host named no context type
- **THEN** normalization SHALL fail with a diagnostic naming the tool and `context`

#### Scenario: A sequence of contexts is rejected
- **WHEN** a tool's function declares `contexts:ChatToolContext+`
- **THEN** normalization SHALL fail with a diagnostic naming the tool and `contexts`

#### Scenario: A context held in another record is rejected
- **WHEN** a program declares `type Request = { orderId:string context:ChatToolContext }` and a
  tool's function declares `request:Request`
- **THEN** normalization SHALL fail with a diagnostic naming the tool and `request`

#### Scenario: A context of a subtype the host did not name is still found
- **WHEN** a library declares `type AuditToolContext extends ToolContext = { actor:string }`, the
  host names `ChatToolContext` as its context type, and a tool's function declares
  `contexts:AuditToolContext+`
- **THEN** normalization SHALL fail with a diagnostic naming the tool and `contexts`

### Requirement: Annotations are fixed per tool and never authored
The package SHALL set a tool's annotations from its type and, for an `HttpTool`, its method. A
`FunctionTool` SHALL be read-only, non-destructive, idempotent and closed-world. A `WebSearchTool`
SHALL be read-only, non-destructive, idempotent and open-world. An `HttpTool` SHALL be open-world,
and: `get` SHALL be read-only, non-destructive and idempotent; `put` and `delete` SHALL be
not read-only, destructive and idempotent; `post` and `patch` SHALL be not read-only, destructive
and not idempotent. Nothing an author writes SHALL change a tool's annotations.

#### Scenario: A function tool is read-only
- **WHEN** a `FunctionTool` is normalized
- **THEN** its annotations SHALL be `{ readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false }`

#### Scenario: A POST tool is a non-idempotent write
- **WHEN** an `HttpTool` with `method={HttpMethod.post}` is normalized
- **THEN** its annotations SHALL be `{ readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: true }`

#### Scenario: A GET tool is read-only
- **WHEN** an `HttpTool` with `method={HttpMethod.get}` is normalized
- **THEN** its `readOnlyHint` SHALL be `true` and its `destructiveHint` SHALL be `false`

### Requirement: WebSearchTool is described as a provider tool and never executed
A `WebSearchTool` SHALL normalize to a tool of `kind` `provider` named `web_search` by default, with
a built-in default description, an empty-object input schema, no output schema, the provider tool
identifier `web_search` and its `allowedDomains` in `config`. The run-time half SHALL return
it as a tool with no `execute`, and the package SHALL never perform a web search.

#### Scenario: A web search tool carries its domains
- **WHEN** `<WebSearchTool allowedDomains={ "docs.example.com" } />` is normalized
- **THEN** the definition SHALL have `name: "web_search"`, `kind: "provider"` and a `config` of
  `{ provider: "web_search", allowedDomains: ["docs.example.com"] }`

#### Scenario: A provider tool has no execute
- **WHEN** `createAgentTools` is given that definition
- **THEN** the tool it returns for `web_search` SHALL have no `execute` function

### Requirement: An HttpTool normalizes to a fixed operation on its connection
An `HttpTool` SHALL normalize to a tool of `kind` `http` whose `config` records the connection's
`name` and `baseUrl` (with a trailing `/` removed), the method, the path template, the placeholder
names of the path, each once, in the order they first appear, the arguments function's `Function` record, its context parameters and
`maxCallsPerConversation` when authored. Its `outputSchema` SHALL describe an object with a required
integer `status` and a `body` of any JSON type, and SHALL admit further properties, so a host's
request function can add its own. Normalization SHALL report an error diagnostic when:

- `baseUrl` is not an absolute URL, has a scheme other than `https`, has a query, a fragment or
  user information, or is not written the way a URL parser writes it (a host name in upper case, a
  default port, a dot segment). A request is that text with a path appended and is refused when
  parsing would change it, so such a base URL would fail every call; the diagnostic SHALL name
  the spelling to write. A host option SHALL allow the `http` scheme for local testing; it SHALL
  be off by default.
- the path does not begin with `/`, contains a character outside the URL path characters and
  `{name}` placeholders, has a `%` that does not begin a percent-encoded byte, has a placeholder
  whose name is not an identifier, has unbalanced braces, or has a literal segment that is `.` or
  `..` in any spelling a URL parser reads as one, `%2e` for a dot included. A placeholder MAY
  appear more than once and is filled each time from the one path parameter of its name.

Normalization SHALL NOT check the arguments function's result type: NX checks it where the program
binds the function, because `HttpTool.arguments` is typed `<function ... />: HttpArguments`, and
each call checks the value the function returned. The package SHALL NOT evaluate the arguments function, resolve the host name or contact the connection during normalization. Whether the function returns a path
parameter for every placeholder depends on its input and is checked at each call.

#### Scenario: An HTTP tool is normalized
- **WHEN** `<HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />` is normalized, `shop` being `<HttpConnection name="shop" baseUrl="https://api.example.com" />`
- **THEN** the definition SHALL have `kind: "http"`, name `lookup_order`, and a `config` holding
  method `get`, path `/orders/{orderId}`, placeholders `["orderId"]` and the connection's name and
  base URL

#### Scenario: A non-HTTPS base URL is rejected
- **WHEN** a connection's `baseUrl` is `http://api.example.com`
- **THEN** normalization SHALL fail with a diagnostic naming the connection and the scheme

#### Scenario: A base URL with a query is rejected
- **WHEN** a connection's `baseUrl` is `https://api.example.com/v1?key=1`
- **THEN** normalization SHALL fail with a diagnostic naming the connection

#### Scenario: A base URL a parser would rewrite is rejected
- **WHEN** a connection's `baseUrl` is `https://API.example.com`
- **THEN** normalization SHALL fail with a diagnostic naming the connection and the spelling
  `https://api.example.com`

#### Scenario: A path with a dot segment is rejected
- **WHEN** an `HttpTool` has `path="/orders/../admin"`
- **THEN** normalization SHALL fail with a diagnostic naming the tool and the path

#### Scenario: An arguments function with the wrong result type is rejected by the compiler
- **WHEN** a program binds a function that returns `string` to an `HttpTool`'s `arguments`
- **THEN** the program SHALL fail to compile with a result mismatch naming `HttpArguments`, so no `Agent` value reaches normalization

### Requirement: Normalization diagnostics are structured
Every diagnostic `normalizeAgent` reports SHALL carry a `severity`, a stable `code` beginning with
`nx-agent-`, a `message`, and a `path` locating the value inside the agent (such as `tools[1]`), and
SHALL carry the tool's name when one was resolved. Normalization SHALL collect every diagnostic it
can find in one pass rather than stopping at the first. A result with only warnings SHALL still
carry the definition and the warnings.

#### Scenario: Two problems are reported together
- **WHEN** an agent has one tool with no description and two other tools with the same name
- **THEN** the result SHALL carry both diagnostics, each with its own code and path

#### Scenario: A warning does not block the definition
- **WHEN** the only diagnostic is the warning for an inexpressible result type
- **THEN** the result SHALL carry the definition and that warning

### Requirement: A host describes its own Tool subtypes
`normalizeAgent` SHALL accept host describers keyed by the `$type` of a `Tool` subtype. For a tool
value of a registered type the package SHALL call the describer with the value and SHALL accept
from it one or more tool descriptions (name, optional title, description, input schema, optional
output schema, annotations, a kind, and a JSON `config`) or diagnostics. The package SHALL apply
the same name pattern, uniqueness, description and JSON-serializability rules to described tools as
to its own, and SHALL let the authored `name` and `description` fields override a single described
tool's defaults. A tool value whose `$type` is neither one of the package's own nor registered
SHALL be an error diagnostic naming the type. A describer SHALL NOT be able to replace the
package's handling of `FunctionTool`, `HttpTool` or `WebSearchTool`.

#### Scenario: A host tool type is described
- **WHEN** a host registers a describer for `RecordSearchTool` that returns a tool named
  `search_records` of kind `builtin`, and an agent's tools hold a `RecordSearchTool`
- **THEN** the definition SHALL hold that tool with the describer's schema, annotations, kind and
  `config`

#### Scenario: One element resolves to two tools
- **WHEN** a describer returns two descriptions for one tool element
- **THEN** the definition SHALL hold both, in the order returned, at that element's position

#### Scenario: An unregistered tool type is rejected
- **WHEN** an agent's tools hold a `MeetingSchedulerTool` and no describer is registered for it
- **THEN** normalization SHALL fail with a diagnostic naming `MeetingSchedulerTool`

#### Scenario: A described tool collides with a derived name
- **WHEN** a describer returns a tool named `find_plans` and the agent also has a `FunctionTool`
  that resolves to `find_plans`
- **THEN** normalization SHALL fail with the duplicate-name diagnostic

### Requirement: Executable tools are created from a stored definition and a linked program
`createAgentTools(definition, program, options)` SHALL accept a normalized agent definition (or its
tool list), a linked `NxPreparedProgram` and host options, and SHALL return one tool per definition,
in order, each carrying the definition's fields unchanged plus, for every kind except `provider`, an
`execute(input, context)` function. It SHALL NOT need declaration schemas, the compiler or NX
source. It SHALL throw an error carrying diagnostics, before any tool runs, when the definition's
`formatVersion` is not one it supports, when the definition or one of its tools does not have the
shape the package writes, when a `function` or `http` tool names a function the linked
program does not declare, when a definition has a kind with no executor available, or when an
`http` tool is present and the host supplied no request function. The package SHALL also export
`checkAgentTools`, a check that reports the first three of those problems as diagnostics without needing executors or a
request function, so a host can verify a definition against the program it just compiled.

A definition is JSON the host stored, so its shape is not assumed. The package SHALL check every
member of a definition it reads before a call is made: that the definition is a record or a list,
that its `tools` is a list, that each tool is a record with a string `name` and `kind`, and that a
`function` or `http` tool has a `config` holding its function's module and name, a list of context
parameters each with a name and, for `http`, a connection with a name. A definition that fails the
check SHALL be reported with `nx-agent-invalid-definition` and the path of the member, and SHALL
NOT make the package throw anything but that error. `evaluateHttpArguments`, which is given one
tool, SHALL answer a failure with code `invalid-request` for a tool that fails it, and
`buildHttpRequest` SHALL answer its failure, and SHALL NOT throw, for a tool that is not a record
or is a record that is not an `http` tool, naming the tool only when it has a name that is a
string.

`execute`'s `context` SHALL carry a `callId` string that the host chooses, MAY carry the host's tool
context record and an abort signal, and `execute` SHALL return a promise of either a success holding
the output or a failure holding an error with a `code`, a `message` and, where the IR runtime
produced them, its diagnostics. `execute` SHALL NOT reject for a failure of the tool itself. A call
whose signal is already aborted SHALL fail with code `aborted` without evaluating anything. A call
with no `callId`, or an empty one, SHALL fail with code `invalid-context`. The package SHALL NOT
generate, alter or remember a `callId`.

#### Scenario: Tools are created without the compiler
- **WHEN** a Durable Object holds a parsed definition and the linked program of the same compiled
  config, and calls `createAgentTools`
- **THEN** it SHALL receive executable tools without importing `@nx-lang/sdk-wasm`

#### Scenario: An unknown format version is refused
- **WHEN** the definition's `formatVersion` is one the installed package does not support
- **THEN** `createAgentTools` SHALL throw an error naming the version it was given and the versions
  it supports

#### Scenario: A damaged definition is reported, not thrown
- **WHEN** a stored definition with a supported `formatVersion` holds `tools: [null]`, or a
  `function` tool with no `config`, or one whose `config` has no `contextParameters`
- **THEN** `checkAgentTools` SHALL answer one diagnostic with code `nx-agent-invalid-definition`
  and the path `tools[0]`, and `createAgentTools` SHALL throw an `NxAgentError` carrying it
- **AND** a definition whose `tools` is not a list SHALL be reported at the path `tools`, and SHALL
  NOT be read as a definition with no tools

#### Scenario: A function missing from the program is refused at creation
- **WHEN** a `function` tool's `config` names `main.nx` and `findPlans`, and the linked program
  holds no module `main.nx` or no function `findPlans` in it
- **THEN** `createAgentTools` SHALL throw an error naming the tool, the module and the function
- **AND** it SHALL return no tools

#### Scenario: A host checks a definition when it compiles
- **WHEN** a host calls the exported check with a definition holding `function`, `http` and
  host-kind tools and the program it just linked, with no executors and no request function
- **THEN** the check SHALL return no diagnostics when every named function is in the program

#### Scenario: A kind with no executor is refused at creation
- **WHEN** the definition holds a tool of kind `builtin` and the host registered no executor for it
- **THEN** `createAgentTools` SHALL throw an error naming the tool and the kind

#### Scenario: An aborted call does nothing
- **WHEN** `execute` is called with a signal that is already aborted
- **THEN** it SHALL resolve to a failure with code `aborted` and SHALL NOT call the function

### Requirement: A function tool runs its function under the evaluation budget
A `function` tool's `execute` SHALL call the function its definition names through the IR runtime's
`callFunction`, with the model's input as arguments by parameter name and every context parameter
filled by the host. The input SHALL be a JSON object, and anything else SHALL fail with code
`invalid-input`. The IR runtime's boundary validation SHALL be what checks the input against the
parameter types; the package SHALL NOT need a JSON Schema validator. A key of the input that names
a context parameter SHALL be discarded before the call, so the model cannot supply one.
A member of the host's context record that is `undefined`, at any depth, SHALL be left out of the
record the function is passed, as an optional field with no value is, and the host's own record
SHALL NOT be changed. The context record itself SHALL be read as the record its own members
make, whatever built it, an instance of a host's class included. Below it only a plain record is
looked into for such members, as the IR runtime's input measure looks into one; a value that is
neither a list nor a plain record, a typed array for one, SHALL be passed as it is. Leaving members out SHALL NOT change which records are refused or how: a
record over `maxContextSize`, one that holds itself and one nested however deeply SHALL fail with
code `resource-limit` naming `maxContextSize`, and SHALL NOT make `execute` reject.

Every call SHALL run with a finite operation budget. The IR runtime applies none of its own when
`maxOperations` is unset, so the package SHALL pass the host's runtime options when they set
`maxOperations` and SHALL otherwise set it to the package's default of 100,000. Each call SHALL
get the whole budget; the package SHALL NOT share a budget across calls.

The result of every call that reached the IR runtime, a success or a failure, SHALL carry `usage`:
`operations`, the operations the call used, and `inputSize`, the size of its input, as the IR
runtime reports them. The package SHALL give each call a usage sink of its own, so that calls
started together each carry their own numbers, and SHALL NOT write to a `usage` in the host's
runtime options. A failure the package finds before it calls the runtime SHALL carry no `usage`;
a failure the runtime finds, a boundary failure on the arguments included, SHALL carry it. For an
HTTP tool the numbers SHALL be those of the call of its arguments function, on the result of
`execute` and of `evaluateHttpArguments` alike, and on a failure that follows that call (a request
that cannot be built, a request that fails) as on a success. A tool run by a host executor or by the provider SHALL carry
none.

Every call SHALL run with its input bounded, in two parts, each measured by the package with the
IR runtime's `measureInputSize` before the function is called. The model's input SHALL be held to
`maxArgumentsSize`, the host's value or the package's default of 1,000, and input over it SHALL
fail with code `invalid-input` carrying `limit: { name: "maxArgumentsSize", value }`. The host's
context record, as it is passed with `callId` set, SHALL be held to `maxContextSize`, the host's
value or the package's default of 1,000, and a context over it SHALL fail with code `resource-limit`
carrying `limit: { name: "maxContextSize", value }`. Neither failure SHALL call the function. The
call SHALL then run with the runtime's `maxInputSize` set by the package to the sum of
`maxArgumentsSize`; of `maxContextSize` and of what the parameter's name costs as a field name,
one for every 64 UTF-16 code units, for each context parameter of the function; and of the size of
the function record the call names, so that a call within both limits is never refused by the
runtime's. A `maxInputSize` in the host's runtime options SHALL NOT be used for
these calls. A host SHALL NOT be able to turn either limit off through the package. The same SHALL
hold for the call of an HTTP tool's arguments function, through `execute` and through
`evaluateHttpArguments`.

A success SHALL carry the function's canonical result as the runtime returns it. A failure of
evaluation SHALL carry the runtime's diagnostics and one of these codes: `invalid-input` when every
diagnostic is a boundary failure on the arguments, which is `nx-ir-arguments` or a code that begins
`nx-ir-boundary-`, as `nx-ir-boundary-type` and `nx-ir-boundary-field` do; `resource-limit` when
any diagnostic is `nx-ir-resource-limit`, with the diagnostic's `limit` carried on the error; and
`evaluation-failed` otherwise. The package SHALL match the family of boundary codes and not a list
of them, so that a check a runtime adds at the boundary later, of a value against a constrained
type for one, is reported as input the model can correct with no change to the package. A host
SHALL NOT need to read IR diagnostic codes to tell these apart.

#### Scenario: An optional context field set to undefined is absent
- **WHEN** the host passes the context record `{ $type: "ChatToolContext", conversationId: "conv_9", contactEmail: undefined }`
  to a tool whose function declares `context:ChatToolContext`, where `contactEmail` is optional
- **THEN** the call SHALL succeed as it does with the member left out

#### Scenario: A context record that holds itself is refused, not followed
- **WHEN** the host passes a context record one of whose members is the record itself, or a record
  nested twenty thousand levels deep
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxContextSize`, and SHALL NOT reject

#### Scenario: A function tool returns the function's result
- **WHEN** the model calls `find_plans` with `{ "teamSize": 5 }`
- **THEN** `execute` SHALL call `findPlans` with `teamSize` bound to `5` and `maxMonthlyPrice` absent
- **AND** SHALL resolve to a success whose output is the canonical `Plan` array

#### Scenario: A result carries what the call used
- **WHEN** the model calls `find_plans` with `{ "teamSize": 5 }` and the call costs `n` operations
- **THEN** the success SHALL carry `usage` whose `operations` is `n` and whose `inputSize` is the size of the call's input

#### Scenario: A failure after the call began carries what it used
- **WHEN** a tool's function loops beyond the budget
- **THEN** the failure SHALL carry `usage` whose `operations` is no more than the budget

#### Scenario: Calls started together carry their own numbers
- **WHEN** a host starts two tool calls that cost different numbers of operations and awaits both
- **THEN** each result SHALL carry the operations of its own call

#### Scenario: A failure before the call carries no usage
- **WHEN** the model's input is over `maxArgumentsSize`, or is not an object, or the call was aborted before it began
- **THEN** the failure SHALL carry no `usage`

#### Scenario: A failure the runtime finds carries usage
- **WHEN** the model calls `find_plans` with `{ "teamSize": "five" }`
- **THEN** the `invalid-input` failure SHALL carry `usage`, since the call reached the runtime

#### Scenario: The host's own usage object is left alone
- **WHEN** the host creates the tools with runtime options that hold a `usage` object, and a tool call runs
- **THEN** the result SHALL carry the call's `usage`
- **AND** the host's object SHALL be as it was before the call

#### Scenario: An HTTP failure after the arguments function ran carries usage
- **WHEN** an HTTP tool's arguments function returns and the request then cannot be built, or the host's request function fails
- **THEN** the failure SHALL carry the `usage` of the arguments function's call

#### Scenario: Input of the wrong type is invalid input
- **WHEN** the model calls `find_plans` with `{ "teamSize": "five" }`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` carrying the runtime's
  boundary diagnostic, and SHALL NOT reject

#### Scenario: A missing required argument is invalid input
- **WHEN** the model calls `find_plans` with `{}`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` naming `teamSize`

#### Scenario: A boundary failure of a kind added later is invalid input
- **WHEN** the runtime fails a call with one diagnostic whose code is `nx-ir-boundary-constraint`,
  a code this package has no knowledge of
- **THEN** the failure SHALL have code `invalid-input` and SHALL carry that diagnostic
- **AND** a failure whose diagnostics are that one and `nx-ir-division-by-zero` SHALL have code
  `evaluation-failed`

#### Scenario: A runaway function hits the budget
- **WHEN** a tool's function loops beyond the budget
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxOperations`, and SHALL NOT hang

#### Scenario: Input that is too large is invalid input
- **WHEN** the model calls `find_plans` with an input object whose size is over `maxArgumentsSize`
- **THEN** `execute` SHALL resolve to a failure with code `invalid-input` whose `limit` is
  `{ name: "maxArgumentsSize", value: 1000 }`
- **AND** the function SHALL NOT have been called

#### Scenario: The host's context does not count against the model's cap
- **WHEN** the host passes a context record of size 900 and the model an input of size 900, under the defaults
- **THEN** the call SHALL proceed

#### Scenario: A context that is too large is a resource limit
- **WHEN** the host passes a context record whose size is one over `maxContextSize` and the model an input of size 5
- **THEN** `execute` SHALL resolve to a failure with code `resource-limit` whose `limit` names
  `maxContextSize`
- **AND** the function SHALL NOT have been called

#### Scenario: A function with two context parameters is given the context twice
- **WHEN** a tool's function has two context parameters and the host passes a context record of size 900 under the defaults
- **THEN** the call SHALL proceed: the runtime's limit allows the context once for each

#### Scenario: The host's own input limit is not used
- **WHEN** the host creates the tools with runtime options that set `maxInputSize: 10` and the model sends an input of size 50
- **THEN** the call SHALL proceed under the package's limits
- **AND** SHALL NOT fail with code `resource-limit`

#### Scenario: A host raises the limits through the package's options
- **WHEN** the host creates the tools with `maxArgumentsSize: 5000` and the model sends an input of size 3,000
- **THEN** the call SHALL proceed

#### Scenario: The arguments function runs under the same limits
- **WHEN** the model calls an HTTP tool, or the host calls `evaluateHttpArguments`, with an input over `maxArgumentsSize`
- **THEN** the result SHALL be a failure with code `invalid-input` whose `limit` names `maxArgumentsSize`
- **AND** the arguments function SHALL NOT have been called

#### Scenario: The package's default applies when the host sets no budget
- **WHEN** the host creates the tools with no runtime options
- **THEN** every function-tool and arguments-function call SHALL run under `maxOperations: 100000`

#### Scenario: The host's budget is used
- **WHEN** the host creates the tools with runtime options that set `maxOperations: 200000`
- **THEN** every function-tool and arguments-function call SHALL run under that budget

#### Scenario: A failure inside the function is an evaluation failure
- **WHEN** a tool's function fails at run time for a reason other than its arguments or a limit
- **THEN** `execute` SHALL resolve to a failure with code `evaluation-failed`

### Requirement: Context parameters are filled from the host's context record at each call
`execute` SHALL accept, in its call context, the host's tool context: a record of the concrete
`ToolContext` subtype the host declares, with its `$type` and fields. For each context parameter a
tool records, `execute` SHALL pass that record with `callId` set to the call's `callId`, replacing
any `callId` the record held. A tool with no context parameter SHALL ignore the record. When a tool
has a context parameter and the call supplies no record, or one with no string `$type`, `execute`
SHALL fail with code `invalid-context` without calling the function.

#### Scenario: The host supplies identity
- **WHEN** a tool's function declares `context:ChatToolContext` and `execute` is called with
  `callId: "t1:0:0"` and the record `{ $type: "ChatToolContext", conversationId: "conv_9" }`
- **THEN** the function SHALL receive `{ $type: "ChatToolContext", callId: "t1:0:0", conversationId: "conv_9" }`

#### Scenario: A base-typed parameter receives the host's subtype
- **WHEN** a tool's function declares `context:ToolContext` and the host supplies the same record
- **THEN** the function SHALL receive it, and `context.callId` SHALL be `t1:0:0`

#### Scenario: The model cannot supply the context
- **WHEN** the model's input is `{ "orderId": "A1", "context": { "conversationId": "other" } }`
- **THEN** the function SHALL receive the host's context record and SHALL NOT receive the model's

#### Scenario: A missing context record is reported
- **WHEN** a tool has a context parameter and `execute` is called with no context record
- **THEN** `execute` SHALL resolve to a failure with code `invalid-context` naming the parameter

### Requirement: An HTTP request is built structurally from the tool and its arguments
The package SHALL export a pure function, `buildHttpRequest`, that builds a request description from an `http` tool
definition and an evaluated `HttpArguments` value, with no I/O. The description SHALL carry the
method, taken from the tool and upper-cased; the URL; headers; and the body. The function SHALL:

- replace each `{name}` placeholder in the path with the value of the path parameter of that name,
  percent-encoding every character outside `A-Z a-z 0-9 - . _ ~`;
- fail when a placeholder has no parameter, when a path parameter names no placeholder, when a path
  parameter is given twice, when a value is empty, or when a substituted path segment is `.` or `..`
  in any spelling a URL parser reads as one;
- append the query parameters in order as `name=value` pairs joined by `&`, percent-encoding names
  and values the same way, and fail on an empty name;
- join the path to `baseUrl` by appending it to the base URL's path with exactly one `/` between
  them;
- verify that the result parses as a URL that a parser reads exactly as it was built: parsing does
  not change the text, the origin equals the base URL's origin, there is no user information, the
  path is the base URL's path followed by the path built, which places it under the base URL's
  path at a segment boundary, the query is the query built, and there is no fragment. Comparing the
  parts, and not the text alone, is what refuses a `?` or a `#` that was not encoded: it leaves the
  text unchanged and ends the path early, which would drop the fixed segments after it;
- for a body, which `HttpArguments.body:object` lets be any one value, serialize it as JSON with
  every `$type` key removed and set `content-type: application/json`; fail when a body is given
  and the method is `get` or `delete`; and set no other header.

Every failure SHALL be a structured error naming the tool and the rule. Nothing in the arguments
SHALL be able to change the scheme, host, port, method, or the fixed part of the path.

#### Scenario: A path parameter is substituted and encoded
- **WHEN** the tool is `get` `/orders/{orderId}` on `https://api.example.com` and the arguments hold path parameter `orderId` = `A 1/2`
- **THEN** the URL SHALL be `https://api.example.com/orders/A%201%2F2` and the method `GET`

#### Scenario: A base path is kept
- **WHEN** the base URL is `https://api.example.com/v1/` and the path is `/orders/{orderId}` with `orderId` = `7`
- **THEN** the URL SHALL be `https://api.example.com/v1/orders/7`

#### Scenario: Query parameters are encoded in order
- **WHEN** the arguments hold query parameters `q` = `a&b=c` and `page` = `2`
- **THEN** the URL SHALL end with `?q=a%26b%3Dc&page=2`

#### Scenario: A traversal value is refused
- **WHEN** the path is `/files/{name}` and `name` is `..`
- **THEN** the build SHALL fail and SHALL produce no request

#### Scenario: A missing path parameter is refused
- **WHEN** the path is `/orders/{orderId}` and the arguments hold no path parameters
- **THEN** the build SHALL fail naming `orderId`

#### Scenario: An undeclared path parameter is refused
- **WHEN** the path is `/orders` and the arguments hold path parameter `orderId`
- **THEN** the build SHALL fail naming `orderId`

#### Scenario: A body is sent as plain JSON
- **WHEN** a `post` tool's arguments hold body `{ "$type": "NewOrder", "sku": "X", "quantity": 2 }`
- **THEN** the body SHALL be the JSON text of `{ "sku": "X", "quantity": 2 }` and the headers SHALL
  be exactly `content-type: application/json`

#### Scenario: A body on GET or DELETE is refused
- **WHEN** a `get` or `delete` tool's arguments hold a body
- **THEN** the build SHALL fail naming the tool and the method

#### Scenario: A host rebuilds the request from stored configuration
- **WHEN** a server receives a tool name and evaluated arguments from another process, loads the
  tool's definition from its own storage and calls the builder
- **THEN** it SHALL get the same request description the first process would have built

### Requirement: An HTTP tool evaluates its arguments and calls the host's request function
An `http` tool's `execute` SHALL evaluate the arguments function exactly as a function tool is
evaluated (input by name, context parameters filled, under the budget, with the same failure
codes), build the request description, and call the request function the host supplied with: the
request description; the evaluated arguments in structured form, as `pathParams` and `query` lists
of `{ name, value }` and the `body` value as evaluated; the `callId`; the tool's name; the
connection's name; the tool's annotations; and the abort signal. The request function's result, an
object with a numeric `status` and a `body`, SHALL be the tool's output, with any further properties
the host put on it kept. The package SHALL NOT call `fetch` itself and SHALL NOT apply any network
policy: it SHALL NOT resolve or filter addresses, follow or refuse redirects, set timeouts, cap
sizes, add credentials or idempotency headers, retry, or count calls against
`maxCallsPerConversation`. A build failure SHALL fail the call with code `invalid-request` without
calling the request function. An error the request function throws SHALL fail the call with code
`request-failed`, unless it is the package's tool error type, whose code and message SHALL be kept.
Before it builds the request, the package SHALL verify that the value the arguments function
returned is a record whose `$type` is `HttpArguments`, and otherwise SHALL fail the call with code
`invalid-request`, naming the function and what it returned, without calling the request function.

The package SHALL also export the first two steps on their own, as `evaluateHttpArguments`: given an `http` tool definition,
the linked program, an input, a call context and options (the runtime's options and the two input
limits), it SHALL evaluate the arguments
function and build the request, and return the structured arguments and the request description or
the same failure `execute` would, without a request function and without sending anything.

#### Scenario: The host's function makes the call
- **WHEN** the model calls `lookup_order` with `{ "orderId": "A1" }` and the host's request function
  answers `{ status: 200, body: { state: "shipped" } }`
- **THEN** the request function SHALL have been called once with method `GET`, URL
  `https://api.example.com/orders/A1`, path parameters `[{ name: "orderId", value: "A1" }]` and the
  call's `callId`
- **AND** `execute` SHALL resolve to a success whose output is `{ status: 200, body: { state: "shipped" } }`

#### Scenario: A host forwards the arguments instead of the request
- **WHEN** the host's request function ignores the request description and posts the tool name,
  the structured arguments and the `callId` to another process, which builds the request itself
  from its own copy of the definition
- **THEN** the package SHALL have given it everything that needs, and whatever `{ status, body }`
  it returns SHALL be the output

#### Scenario: A non-2xx status is an output, not a failure
- **WHEN** the request function answers `{ status: 404, body: "not found" }`
- **THEN** `execute` SHALL resolve to a success with that output

#### Scenario: Extra result properties are kept
- **WHEN** the request function answers `{ status: 200, body: "…", bodyNote: "truncated" }`
- **THEN** the output SHALL include `bodyNote`

#### Scenario: A structural failure never reaches the network
- **WHEN** the arguments function returns a path parameter the path does not declare
- **THEN** `execute` SHALL resolve to a failure with code `invalid-request`
- **AND** the request function SHALL NOT have been called

#### Scenario: A function that does not return HttpArguments is refused at the call
- **WHEN** a stored `http` tool definition's arguments function, in the linked program, returns the string `"A1"`
- **THEN** `execute` and `evaluateHttpArguments` SHALL each resolve to a failure with code `invalid-request` naming the function
- **AND** the request function SHALL NOT have been called

#### Scenario: Arguments are evaluated without sending
- **WHEN** a host calls the exported evaluation with the `lookup_order` definition, the linked
  program, input `{ "orderId": "A1" }` and a call context, and supplies no request function
- **THEN** it SHALL receive path parameters `[{ name: "orderId", value: "A1" }]` and the request
  for `GET https://api.example.com/orders/A1`
- **AND** nothing SHALL have been sent

#### Scenario: A host policy error keeps its code
- **WHEN** the request function throws the package's tool error with code `unknown-outcome`
- **THEN** `execute` SHALL resolve to a failure with code `unknown-outcome`

#### Scenario: The package sets no idempotency header
- **WHEN** a `post` tool is executed
- **THEN** the request description's headers SHALL NOT contain `Idempotency-Key`
- **AND** the request function SHALL have received the `callId` so the host can set one

### Requirement: A host executes its own tool kinds
`createAgentTools` SHALL accept host executors keyed by `kind`. For a tool of a registered kind,
`execute` SHALL call the executor with the tool's definition (including the `config` its describer
stored), the input and the call context, and SHALL treat its return value as the output
and an error it throws as a failure, keeping the code of the package's tool error type. A host
executor SHALL NOT be able to replace the executors of `function` and `http`, and SHALL NOT be
consulted for `provider`.

#### Scenario: A host kind is executed
- **WHEN** the host registers an executor for `builtin` and the model calls `search_records`
- **THEN** the executor SHALL receive the stored `config`, the input and the `callId`
- **AND** its return value SHALL be the tool's output

#### Scenario: A host executor failure is a result
- **WHEN** the executor throws
- **THEN** `execute` SHALL resolve to a failure and SHALL NOT reject

### Requirement: Executable tools adapt to AI SDK tools
`@nx-lang/agent/ai-sdk` SHALL export `toAiSdkTools`, a function that maps executable tools to an AI SDK 7 tool set
keyed by tool name. A tool with `execute` SHALL become an AI SDK tool defined at run time, with the
definition's description, its `inputSchema` wrapped as an AI SDK JSON schema, and an `execute` that
calls the tool with the SDK's abort signal and a `callId`. The `callId` SHALL be the SDK's
`toolCallId` unless the host supplies a function that derives one, and the host SHALL be able to
supply the tool context record per call. A success SHALL return the output to the SDK, without the
result's `usage`; a failure SHALL
throw the package's tool error, so the SDK reports a tool error to the model. The SDK sends the
model the thrown error's message, so the message is what the model is told. The thrown error
SHALL carry the failure's code. It SHALL carry the failure's message, diagnostics and limit only
when the code is `invalid-input`, which the model can correct, or a code that is not one of the
package's result codes, which a host function chose and whose message is the host's to have
written for the model. For the package's other result codes it SHALL carry a fixed sentence that
names the tool and the kind of failure and nothing of its cause, and no diagnostics, no limit and
no cause, since the cause may hold what a model is not to see: the text a request function threw,
the runtime's account of a function, a connection's address. The sentence SHALL NOT claim what
the package does not know: for `request-failed` it SHALL NOT say the request was not carried
out. The whole failure SHALL still reach `onResult`. So that a host
using the adapter can still read what each call used, the host SHALL be able to supply a function,
`onResult`, that the adapter calls with the tool's name, the `callId` and the whole result, `usage`
included, once for every call of an executable tool, on a success and on a failure. The adapter
SHALL await what the function returns before it returns the output or throws the error, so that a
host that journals the result has written it before the SDK goes on. The adapter SHALL NOT catch
what the function throws, and a promise it returns that rejects SHALL be treated as a throw. A `provider` tool
SHALL be mapped by a function the host supplies, which returns the provider's own tool or `null` to
leave it out; a `provider` tool with no mapping SHALL be an error naming the tool, not a silent
omission. The adapter SHALL NOT call a model and SHALL NOT set strict mode, approval or any other
tool option.

#### Scenario: A host using the adapter reads what a call used
- **WHEN** a host passes `toAiSdkTools` a result function and the model calls `find_plans`
- **THEN** the function SHALL be called once with the tool's name, the `callId` and the result, whose `usage` holds the call's operations and input size
- **AND** the SDK SHALL be given the output alone

#### Scenario: The adapter waits for the result function
- **WHEN** a host's `onResult` returns a promise that resolves after it has stored the result
- **THEN** the adapted `execute` SHALL NOT return the output to the SDK until that promise has resolved
- **AND** if the promise rejects, the adapted `execute` SHALL reject with that error

#### Scenario: The result function sees a failure too
- **WHEN** a tool call made through the adapter fails for its budget
- **THEN** the result function SHALL be called with the failure and its `usage` before the tool error is thrown

#### Scenario: A function tool is given to streamText
- **WHEN** a host passes the adapted tool set to the AI SDK's `streamText` and the model calls `find_plans`
- **THEN** the SDK SHALL invoke the tool's `execute` with the parsed input
- **AND** the tool result the model receives SHALL be the function's canonical result

#### Scenario: A host-derived call identifier is used
- **WHEN** the host supplies a function that maps a tool call to `turn_7:1:0`
- **THEN** the tool's `execute` SHALL receive `callId: "turn_7:1:0"`

#### Scenario: A failure becomes a tool error
- **WHEN** a tool's `execute` resolves to a failure with code `resource-limit`
- **THEN** the adapted `execute` SHALL throw the package's tool error with that code

#### Scenario: The model is told why a call failed only when it can correct it
- **WHEN** an `http` tool's request function throws an error whose message holds an internal
  address, so the call fails with code `request-failed`
- **THEN** the error the adapter throws SHALL have the code `request-failed` and a message that
  names the tool and holds nothing of the thrown text, and SHALL carry no diagnostics
- **AND** what the SDK sends the model about the call SHALL hold nothing of the thrown text
- **AND** `onResult` SHALL be called with the failure and the message as thrown
- **AND** a call that fails with `invalid-input` SHALL throw an error with the failure's own
  message and diagnostics
- **AND** a failure with a code a host function chose, such as `order-not-found`, SHALL throw an
  error with the host's message

#### Scenario: A provider tool is mapped by the host
- **WHEN** the host's mapping returns its provider's web search tool for `web_search`
- **THEN** the tool set SHALL hold that tool under `web_search`

#### Scenario: An unmapped provider tool is an error
- **WHEN** the tools include `web_search` and the host supplies no provider mapping
- **THEN** the adapter SHALL throw an error naming `web_search`
