## MODIFIED Requirements

### Requirement: A built program exports the schema of a function
A built program SHALL answer, for a function named by the identity of the module that declares it
and its declared name, with that function's schema: the module identity and name, the function's
documentation, one entry per declared parameter in declaration order, an input schema and an output
schema. The pair of identity and name SHALL be the pair a canonical `Function` record carries, so a
host holding such a record can ask for its schema with no translation. A reference that gives no
module SHALL name the program's entry module. The function need not be exported or be an entrypoint,
and it MAY be declared in a library module the program links, named by that module's identity in the
program.

The input schema SHALL be a JSON Schema draft 2020-12 document whose root is an object schema with
one property per parameter, keyed by the parameter's name, in declaration order. A parameter SHALL
be listed in `required` unless it carries the `?` mark or declares a default. The root SHALL set
`additionalProperties` to `false`. A content parameter SHALL be an ordinary property under its name.
A function with no parameters SHALL have an input schema with no properties.

The output schema SHALL be a JSON Schema draft 2020-12 document for the function's result type, the
declared one when the source declares it and otherwise the type the checker inferred.

Each parameter entry SHALL give the parameter's name, its type in NX spelling, whether it is
required, and its documentation when it has any. When the parameter's declared type, apart from its
`?` mark, is a declared record or union, the entry SHALL also name that declaration by module
identity and name, so a host can ask for its type schema without parsing the spelling. A type alias
denotes its target: when the declared type is an alias of a record or union, through any number of
aliases, the entry SHALL name the record or union, and SHALL keep the alias in the spelling. An
alias of anything else, an occurrence for one, names no declaration.

The answer SHALL give the source location of the function's declaration, as the span of the whole
declaration in the declaring module's source in the shape a diagnostic label's span takes, whenever
the artifact holds that module's source text, so a host can point its own diagnostics about the
function at it. An artifact that does not hold the source SHALL answer without a location.

A reference that names no function in the program SHALL fail with a diagnostic of code
`schema-unknown-declaration` naming the module and the name. The export SHALL NOT evaluate the
program and SHALL NOT change the artifact, which SHALL remain usable afterwards.

#### Scenario: A function's arguments and result are described
- **WHEN** a host builds `type Plan = { name:string seats:int }` followed by
  `let findPlans(teamSize:int, maxMonthlyPrice?:int): Plan* = { ... }` and asks for the schema of
  `findPlans`
- **THEN** the input schema SHALL be an object schema with the properties `teamSize` and
  `maxMonthlyPrice`, each `{ "type": "integer" }`, with `required` equal to `["teamSize"]` and
  `additionalProperties` equal to `false`
- **AND** the output schema SHALL be an array schema whose `items` reference the schema of `Plan`
- **AND** the parameter entries SHALL be `teamSize` of type `int`, required, and `maxMonthlyPrice`
  of type `int`, not required

#### Scenario: The answer locates the declaration
- **WHEN** a host builds a workspace whose module `tools.nx` declares `findPlans` on lines 4 to 9
  and asks for the schema of `findPlans`
- **THEN** the answer's declaration location SHALL be a span in `tools.nx` that starts on line 4 and
  ends on line 9
- **AND** a host diagnostic labeled with that span SHALL mark the function in an editor showing
  `tools.nx`

#### Scenario: A parameter entry names its declared type
- **WHEN** a host asks for the schema of `let book(request?:Booking): string = { ... }` where
  `Booking` is declared in `types.nx`
- **THEN** the entry for `request` SHALL name the declaration `Booking` of module `types.nx`
- **AND** the entry for a parameter typed `string` SHALL name no declaration

#### Scenario: A parameter declared through an alias names what the alias denotes
- **WHEN** a program declares `type Order = { id:string }`, `type OrderAlias = Order` and
  `type Orders = Order+`, and a function declares `order:OrderAlias` and `orders:Orders`
