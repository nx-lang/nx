# declaration-schema-export Specification

## Purpose

Let a host get, from a compiled NX program, JSON Schema for the arguments and result of a function
and for a declared type, with the author's `///` documentation as descriptions, so the host can
describe NX functions and values to systems that speak JSON Schema, such as a language model's tool
interface, without reading compiler or runtime internals.

## Requirements

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

### Requirement: A built program exports the schema of a declared type
A built program SHALL answer, for a type named by module identity and declared name, with that
type's schema as a JSON Schema draft 2020-12 document and with the declaration's documentation. The
type MAY be a record, an abstract record, an action, a union, a type alias, a derived update record
named `<Target>.Update`, or a derived property union named `<Target>.Property`. The caller SHALL be
able to ask for the schema in the input direction or the output direction, which
"Schemas are written for a direction" defines, and the output direction SHALL be the default.

A reference that names no type in the program SHALL fail with a `schema-unknown-declaration`
diagnostic. A reference that names a component SHALL be answered as a type with no JSON form.

#### Scenario: A record type is described
- **WHEN** a host asks for the schema of `Plan` declared `type Plan = { name:string seats:int }`
- **THEN** the document SHALL describe an object with the properties `$type`, `name` and `seats`

#### Scenario: A type alias is described by its target
- **WHEN** a host asks for the schema of `Names` declared `type Names = string+`
- **THEN** the document SHALL describe a non-empty array of strings

#### Scenario: A union is described
- **WHEN** a host asks for the schema of `Status` declared `type Status = active | paused`
- **THEN** the document SHALL describe a string that is one of `active` and `paused`

### Requirement: Schema documents are self-contained and deterministic
Every schema document SHALL carry `$schema` with the value
`https://json-schema.org/draft/2020-12/schema` at its root and SHALL be complete on its own: every
reference in it SHALL be a `$ref` to an entry of the document's own root `$defs`. Each record,
union, payload union case and applied generic record a document mentions SHALL have one entry in
`$defs`, however many times it is mentioned, and every mention SHALL be a `$ref` to it, so that a
type that refers to itself is described without unbounded expansion. A document that mentions none
SHALL have no `$defs`.

A `$defs` key SHALL contain only letters, digits, `_`, `.` and `-`. The key of a record or union
SHALL be its declared name, and the key of a payload case SHALL be `<Union>.<case>`. The key of an
applied generic record SHALL be the record's name followed, for each type argument in the record's
declaration order, by `_` and the argument's own key or primitive name. Where two different
declarations would take the same key, the one mentioned later in the document SHALL take the key
with `_2` appended, then `_3`, and so on.

The same program, declaration and options SHALL produce the same document, byte for byte, in every
SDK that provides the export: properties in declaration order with inherited fields first, and
`$defs` entries in the order the document first mentions them.

#### Scenario: A recursive type is described once
- **WHEN** a host asks for the schema of `type Folder = { name:string children?:Folder+ }`
- **THEN** the document SHALL hold one `$defs` entry `Folder`
- **AND** the schema of `children` SHALL be an array whose `items` is `{ "$ref": "#/$defs/Folder" }`

#### Scenario: Two declarations of one name get distinct keys
- **WHEN** a function takes `a:Card` from module `left.nx` and `b:Card` from module `right.nx`
  through aliased imports
- **THEN** the input schema's `$defs` SHALL hold `Card` for the first and `Card_2` for the second

#### Scenario: A function over primitives has no definitions
- **WHEN** a host asks for the schema of `let add(a:int, b:int): int = { a + b }`
- **THEN** neither the input schema nor the output schema SHALL have a `$defs` keyword

### Requirement: Primitive types map to JSON Schema types
The export SHALL map `string` to `{ "type": "string" }`, `boolean` to `{ "type": "boolean" }`,
`int` and `int64` to `{ "type": "integer" }`, `int32` to
`{ "type": "integer", "minimum": -2147483648, "maximum": 2147483647 }`, and `float32` and `float64`
to `{ "type": "number" }`. It SHALL map `object` to `{ "not": { "type": "null" } }`, which admits
any JSON value but `null`: a runtime takes an array or an object at an `object` site whole, and
reads `null` as no value, which a site of exactly one value refuses.

