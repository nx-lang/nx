# sequence-model Specification

## Purpose

Defines the flat sequence model that every NX sequence type, collecting position, runtime and code
generation target follows: an item is a sequence of one, a sequence never contains a sequence, and a
sequence placed where items are collected contributes its items.

## Requirements

### Requirement: A sequence never contains a sequence
A sequence type — `T+` or `T*` — SHALL be defined only for an exactly-one item type `T`, and so
SHALL an optional type `T?`. A type that itself carries an occurrence SHALL NOT be an item type, so
no type reference, alias chain, type-argument substitution or inference result SHALL produce a
type whose item type carries an occurrence. A record, a union, a function type, an applied type,
`object` and every primitive SHALL be item types. Where a spelled type would put an occurrence
inside another, the system SHALL reject it; where an inferred type would, the system SHALL flatten
it under the rules below rather than nest it.

At runtime a sequence value SHALL NOT hold a sequence value as an element. A value of static type
`object` MAY hold a sequence, and such a value is opaque: it is an item of type `object` where it
is typed as one, and it contributes its own items where it is collected, as every sequence value
does. Because the empty sequence is also the absent value, an empty sequence held by a value of
static type `object?` SHALL read as absent; `object?` is the one type at which a held sequence and
absence meet, and the system SHALL document rather than distinguish them.

#### Scenario: A spelled nested sequence is rejected
- **WHEN** a file contains `type Grid = int*+`, `type Maybe = int?*` and `type Paren = (int+)?`
- **THEN** the system SHALL reject each declaration on its second suffix
- **AND** the diagnostic SHALL explain that the type already carries an occurrence

#### Scenario: A nested sequence reached through an alias is rejected
- **WHEN** a file contains `type Ints = int+` and `type Grid = Ints*`, `type MaybeInts = Ints?`
- **THEN** the system SHALL reject `Grid` and `MaybeInts` on the same terms, naming `Ints` as already carrying an occurrence

#### Scenario: A record is how data nests
- **WHEN** a file contains `type Row = { cells:int+ }` and `let grid:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }`
- **THEN** the system SHALL accept the binding
- **AND** evaluation SHALL produce a sequence of two records, each holding a sequence of two integers

#### Scenario: A nullable sequence and a sequence of nullables remain item-typed
- **WHEN** a file contains `type Box = { T:type value:T }` and `let b = <Box T=int? value=1 />`
- **THEN** the system SHALL reject the type argument, explaining that a type argument must be exactly one value
- **AND** `type Box2 = { T:type value?:T }` with `<Box2 T=int />` SHALL be accepted

### Requirement: A collecting position splices a sequence-valued item
Every position that collects items into a sequence — a braced value, a call argument written as a
braced value, element body content, and the yields of a `for` — SHALL treat an item whose static
type carries an occurrence as contributing its items in order, and an item of exactly-one type as
contributing itself. The item type of the collected sequence SHALL be the join of each item's item
type, and its occurrence SHALL be the sum of each item's occurrence as `occurrence-types` defines
it. An item of type `object` SHALL contribute `object`.

How many items are written alongside it SHALL make no difference to how one item is treated. A
braced value holding a single item SHALL apply the same rule to that item as a braced value holding
several.

The interpreter, the TypeScript IR runtime and every code generation target SHALL produce the same
value for the same source. A sequence value in a collecting position SHALL contribute its elements
in every runtime, so no runtime SHALL produce a sequence holding a sequence.

This rule SHALL be the whole of it. An `if` or `if … is` with no `else` needs no rule of its own
here: its value is optional, empty when no branch was taken, so it splices like any other item that
admits zero. No implementation SHALL carry a notion of what an item contributes that is separate
from the item's own type and value.

#### Scenario: Two sequences in a braced value concatenate
- **WHEN** a file contains `let xs:string+ = {"a" "b"}`, `let ys:string+ = {"c"}` and `let all = {xs ys}`
- **THEN** `all` SHALL infer as `string+`
- **AND** evaluation SHALL produce the sequence `"a" "b" "c"`

#### Scenario: A sequence and an item in a braced value share an item type
- **WHEN** a file contains `let xs:string* = {"a" "b"}` and `let all = {xs "c"}`
- **THEN** `all` SHALL infer as `string+` rather than `object+`
- **AND** evaluation SHALL produce three strings, none of them a nested sequence