- **THEN** the entry for `order` SHALL name the declaration `Order` and SHALL give its type as
  `OrderAlias`
- **AND** the entry for `orders` SHALL name no declaration

#### Scenario: A Function record names the function to describe
- **WHEN** evaluating a program yields `{ "$type": "Function", "module": "tools.nx", "name": "findPlans" }`
- **AND** the host asks the artifact that built that program for the schema of module `tools.nx`,
  name `findPlans`
- **THEN** the answer SHALL describe the function that record names

#### Scenario: A library function is described through the program that links it
- **WHEN** a program is built against a loaded library `libraries/catalog` whose module `Plans.nx`
  declares `findPlans`
- **AND** a host asks for the schema of module `libraries/catalog/Plans.nx`, name `findPlans`
- **THEN** the answer SHALL describe that function

#### Scenario: An element-style function is described by its parameter names
- **WHEN** a host asks for the schema of `let <Greeting name:string content body:string />: string = { ... }`
- **THEN** the input schema SHALL have the properties `name` and `body`, both required

#### Scenario: An unannotated result uses the inferred type
- **WHEN** a host asks for the schema of `let double(n:int) = { n * 2 }`
- **THEN** the output schema SHALL be `{ "type": "integer" }` beside the `$schema` keyword

#### Scenario: An unknown function is reported by name
- **WHEN** a host asks for the schema of `missing` in a program that declares no such function
- **THEN** the call SHALL fail with a `schema-unknown-declaration` diagnostic naming `missing`
- **AND** the artifact SHALL still emit NX IR afterwards

### Requirement: A host can mark parameter types as host-supplied
A request for a function's schema SHALL accept a list of types, each named by module identity and
name, whose parameters the host fills in itself. A parameter whose declared type is one of those
types, or a record that extends one of them, or a type alias that denotes either, SHALL be left out
of the input schema's properties and
`required`, and its parameter entry SHALL say it is host-supplied and name the listed type it
matched. Such a parameter SHALL NOT be checked for a JSON form, so a listed type that is abstract
and that nothing in the program extends SHALL still be matched and SHALL NOT be reported. A
parameter whose type merely
contains a listed type, as a field or under `+`, SHALL NOT be treated as host-supplied. A listed
type the program does not declare SHALL match nothing and SHALL NOT be an error, so a host can pass
one list for every program.

The entry of a parameter that is not host-supplied SHALL name each listed type its type holds, in
the order the host listed them and each once. A type holds a listed type when the listed type, or
a record that extends it, is reached from it at any depth: under an occurrence, as a field of a
record or of a payload case of a union, through a type alias, or as a type argument. A form of
type the language gains later that wraps or narrows another type SHALL be looked through the same
way. A record that is more than one of the listed types, as when one listed type extends another,
SHALL be named for each of them. A type derived from a listed type, its `.Update` record or its
`.Property` union, is another type: a parameter declared with one SHALL NOT be host-supplied and
SHALL NOT be counted as holding the listed type. The entry SHALL carry no such list when there is none to name, and a parameter that is
host-supplied SHALL carry none. Holding a listed type SHALL change nothing else in the answer: the
parameter stays in the input schema, and what to make of it is the caller's to decide. A type that
holds itself SHALL be looked into once.

#### Scenario: A context parameter is left out of the input schema
- **WHEN** a program imports `@nx/agent`, whose module `@nx/agent/agent.nx` declares the abstract
  record `ToolContext`, and declares
  `let lookupOrder(orderId:string, context:ToolContext): HttpArguments = { ... }`
- **AND** a host asks for the schema of `lookupOrder` and lists `ToolContext` of module
  `@nx/agent/agent.nx` as host-supplied
- **THEN** the input schema SHALL have the single property `orderId`
- **AND** the parameter entry for `context` SHALL be marked host-supplied by that `ToolContext`
- **AND** the answer SHALL carry no diagnostic for `context`, whether or not the program declares a
  record extending `ToolContext`