#### Scenario: Numeric primitives
- **WHEN** a host asks for the schema of `type Sizes = { n:int a:int32 b:int64 c:float32 d:float64 }`
- **THEN** `n` and `b` SHALL be `{ "type": "integer" }`
- **AND** `a` SHALL be an integer schema bounded by -2147483648 and 2147483647
- **AND** `c` and `d` SHALL be `{ "type": "number" }`

#### Scenario: The top type admits any value but null
- **WHEN** a host asks for the schema of `type Bag = { value:object }`
- **THEN** the schema of `value` SHALL be `{ "not": { "type": "null" } }`
- **AND** `{ "value": null }` SHALL NOT be valid against the schema

### Requirement: Occurrences map to arrays and to absence
A `+` occurrence of an item type SHALL map to an array schema whose `items` is the item type's
schema, with `minItems` 1. A `*` occurrence SHALL map to the same array schema without `minItems`.

A `?` occurrence SHALL map by where it stands, as the canonical encoding does. A field or parameter
that carries the `?` mark SHALL have the schema of its declared type and SHALL be left out of
`required`; its schema SHALL NOT admit `null`. A function result whose type is a standalone `T?`
SHALL map to `{ "anyOf": [<schema of T>, { "type": "null" }] }` in the output schema, because an
entry call returns an empty optional result as `null`.

A schema SHALL describe only the canonical encoding. It SHALL NOT admit the other spellings a
runtime decodes at the boundary: a single value where an array is expected, `null` or an empty
array for an absent optional, or a one-element array at a `?` site.

#### Scenario: A required sequence has at least one item
- **WHEN** a host asks for the schema of `type Box = { items:string+ }`
- **THEN** the schema of `items` SHALL be `{ "type": "array", "items": { "type": "string" }, "minItems": 1 }`
- **AND** `items` SHALL be in `required`

#### Scenario: An optional sequence is absent or non-empty
- **WHEN** a host asks for the schema of `type Box = { tags?:string+ }`
- **THEN** the schema of `tags` SHALL be the same non-empty array schema
- **AND** `tags` SHALL NOT be in `required`

#### Scenario: A sequence result may be empty
- **WHEN** a host asks for the schema of a function declared `: Plan*`
- **THEN** the output schema SHALL be an array schema with no `minItems`

#### Scenario: An optional result may be null
- **WHEN** a host asks for the schema of `let find(id:string): Plan? = { ... }`
- **THEN** the output schema SHALL be `anyOf` the schema of `Plan` and `{ "type": "null" }`

#### Scenario: An optional parameter does not admit null
- **WHEN** a host asks for the schema of `let find(id:string, region?:string): Plan? = { ... }`
- **THEN** the schema of `region` in the input schema SHALL be `{ "type": "string" }`
- **AND** `region` SHALL NOT be in `required`

### Requirement: Records map to closed object schemas
A concrete record, an action included, SHALL map to an object schema with one property per effective
field, inherited fields first, each keyed by the field's name exactly as declared, and with
`additionalProperties` set to `false`. A content field SHALL be an ordinary property under its name.
A field SHALL be listed in `required` when it has neither the `?` mark nor a default. A field with
a default SHALL also be listed in `required` in the output direction, because a runtime always
writes it.

A field's default SHALL be written as the `default` keyword when the default is a string, numeric
or boolean literal, or names a constant case of the field's union type, in which case it is the
case's bare name. Any other default SHALL leave the field without a `default` keyword and SHALL NOT
be reported. The same rules for `required` and `default` SHALL apply to a function's parameters in
its input schema.

The object schema's `$type` property SHALL be `{ "const": "<record name>" }`, written as "Schemas
are written for a direction" requires.

#### Scenario: Required, optional and defaulted fields
- **WHEN** a host asks for the input-direction schema of
  `type Booking = { host:string notes?:string durationMinutes:int = 30 }`
- **THEN** `required` SHALL be `["host"]`
- **AND** the schema of `durationMinutes` SHALL be `{ "type": "integer", "default": 30 }`
- **AND** `additionalProperties` SHALL be `false`

#### Scenario: A defaulted field is always present in output
- **WHEN** a host asks for the output-direction schema of that `Booking`
- **THEN** `required` SHALL be `["$type", "host", "durationMinutes"]`

#### Scenario: Inherited fields come first
- **WHEN** a host asks for the schema of `User` declared
  `abstract type Entity = { id:int } type User extends Entity = { name:string }`
- **THEN** the properties after `$type` SHALL be `id` then `name`

