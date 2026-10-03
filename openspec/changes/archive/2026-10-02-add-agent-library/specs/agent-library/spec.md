## Purpose

Defines `@nx/agent`, the standard library of product-neutral types with which an NX program declares
an AI agent, its reference documents and its tools, so that hosts and their own libraries share one
vocabulary and extend it rather than each declaring their own.

## ADDED Requirements

### Requirement: The agent library is the standard library `@nx/agent`
The system SHALL provide a standard library with root `@nx/agent` and one module,
`@nx/agent/agent.nx`, whose exports are exactly the types `Agent`, `Document`, `AgentLimits`, `Tool`,
`FunctionTool`, `WebSearchTool`, `ToolContext`, `Connection`, `HttpConnection`, `HttpMethod`,
`HttpParam`, `HttpArguments` and `HttpTool`, together with the companions NX derives for each. The
library SHALL export no function and no component, SHALL be `unstable`, and every exported type and
every field SHALL carry a `///` doc comment. The library SHALL build with no diagnostics, including
no doc-comment warnings. Because two of its fields are typed by a function reference type, the library module's NX IR
image SHALL list the required feature `function-reference-type-v1`, and an entry image SHALL list
that feature only when its own type table holds the type.

#### Scenario: The library exports exactly its types
- **WHEN** the standard library `@nx/agent` is analyzed
- **THEN** its exported declarations, leaving out derived companions, SHALL be the thirteen types named above and nothing else
- **AND** analysis SHALL report no error, warning, info or hint

#### Scenario: The library image names the feature it needs
- **WHEN** `@nx/agent/agent.nx` is emitted from a program that imports the library
- **THEN** the image's required features SHALL include `function-reference-type-v1`
- **AND** a runtime that does not support that feature SHALL refuse the image naming the feature

#### Scenario: Every declaration is documented
- **WHEN** a client hovers any exported type of `@nx/agent`, or any field of one, in a document that imports the library
- **THEN** the hover SHALL show that declaration's documentation

### Requirement: `Agent`, `Document` and `AgentLimits` declare an agent
`Agent` SHALL be a record with a required `name:string`, optional `description:string`,
`model:string`, `documents:Document+`, `tools:Tool+` and `limits:AgentLimits`, and a required content
property `instructions:string`. `Document` SHALL be a record with a required `title:string` and a
required content property `text:string`. `AgentLimits` SHALL be a record with optional
`maxSteps:int` and `maxToolCalls:int`. No field SHALL carry a default. `Agent.model` SHALL be an
ordinary string that NX does not validate. Because `instructions` and `text` are content properties,
each SHALL be writable as the element's body, including as typed text with `@{}` interpolation, on an
element that also passes record-valued and sequence-valued properties as attributes.

#### Scenario: An agent is written with a markdown body and record-valued attributes
- **WHEN** a module that imports `@nx/agent` contains `let company = "Example"` and `let policy = <Document:markdown title="Refund policy">Refunds are available within 30 days.</Document>` and `let agent = <Agent:markdown name="support" model="balanced" documents={ policy } limits={ <AgentLimits maxSteps=6 /> }>You are the support assistant for @{company}.</Agent>`
- **THEN** analysis SHALL accept the module
- **AND** `agent.instructions` SHALL evaluate to `You are the support assistant for Example.`, `agent.documents` to a one-item sequence whose item has `title` `Refund policy` and `text` `Refunds are available within 30 days.`, and `agent.limits.maxSteps` to `6`

#### Scenario: Instructions are required
- **WHEN** a module contains `<Agent name="support" />`
- **THEN** analysis SHALL reject the element because `instructions` is missing

#### Scenario: Instructions can be passed by name
- **WHEN** a module contains `<Agent name="support" instructions="Be brief." />`
- **THEN** analysis SHALL accept the element

#### Scenario: Any model string is accepted
- **WHEN** a module contains `<Agent name="a" model="anything-at-all">Hi.</Agent>`
- **THEN** analysis SHALL accept the element
- **AND** the evaluated record's `model` SHALL be the string `anything-at-all`

#### Scenario: An agent evaluates to a plain record
- **WHEN** a `root` returning the agent of the first scenario is evaluated by an IR runtime
- **THEN** the result SHALL be a record whose `$type` is `Agent`, holding only the fields that were written

### Requirement: `Tool` is an abstract base that hosts extend
`Tool` SHALL be an abstract record with optional `name:string` and `description:string`.
`FunctionTool` SHALL extend `Tool` with a required `function` typed `<function ... />: object*`, the
function reference type of `function-reference-type` that a function of any parameters and any result satisfies. `WebSearchTool` SHALL extend `Tool` with an optional `allowedDomains:string+`.
The library SHALL NOT declare any authored field for a tool's schemas or annotations. A library or
program that imports `@nx/agent` SHALL be able to declare further record types that extend `Tool`,
with required fields, optional fields and fields that carry defaults, and a value of such a type
SHALL be accepted wherever a `Tool` is expected.

#### Scenario: A tool list mixes library and host tool types
- **WHEN** a host library declares `export type RecordSearchTool extends Tool = { recordKind:string maxResults:int = 5 }` and a program that imports both contains `let findPlans(teamSize:int): string* = { "Team" }` and `let tools: Tool+ = { <WebSearchTool allowedDomains={ "docs.example.com" } /> <FunctionTool function={findPlans} /> <RecordSearchTool recordKind="company" /> }`
- **THEN** analysis SHALL accept the program
- **AND** evaluating `tools` in an IR runtime SHALL return three records whose `$type` values are `WebSearchTool`, `FunctionTool` and `RecordSearchTool`, the last with `maxResults` `5`