#### Scenario: A subtype of a listed type is host-supplied too
- **WHEN** a host library declares `export type ChatToolContext extends ToolContext = { conversationId:string contactEmail?:string }`
  and a parameter is typed `ChatToolContext`
- **AND** the host lists `ToolContext` as host-supplied
- **THEN** that parameter SHALL be left out of the input schema and marked host-supplied by
  `ToolContext`
- **AND** its entry SHALL name `ChatToolContext` as the parameter's declared type

#### Scenario: A result holding an object field is open at that field
- **WHEN** a host asks for the schema of that `lookupOrder`, whose result type `HttpArguments`
  declares `pathParams?:HttpParam+`, `query?:HttpParam+` and `body?:object`
- **THEN** the output schema's `HttpArguments` entry SHALL give `body` the schema of `object` and SHALL
  NOT list it in `required`

#### Scenario: Without the list every parameter is an argument
- **WHEN** the same function's schema is asked for with no host-supplied types
- **THEN** the input schema SHALL have the properties `orderId` and `context`

#### Scenario: An alias of a listed type is host-supplied too
- **WHEN** a program declares `type Context = ChatToolContext` and a function declares
  `context:Context`, and the host lists `ToolContext` as host-supplied
- **THEN** that parameter SHALL be left out of the input schema and marked host-supplied by
  `ToolContext`
- **AND** its entry SHALL name `ChatToolContext` as the parameter's declared type
- **AND** a parameter declared with `type Contexts = ChatToolContext+` SHALL NOT be host-supplied,
  and its entry SHALL name `ToolContext` as a listed type its type holds

#### Scenario: A listed type under an occurrence is named, and the parameter is kept
- **WHEN** a function declares `contexts:ChatToolContext+` and the host lists `ToolContext` as
  host-supplied
- **THEN** the input schema SHALL have the property `contexts`
- **AND** the parameter entry for `contexts` SHALL NOT be marked host-supplied and SHALL name
  `ToolContext` as a listed type its type holds

#### Scenario: A listed type held as a field is named, at any depth
- **WHEN** a program declares `type Request = { orderId:string context:ChatToolContext }` and
  `type Batch = { requests:Request+ }`, and a function declares `request:Request` and `batch:Batch`
- **AND** the host lists `ToolContext` as host-supplied
- **THEN** the entries for `request` and for `batch` SHALL each name `ToolContext` as a listed type
  its type holds

#### Scenario: A record that is two listed types is named for each
- **WHEN** a function declares `contexts:ChatToolContext+` and the host lists `ToolContext` and
  `ChatToolContext` as host-supplied
- **THEN** the entry for `contexts` SHALL name `ToolContext` and `ChatToolContext`, in the order
  the host listed them
- **AND** a host that lists `ToolContext` twice SHALL be answered as one that lists it once

#### Scenario: A type derived from a listed type holds none of it
- **WHEN** a function declares `patch:ChatToolContext.Update` and the host lists `ToolContext` as
  host-supplied
- **THEN** the parameter SHALL NOT be marked host-supplied, SHALL stay in the input schema and
  SHALL name no listed type its type holds
- **AND** a parameter declared `Request.Update`, where `Request` declares a field of
  `ChatToolContext`, SHALL name `ToolContext` as a listed type its type holds

#### Scenario: A parameter that holds no listed type says nothing
- **WHEN** a function declares `orderId:string` and `plan:Plan`, where `Plan` holds no record that
  extends `ToolContext`, and the host lists `ToolContext` as host-supplied
- **THEN** neither entry SHALL name a listed type its type holds

#### Scenario: A record that holds itself ends
- **WHEN** a program declares `type Node = { next?:Node context?:ChatToolContext }` and a function
  declares `node:Node`
- **THEN** the answer SHALL be given, and the entry for `node` SHALL name `ToolContext` once