#### Scenario: A computed default is not written
- **WHEN** a field is declared `label:string = { prefix + name }`
- **THEN** the field's schema SHALL have no `default` keyword
- **AND** the field SHALL NOT be in `required` in the input direction

#### Scenario: A constant case default is its bare name
- **WHEN** a field is declared `status:Status = Status.active` and `Status` is `active | paused`
- **THEN** the field's schema SHALL carry `"default": "active"`

### Requirement: Schemas are written for a direction
A schema SHALL be written for one of two directions. The input direction describes a value a host
supplies to a runtime: a function's arguments. The output direction describes a value a runtime
returns to a host: a function's result. A function's input schema SHALL be in the input direction
and its output schema in the output direction.

In the output direction every record and every payload union case SHALL have the `$type` property
and SHALL list it in `required`.

In the input direction a record or payload case SHALL have the `$type` property, listed in
`required`, only where it is mentioned at a site that admits more than one shape: a site typed by
an abstract record, or by a union with a payload case. A concrete record at a site typed by that
record SHALL have no `$type` property, because a runtime takes the field list from the declared
type there. Where one record is mentioned in a document both at a site that needs the discriminator
and at one that does not, it SHALL have one `$defs` entry, with `$type` required.

#### Scenario: An argument record needs no discriminator
- **WHEN** a host asks for the schema of `let book(request:Booking): string = { ... }` where
  `Booking` is a concrete record
- **THEN** the `Booking` entry of the input schema's `$defs` SHALL have no `$type` property

#### Scenario: A result record carries its discriminator
- **WHEN** a host asks for the schema of `let next(): Booking = { ... }`
- **THEN** the `Booking` entry of the output schema's `$defs` SHALL have
  `"$type": { "const": "Booking" }` and SHALL list `$type` first in `required`

#### Scenario: A polymorphic argument carries its discriminator
- **WHEN** a host asks for the schema of `let describe(shape:Shape): string = { ... }` where
  `Shape` is abstract and `Circle` and `Square` extend it
- **THEN** the `Circle` and `Square` entries of the input schema's `$defs` SHALL each require
  `$type`

### Requirement: Unions map by the constant-ness of their cases
A union whose cases are all constant SHALL map to `{ "type": "string", "enum": [...] }` listing the
cases' authored names in declaration order.

A payload case SHALL map to an object schema built as a record's is, from the case's fields and the
fields it inherits from the union's base, whose `$type` is `{ "const": "<Union>.<case>" }` and is
always required, in both directions. A case that declares no fields in a union that extends an
abstract base is a payload case.

A union with a payload case SHALL map to `{ "anyOf": [...] }` holding, when it has constant cases,
one string `enum` schema listing them, followed by a `$ref` to each payload case in declaration
order.

A derived property union, `<Target>.Property`, SHALL map as the constant union it is.

#### Scenario: A constant union is a string enum
- **WHEN** a host asks for the schema of `type HttpMethod = get | post | put | patch | delete`
- **THEN** its `$defs` entry SHALL be
  `{ "type": "string", "enum": ["get", "post", "put", "patch", "delete"] }`

#### Scenario: A payload case carries a scoped discriminator
- **WHEN** a host asks for the schema of `type LoadState = | failed { message:string }`
- **THEN** the `LoadState.failed` entry SHALL require `$type` equal to `LoadState.failed` and
  `message`

#### Scenario: A mixed union lists constant cases and payload cases
- **WHEN** a host asks for the schema of `type Shape = circle | square { n:int }`
- **THEN** the `Shape` entry SHALL be `anyOf` `{ "type": "string", "enum": ["circle"] }` and a
  `$ref` to `Shape.square`

#### Scenario: A fieldless case of a union with a base is an object
- **WHEN** a host asks for the schema of `UiEvent` declared
  `abstract type EventBase = { source:string = "ui" } type UiEvent extends EventBase = | closed`
- **THEN** the `UiEvent.closed` entry SHALL be an object schema with `$type` equal to
  `UiEvent.closed` and the property `source`
- **AND** the `UiEvent` entry SHALL NOT list `closed` in a string `enum`

### Requirement: An abstract record maps to its concrete descendants
A site typed by an abstract record SHALL map to `{ "anyOf": [...] }` holding a `$ref` to every
shape a value there may take: each concrete record that extends the abstract record, directly or
through further abstract records, and each case of each union that extends it. The shapes SHALL be
those declared in any module of the program, in a deterministic order: by module in the program's
module order, then by declaration order within a module. Where a function of the program takes or
returns such a site, the entry module's NX IR artifact lists every module that declares one of the
shapes (`nx-ir-format`), so a runtime that links the program from its entry accepts each shape the
schema lists.