#### Scenario: A function tool holds a reference to any function
- **WHEN** a program contains two functions with different parameter lists and result types, and a `FunctionTool` for each
- **THEN** analysis SHALL accept both elements
- **AND** each evaluated `function` field SHALL be the function reference record naming that function's module and name

#### Scenario: A function tool rejects a value that is not a function
- **WHEN** a program contains `<FunctionTool function="findPlans" />`
- **THEN** analysis SHALL reject the property because a string does not satisfy a function reference type

#### Scenario: A function reference cannot be forged in source
- **WHEN** a program contains `<FunctionTool function={ <Function module="main.nx" name="findPlans" /> } />`
- **THEN** analysis SHALL reject the property, because only a function value satisfies a function reference type

#### Scenario: `Tool` cannot be constructed
- **WHEN** a program contains `<Tool name="x" />`
- **THEN** analysis SHALL reject the element because `Tool` is abstract

### Requirement: `ToolContext` is an abstract base for host-supplied values
`ToolContext` SHALL be an abstract record with a required `callId:string`. A library or program that
imports `@nx/agent` SHALL be able to declare record types that extend it, and a function parameter
typed `ToolContext` SHALL accept a value of any such type.

#### Scenario: A host extends the tool context
- **WHEN** a host library declares `export type ChatToolContext extends ToolContext = { conversationId:string contactEmail?:string }` and a program declares `let whoAmI(ctx:ChatToolContext): string = { ctx.conversationId }`
- **THEN** analysis SHALL accept both
- **AND** calling `whoAmI` through an IR runtime with `ctx` set to a `ChatToolContext` record holding `callId` and `conversationId` SHALL return the conversation id

#### Scenario: A base-typed parameter accepts the host's subtype
- **WHEN** a program declares `let key(ctx:ToolContext): string = { ctx.callId }` and is called with a `ChatToolContext` record
- **THEN** the call SHALL return the record's `callId`

### Requirement: HTTP types describe one operation on a connection
`HttpMethod` SHALL be the constant union `get | post | put | patch | delete`. `Connection` SHALL be
an abstract record with a required `name:string` and an optional `description:string`.
`HttpConnection` SHALL extend `Connection` with a required `baseUrl:string` and SHALL declare no
field named `auth`. `HttpParam` SHALL be a record with required `name:string` and `value:string`.
`HttpArguments` SHALL be a record with optional `pathParams:HttpParam+`, `query:HttpParam+` and
`body:object`. `HttpTool` SHALL extend `Tool` with required `connection:HttpConnection`,
`method:HttpMethod` and `path:string`, a required `arguments` typed `<function ... />: HttpArguments`,
the function reference type of `function-reference-type` whose result is `HttpArguments`,
and an optional `maxCallsPerConversation:int`. A function of any parameters SHALL satisfy
`arguments` when it returns `HttpArguments`, and a function of another result SHALL NOT. NX SHALL
NOT validate `baseUrl`, `path` or the
relationship between a path's placeholders and the arguments; those are host checks.

#### Scenario: A read tool with a path parameter
- **WHEN** a program that imports `@nx/agent` contains `let shop = <HttpConnection name="shop" baseUrl="https://api.example.com" />` and `let lookupOrder(orderId:string): HttpArguments = <HttpArguments pathParams={ <HttpParam name="orderId" value={orderId} /> } />` and `let orderTool = <HttpTool connection={shop} method={HttpMethod.get} path="/orders/{orderId}" arguments={lookupOrder} />`
- **THEN** analysis SHALL accept the program
- **AND** calling `lookupOrder` through an IR runtime with `orderId` `A17` SHALL return an `HttpArguments` record whose `pathParams` holds one `HttpParam` with `name` `orderId` and `value` `A17`

#### Scenario: A body holds a record the program declares
- **WHEN** a program declares `type NewTicket = { subject:string priority:int }` and `let openTicket(subject:string): HttpArguments = <HttpArguments body={ <NewTicket subject={subject} priority=2 /> } />`
- **THEN** analysis SHALL accept the function
- **AND** calling it through an IR runtime SHALL return an `HttpArguments` record whose `body` is the `NewTicket` record

#### Scenario: Arguments with no body need no type argument
- **WHEN** a program contains `<HttpArguments query={ <HttpParam name="q" value="x" /> } />`
- **THEN** analysis SHALL accept the element with no type argument written

#### Scenario: An arguments function must return HttpArguments
- **WHEN** a program contains `let findPlans(teamSize:int): string* = { "Team" }` and `<HttpTool connection={shop} method={HttpMethod.get} path="/plans" arguments={findPlans} />`
- **THEN** analysis SHALL reject the `arguments` property as a result mismatch, naming the result `string*` and the expected result `HttpArguments`

#### Scenario: An unknown method is rejected
- **WHEN** a program contains `<HttpTool connection={shop} method={HttpMethod.head} path="/" arguments={lookupOrder} />`
- **THEN** analysis SHALL reject `HttpMethod.head` as not a case of `HttpMethod`

#### Scenario: A connection has no credentials field
- **WHEN** a program contains `<HttpConnection name="shop" baseUrl="https://api.example.com" auth="token" />`
- **THEN** analysis SHALL reject `auth` as an unknown property of `HttpConnection`