#### Scenario: A `for` concatenates what its body yields
- **WHEN** a file contains `type Row = { cells:int+ }` and `let flat(rows:Row+): int+ = {for r in rows { r.cells }}`
- **THEN** type checking SHALL accept the return type, because the body's item type is `int` and both occurrences are at least one
- **AND** evaluation over two rows holding `1 2` and `3 4` SHALL produce the sequence `1 2 3 4`

#### Scenario: A `for` whose body yields nothing yields the empty sequence
- **WHEN** a file contains `let ys:string+ = {"q"}` and `let xs:string* = {for y in ys {}}`
- **THEN** type checking SHALL accept the binding
- **AND** evaluation SHALL bind the empty value

#### Scenario: A `for` body that yields one item still yields a sequence of items
- **WHEN** a file contains `let squares(ns:int+): int+ = {for n in ns { n * n }}`
- **THEN** type checking SHALL accept the return type
- **AND** evaluation SHALL produce one integer per input

#### Scenario: Body content and braced values follow the same rule
- **WHEN** a file contains `external component <List content items:Badge+ />`, `let some:Badge+ = { <Badge/> <Badge/> }` and both `<List>{some}<Badge/></List>` and `<List items={some <Badge/>} />`
- **THEN** type checking SHALL accept both
- **AND** evaluation SHALL bind three badges to `items` in each case

#### Scenario: Every backend splices body content
- **WHEN** a component body binds `{some <Badge/>}` as content, with `some` a two-element sequence
- **THEN** the interpreter, the TypeScript IR runtime and generated TypeScript SHALL each produce a
  content sequence of three badges
- **AND** none SHALL produce a sequence whose first element is a sequence

### Requirement: An item is a sequence of one, and only one level deep
Where a type carrying an occurrence is expected and an exactly-one value of its item type is
supplied, the system SHALL treat the value as a sequence of one. That lift SHALL apply once: because
no type with an occurrence has an item type with an occurrence, no site SHALL lift a value twice,
and a runtime SHALL NOT wrap a value in more than one sequence when coercing it to a declared type.
The lift SHALL apply to an exactly-one value only: a value whose occurrence admits zero SHALL NOT
satisfy a `+` site, and a value whose occurrence admits many SHALL NOT satisfy a `?` site, by the
lattice in `occurrence-types`.

A join — an `if`, a match or a `??` — whose type admits many SHALL produce a sequence in every
engine, including from a branch or operand that supplies an exactly-one or `?` value: that branch
SHALL evaluate to a sequence of its item, or to the empty value when it holds none. A value whose
static type admits many SHALL therefore be a sequence at runtime, so iterating it, returning it to a
host and the type generated TypeScript declares for it all see a sequence.

#### Scenario: A scalar lifts once at a sequence site
- **WHEN** a file contains `type Box = { items:int+ }` and `let b = <Box items={1} />`
- **THEN** evaluation SHALL bind a one-element sequence

#### Scenario: A join that admits many is a sequence in every branch
- **WHEN** a file contains `let pick(c:boolean, xs:int+) = { if c { xs } else { 5 } }` and `let each(c:boolean, xs:int+) = { for x in pick(c, xs) { x + 1 } }`
- **THEN** `pick(false, {1 2})` SHALL evaluate to a one-item sequence holding `5`, and `each(false, {1 2})` to a one-item sequence holding `6`
- **AND** the interpreter, the TypeScript IR runtime and generated JavaScript SHALL agree

#### Scenario: A nullable item does not lift to a nullable sequence
- **WHEN** a file contains `type Box = { names:string+ }`, `let maybe:string? = {}` and `let b = <Box names={maybe} />`
- **THEN** type checking SHALL reject the binding, naming `string?` and `string+`

#### Scenario: A type argument is an item type
- **WHEN** a file contains `type Box = { T:type value:T }` and `type Ints = int+` and `let b:<Box T=Ints/> = <Box T=Ints value={1 2} />`
- **THEN** the system SHALL reject both the annotation and the construction, explaining that a type
  argument must be exactly one value
- **AND** `let n:<Box T=int/> = <Box T=int value=1 />` SHALL be accepted