An abstract record that no shape in the program extends SHALL be reported as a type with no JSON
form. Where two of the shapes carry the same `$type`, which two modules that each declare a record
of one name can produce, the export SHALL report a `schema-ambiguous-discriminator` diagnostic
naming the discriminator and the abstract record, and SHALL NOT emit a schema, because a runtime
refuses such a value as ambiguous.

#### Scenario: Descendants across modules are listed
- **WHEN** a library declares `abstract type Source = { name?:string }` and
  `type WebSource extends Source = { allowedDomains?:string+ }`
- **AND** the entry module declares `type TableSource extends Source = { table:string }` and a
  function `let pick(): Source = { ... }`
- **AND** a host asks for the schema of `pick`
- **THEN** the output schema SHALL be `anyOf` references to `WebSource` and `TableSource`

#### Scenario: A union extending the abstract record contributes its cases
- **WHEN** `abstract type EventBase = { source:string }` is extended by
  `type UiEvent extends EventBase = | clicked { x:int } | closed`
- **AND** a host asks for the schema of `EventBase`
- **THEN** the document SHALL be `anyOf` references to `UiEvent.clicked` and `UiEvent.closed`

#### Scenario: An abstract record with no descendant has no JSON form
- **WHEN** a host asks for the schema of `abstract type Lonely = { id:int }` in a program that
  declares nothing extending it
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming `Lonely`
- **AND** it SHALL carry no schema

#### Scenario: Two descendants with one discriminator are refused
- **WHEN** modules `a.nx` and `b.nx` each declare a record `Card` extending the same abstract
  `Item`, and a function takes `item:Item`
- **THEN** the function's schema SHALL carry a `schema-ambiguous-discriminator` diagnostic naming
  `Card` and `Item`
- **AND** it SHALL carry no input schema

### Requirement: Generic records map per instantiation
An applied type SHALL map to a `$defs` entry for that instantiation, built as a record's is with
each type parameter replaced by its type argument. Two applied types of one record with different
arguments SHALL be two entries. A field typed by a type parameter of a generic record whose schema
is asked for by name, with no arguments, SHALL map as `object` does, as a runtime treats such a
field as `object`. An applied type whose type arguments apply the same record twice or more, or nest
applied types more than four deep, SHALL map as that record named with no arguments, so a record
that grows its own argument at every step, directly or through another generic record, is still
described by a finite document.

#### Scenario: An applied type substitutes its argument
- **WHEN** a host asks for the schema of `let first(): <Page T=Plan /> = { ... }` where
  `type Page = { T:type items:T+ total:int }`
- **THEN** the output schema SHALL reference a `$defs` entry `Page_Plan` whose `items` is an array
  of references to `Plan`
- **AND** that entry SHALL have no property for `T`

#### Scenario: Two instantiations are two entries
- **WHEN** a function takes `a:<Page T=Plan />` and `b:<Page T=string />`
- **THEN** the input schema's `$defs` SHALL hold `Page_Plan` and `Page_string`

#### Scenario: A record applied to itself without end stays finite
- **WHEN** a host asks for the schema of `let f(b:<Box T=int />): int` where
  `type Box = { T:type v:T inner?:<Box T=<Box T=T /> /> }`
- **THEN** the answer SHALL hold an input schema with finitely many `$defs` entries
- **AND** the entry for `<Box T=<Box T=int />/>` SHALL refer its `inner` to the entry `Box`, the
  record named with no arguments

#### Scenario: An unapplied generic record leaves its parameter open
- **WHEN** a host asks for the schema of the type `Page` by name
- **THEN** the schema of `items` SHALL be an array whose `items` is `{ "not": { "type": "null" } }`

### Requirement: Type aliases and update records map to what they denote
A reference to a type alias SHALL map to the schema of the alias's target, with no `$defs` entry
for the alias itself.

A derived update record, `<Target>.Update`, SHALL map to an object schema with `$type` equal to
`<Target>.Update`, written as the direction requires, one property per field of the target, none of
them in `required` other than `$type`, and `additionalProperties` set to `false`. The schema of a
property whose target field carries the `?` mark SHALL also admit `null`, which is how the canonical
encoding spells a cleared field. No property of an update record SHALL carry a `default`.

