# sequence-model Specification

## Purpose

Defines the flat sequence model that every NX sequence type, collecting position, runtime and code
generation target follows: an item is a sequence of one, a sequence never contains a sequence, and a
sequence placed where items are collected contributes its items.

## Requirements

### Requirement: A sequence never contains a sequence
The sequence type `T[]` SHALL be defined only for an item type `T`. A type that is itself a sequence
SHALL NOT be an item type, so no type reference, alias chain, type-argument substitution or
inference result SHALL produce a sequence whose element type is a sequence. A nullable type, a
record, a union, a function type, an applied type, `object` and every primitive SHALL be item
types. Where a spelled type would nest a sequence, the system SHALL reject it; where an inferred
type would nest one, the system SHALL flatten it under the rules below rather than nest it.

At runtime a sequence value SHALL NOT hold a sequence value as an element. A value of static type
`object` MAY hold a sequence, and such a value is opaque: it is an item of type `object` where it
is typed as one, and it contributes its own items where it is collected, as every sequence value
does.

#### Scenario: A spelled nested sequence is rejected
- **WHEN** a file contains `type Grid = int[][]`
- **THEN** the system SHALL reject the declaration
- **AND** the diagnostic SHALL explain that a sequence cannot contain sequences

#### Scenario: A nested sequence reached through an alias is rejected
- **WHEN** a file contains `type Ints = int[]` and `type Grid = Ints[]`
- **THEN** the system SHALL reject `Grid` on the same terms, naming `Ints` as already a sequence

#### Scenario: A nullable sequence and a sequence of nullables remain item-typed
- **WHEN** a file contains `type A = { names:string[]? aliases:string?[] }`
- **THEN** the system SHALL accept both fields, since `string` and `string?` are item types

#### Scenario: A record is how data nests
- **WHEN** a file contains `type Row = { cells:int[] }` and `let grid:Row[] = { <Row cells={1 2}/> <Row cells={3 4}/> }`
- **THEN** the system SHALL accept the binding
- **AND** evaluation SHALL produce a sequence of two records, each holding a sequence of two integers

### Requirement: A collecting position splices a sequence-valued item
Every position that collects items into a sequence — a braced value, a call argument written as a
braced value, element body content, and the yields of a `for` — SHALL treat an item whose static
type is a sequence as contributing its items in order, and an item of any other static type as
contributing itself. The item type of the collected sequence SHALL be the join of each item's
contribution: the element type of a sequence-typed item, and the type itself otherwise. An item of
type `object` SHALL contribute `object`.

How many items are written alongside it SHALL make no difference to how one item is treated. A
braced value holding a single item SHALL apply the same rule to that item as a braced value holding
several.

The interpreter, the TypeScript IR runtime and every code generation target SHALL produce the same
value for the same source. A sequence value in a collecting position SHALL contribute its elements
in every runtime, so no runtime SHALL produce a sequence holding a sequence.

This rule SHALL be the whole of it. An `if` or `if … is` with no `else` needs no rule of its own
here: its value is a sequence, empty when no branch was taken, so it splices like any other
sequence-valued item. No implementation SHALL carry a notion of what an item contributes that is
separate from the item's own type and value.

#### Scenario: Two sequences in a braced value concatenate
- **WHEN** a file contains `let xs:string[] = {"a" "b"}`, `let ys:string[] = {"c"}` and `let all = {xs ys}`
- **THEN** `all` SHALL infer as `string[]`
- **AND** evaluation SHALL produce the sequence `"a" "b" "c"`

#### Scenario: A sequence and an item in a braced value share an item type
- **WHEN** a file contains `let xs:string[] = {"a" "b"}` and `let all = {xs "c"}`
- **THEN** `all` SHALL infer as `string[]` rather than `object[]`
- **AND** evaluation SHALL produce three strings, none of them a nested sequence

#### Scenario: A `for` concatenates what its body yields
- **WHEN** a file contains `type Row = { cells:int[] }` and `let flat(rows:Row[]): int[] = {for r in rows { r.cells }}`
- **THEN** type checking SHALL accept the return type, because the body's item type is `int`
- **AND** evaluation over two rows holding `1 2` and `3 4` SHALL produce the sequence `1 2 3 4`

#### Scenario: A `for` whose body yields nothing yields the empty sequence
- **WHEN** a file contains `let ys:string[] = {"q"}` and `let xs:string[] = {for y in ys {}}`
- **THEN** type checking SHALL accept the binding
- **AND** evaluation SHALL bind an empty `string[]`

#### Scenario: A `for` body that yields one item still yields a sequence of items
- **WHEN** a file contains `let squares(ns:int[]): int[] = {for n in ns { n * n }}`
- **THEN** type checking SHALL accept the return type
- **AND** evaluation SHALL produce one integer per input, as it does today

#### Scenario: Body content and braced values follow the same rule
- **WHEN** a file contains `external component <List content items:Badge[] />`, `let some:Badge[] = { <Badge/> <Badge/> }` and both `<List>{some}<Badge/></List>` and `<List items={some <Badge/>} />`
- **THEN** type checking SHALL accept both
- **AND** evaluation SHALL bind three badges to `items` in each case

#### Scenario: Every backend splices body content
- **WHEN** a component body binds `{some <Badge/>}` as content, with `some` a two-element sequence
- **THEN** the interpreter, the TypeScript IR runtime and generated TypeScript SHALL each produce a
  content sequence of three badges
- **AND** none SHALL produce a sequence whose first element is a sequence

### Requirement: An item is a sequence of one, and only one level deep
Where a sequence type is expected and an item of its element type is supplied, the system SHALL
treat the item as a one-element sequence, as it does today. That lift SHALL apply once: because no
sequence type has a sequence element type, no site SHALL lift a value twice, and a runtime SHALL
NOT wrap a value in more than one sequence when coercing it to a declared type. A nullable item
SHALL NOT lift to a nullable sequence: a `string?` value at a `string[]?` site SHALL be rejected,
since `null` is not a `string` and the lift applies to the element type alone.

#### Scenario: A scalar lifts once at a sequence site
- **WHEN** a file contains `type Box = { items:int[] }` and `let b = <Box items={1} />`
- **THEN** evaluation SHALL bind a one-element `int[]`, as today

#### Scenario: A nullable item does not lift to a nullable sequence
- **WHEN** a file contains `type Box = { names:string[]? }`, `let maybe:string? = null` and `let b = <Box names={maybe} />`
- **THEN** type checking SHALL reject the binding, naming `string?` and `string[]?`

#### Scenario: A type argument is an item type
- **WHEN** a file contains `type Box = { T:type value:T }` and `type Ints = int[]` and `let b:<Box T=Ints/> = <Box T=Ints value={1 2} />`
- **THEN** the system SHALL reject both the annotation and the construction, explaining that a type
  argument must not be a sequence
- **AND** `let n:<Box T=int?/> = <Box T=int? value={null} />` SHALL be accepted, since `int?` is an
  item type