#### Scenario: An alias is transparent
- **WHEN** a function takes `names:Names` where `type Names = string+`
- **THEN** the schema of `names` SHALL be a non-empty array of strings
- **AND** the input schema SHALL have no `$defs` entry `Names`

#### Scenario: An update record makes every field optional and a clearable field nullable
- **WHEN** a host asks for the output-direction schema of `User.Update` where
  `type User = { name:string email?:string }`
- **THEN** `required` SHALL be `["$type"]`
- **AND** the schema of `name` SHALL be `{ "type": "string" }`
- **AND** the schema of `email` SHALL be `anyOf` `{ "type": "string" }` and `{ "type": "null" }`

### Requirement: Documentation becomes descriptions
The export SHALL carry `///` documentation as follows, in each case as the documentation's Markdown
text with every doc link replaced by its label in a code span and nothing else changed:

- a function's documentation as the `description` of its schema answer, with its summary beside it
- a parameter's documentation as the `description` of its property in the input schema, and on its
  parameter entry
- a record's, union's or payload case's documentation as the `description` of its `$defs` entry
- a field's documentation as the `description` of its property, including a field of a payload case
  and an inherited field, which carries the documentation written where it is declared
- a type's documentation, a type alias's included, as the `description` of its type schema answer

A property whose schema is a `$ref` SHALL carry its `description` beside the `$ref`. An item with no
documentation SHALL have no `description` keyword; the export SHALL NOT invent one.

A constant case has no schema of its own. When at least one constant case of a union is documented,
the `description` of the union's `$defs` entry SHALL be the union's documentation, if any, then a
blank line, then one line per documented constant case in declaration order of the form
`` - `<case>`: <the case's summary> ``.

Adding, removing or editing a doc comment SHALL change only `description` values and the
documentation fields of the answer, never the structure of a schema.

#### Scenario: A documented function and its parameters
- **WHEN** a host asks for the schema of a function documented `Finds the plans that fit a team.`
  whose parameter `teamSize` is documented `Number of people who need a seat.`
- **THEN** the answer's description SHALL be `Finds the plans that fit a team.`
- **AND** the `teamSize` property of the input schema SHALL carry
  `"description": "Number of people who need a seat."`
- **AND** the `teamSize` parameter entry SHALL carry the same text

#### Scenario: A documented field beside a reference
- **WHEN** a record field `plan:Plan` is documented `The plan the team chose.`
- **THEN** the property SHALL be
  `{ "$ref": "#/$defs/Plan", "description": "The plan the team chose." }`

#### Scenario: A doc link is written as code
- **WHEN** a parameter is documented `Ignored when [maxMonthlyPrice] is set.`
- **THEN** its description SHALL be ``Ignored when `maxMonthlyPrice` is set.``

#### Scenario: Documented constant cases are listed on the union
- **WHEN** `/// How a call is sent.` documents `type HttpMethod = get | post` and the case `post` is
  documented `Creates a resource.`
- **THEN** the `HttpMethod` entry's description SHALL be `How a call is sent.`, a blank line, and
  `` - `post`: Creates a resource. ``

#### Scenario: Undocumented source has no descriptions
- **WHEN** a host asks for the schema of a function with no doc comments over undocumented types
- **THEN** no schema in the answer SHALL have a `description` keyword

#### Scenario: Editing documentation leaves the structure alone
- **WHEN** a doc comment is added to a parameter and the schema is asked for again
- **THEN** the two input schemas SHALL differ only in that property's `description`

### Requirement: A type with no JSON form is reported, never approximated
The export SHALL treat each of the following as a type with no JSON form: a function type; the
function reference type `<function ... />: R`, which accepts a function of any parameters, written
out or through an alias; a component used as a
type, and markup, an element or the built-in `Element`; an abstract record nothing in the program extends; and a result type the
checker could not infer. Where a schema would have to mention such a type, at any depth, the export
SHALL report a diagnostic of code `schema-inexpressible-type` with error severity that names the
type in NX spelling and the parameter, field or result that holds it, labeled at that member's
declaration, and SHALL NOT emit the schema that would have mentioned it. It SHALL NOT substitute an
open schema, drop the member, or describe the wire form of a function value.

A function's answer SHALL report its input schema and its output schema independently: a type with
no JSON form among the parameters SHALL leave the input schema absent, and one in the result SHALL
leave the output schema absent, each with its diagnostic, and the other schema SHALL still be
answered. The diagnostics SHALL be returned as data with the answer, in the shape build diagnostics
take. An answer whose schemas are all present SHALL carry no error diagnostic.

#### Scenario: A function-typed parameter has no input schema
- **WHEN** a host asks for the schema of
  `let render(template:<function Item:string />: string, title:string): string = { ... }`
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming the parameter
  `template` and its function type
- **AND** it SHALL carry no input schema
- **AND** its output schema SHALL be `{ "type": "string" }` beside the `$schema` keyword

#### Scenario: A function-typed field deep in a result is found
- **WHEN** a function returns `Catalog`, whose field `rows` is a sequence of `Row`, whose field
  `template` has a function type
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming `Row.template`
- **AND** it SHALL carry no output schema

#### Scenario: A function reference field is not described as a record
- **WHEN** a record is declared `type FunctionTool = { function: <function ... />: object* }`
- **AND** a host asks for that record's schema
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming
  `FunctionTool.function` and the type `<function ... />: object*`
- **AND** it SHALL NOT answer with an object schema of `$type`, `module` and `name`

#### Scenario: A function reference field with a stated result is reported the same way
- **WHEN** a record is declared `type Args = { q:string } type HttpTool = { arguments: <function ... />: Args }`
- **AND** a host asks for the schema of `HttpTool`
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming `HttpTool.arguments` and the type `<function ... />: Args`
- **AND** it SHALL NOT describe `Args` in its place

#### Scenario: A function reference field is told apart from an object field
- **WHEN** a host asks for the schema of `type Pair = { run: <function ... />: object* data:object }`
- **THEN** the diagnostic SHALL name `Pair.run` only
- **AND** a record declared with `data:object` alone SHALL be answered with `data` as
  `{ "not": { "type": "null" } }`

#### Scenario: A site typed by the agent library's `Tool` has no JSON form
- **WHEN** a program imports `@nx/agent` and declares `let pick(): Tool = { ... }`
- **AND** a host asks for the schema of `pick`
- **THEN** the answer SHALL carry `schema-inexpressible-type` diagnostics naming
  `FunctionTool.function` and `HttpTool.arguments`
- **AND** it SHALL carry no output schema

#### Scenario: A component used as a type has no JSON form
- **WHEN** a host asks for the schema of a function whose parameter is typed by a component
- **THEN** the answer SHALL carry a `schema-inexpressible-type` diagnostic naming that parameter

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

### Requirement: Schemas agree with the runtimes' boundary
The schemas SHALL describe the canonical value encoding that `runtime-output-format`,
`occurrence-types` and `update-records` define, which every NX runtime reads and writes. For every
function whose input schema the export answers, every JSON object that is valid against that schema
SHALL be accepted as the function's arguments, keyed by parameter name, by a runtime running the IR
emitted from the same artifact and linked from its entry module, with no boundary diagnostic, and SHALL mean what the schema says: a
property left out is the parameter left out. For every function whose output schema the export
answers, every value an entry call of that function returns SHALL be valid against that schema.

A schema MAY be narrower than what a runtime accepts, since it describes only the canonical
encoding, and SHALL NOT be wider. The repository's tests SHALL check both properties against the
TypeScript IR runtime, the runtime the export's consumers run, over a corpus that covers every
mapping this capability defines.

#### Scenario: Schema-valid arguments are accepted
- **WHEN** the tests generate arguments valid against a corpus function's input schema, including
  ones that omit every optional property
- **AND** call the function by name through `@nx-lang/ir-runtime` on the image emitted from the same
  artifact
- **THEN** no call SHALL fail with an `nx-ir-boundary-type`, `nx-ir-boundary-field` or
  `nx-ir-arguments` diagnostic

#### Scenario: Results validate against the output schema
- **WHEN** the tests call each corpus function and validate what it returns against its output
  schema with a JSON Schema draft 2020-12 validator
- **THEN** every result SHALL be valid, including a `null` result of a standalone `T?`, an empty
  array of a `T*`, a record that omits an empty optional field, and a constant case returned as a
  bare string

#### Scenario: A discriminator the schema omits is not needed
- **WHEN** an argument record valid against an input schema carries no `$type`
- **THEN** the runtime SHALL accept it and normalize it to the declared record
