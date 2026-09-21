# conditional-result-types Specification

## Purpose

Defines what an `if` or `if … is` with no `else` produces: the branch it takes, or the empty
sequence. One rule, so that nothing has to distinguish a conditional's type from what it
contributes, and no diagnostic ever names an internal unit type.

## Requirements

### Requirement: A conditional with no `else` carries an implicit empty sequence
An `if` or `if … is` expression with no `else` branch SHALL behave as though it had an `else`
branch whose body is the empty sequence `{}`. The rule governs the expression itself, not the
position it is written in: the expression has one type and one value wherever it appears.

When no branch is taken, it SHALL evaluate to the empty sequence. It SHALL NOT evaluate to null,
and SHALL NOT evaluate to a value of any other kind.

Its type SHALL be the join of its present branches with the type of `{}`, by the ordinary rules of
the join. Because the join of an item type with a sequence type is a sequence, a conditional with
no `else` whose branches produce `T` SHALL have type `T[]`, and one whose branches already produce
a sequence SHALL have that sequence type unchanged — a sequence never contains a sequence.

The interpreter, the TypeScript IR runtime and every code generation target SHALL agree on this.

#### Scenario: An untaken conditional is the empty sequence
- **WHEN** a file binds `let xs:int[] = { if c { 1 } }` and `c` evaluates to false
- **THEN** evaluation SHALL produce the empty sequence
- **AND** SHALL NOT produce null

#### Scenario: A taken conditional is its branch
- **WHEN** the same file is evaluated with `c` true
- **THEN** evaluation SHALL produce a one-item sequence holding `1`

#### Scenario: The type is a sequence of what the branch produces
- **WHEN** a file contains `let v = { if c { 1 } }`
- **THEN** the inferred type of `v` SHALL be `int[]`

#### Scenario: A branch that is already a sequence does not nest
- **WHEN** a file contains `let xs:string[] = {"a"}` and `let v = { if c { xs } }`
- **THEN** the inferred type of `v` SHALL be `string[]`
- **AND** SHALL NOT be a sequence of sequences

#### Scenario: A conditional nested in a conditional is still a sequence of items
- **WHEN** a file binds an element body containing a sibling element and
  `{if c { if d { <A/> } }}` at a content property declared `A[]`, with `c` true and `d` false
- **THEN** type checking SHALL accept the binding
- **AND** evaluation SHALL produce the content property holding only the sibling element
- **AND** the produced value SHALL NOT contain a null item

### Requirement: A conditional with no `else` contributes no items in a collecting position
Where an `if` or `if … is` expression with no `else` branch appears in a collecting position as
`sequence-model` defines it — an element body, a content-property binding, a braced value list, or
the body of a `for` — and no branch is taken, it SHALL contribute no items. It SHALL NOT contribute
a null item, and SHALL NOT contribute an item of any other kind.

This SHALL follow from the requirement above together with the splice rule, and SHALL NOT be a
separate rule that an implementation applies to conditionals: the conditional's value is a sequence,
and an item whose value is a sequence contributes its items.

The item type of the surrounding sequence SHALL be determined by the branches that are present. A
conditional with no `else` SHALL NOT widen that type, and SHALL NOT prevent the sequence from
satisfying a declared element type that the present branches satisfy.

#### Scenario: A false conditional child contributes nothing
- **WHEN** a file binds an element body containing a sibling element and `{if c { <A/> }}`, and `c`
  evaluates to false
- **THEN** evaluation SHALL produce the content property holding only the sibling element
- **AND** the produced value SHALL NOT contain a null item

#### Scenario: A true conditional child contributes its items
- **WHEN** the same file is evaluated with `c` true
- **THEN** evaluation SHALL produce the content property holding the sibling element and the
  conditional's element, in source order

#### Scenario: Generated code and the interpreter agree
- **WHEN** a source containing a conditional child with no `else` is evaluated by the interpreter and
  by generated TypeScript, with the condition false
- **THEN** both SHALL produce the same content property value
- **AND** neither SHALL produce a null item

#### Scenario: A conditional child does not widen the declared element type
- **WHEN** a file binds `{<A/>{if c { <A/> }}}` at a content property declared `A[]`
- **THEN** type checking SHALL accept the binding
- **AND** SHALL NOT report that the value does not satisfy `A[]`

#### Scenario: A conditional as a `for` body contributes nothing on the iterations it is not taken
- **WHEN** a file contains `let evens(ns:int[]): int[] = {for n in ns { if (n % 2 == 0) { n } }}`
- **THEN** type checking SHALL accept the return type, the body's contribution being `int`
- **AND** evaluation over `1 2 3 4` SHALL produce `2 4`, with no null and no empty item between them

#### Scenario: An explicit null item is not an untaken conditional
- **WHEN** a file contains `let xs:string?[] = { "a" null }`
- **THEN** evaluation SHALL produce a two-item sequence whose second item is null, so splicing away
  an untaken conditional SHALL NOT also remove a null item that was written

#### Scenario: A conditional that is the whole body produces the empty list when not taken
- **WHEN** a file binds an element body that is exactly `{if c { <A/> }}` at a content property
  declared `A[]` with a non-empty default, and `c` evaluates to false
- **THEN** evaluation SHALL bind the empty list rather than the declared default
- **AND** SHALL NOT bind null

#### Scenario: A one-item braced value is the same expression as a braced value of several
- **WHEN** a file contains `let xs:int[] = { if c { 1 } }` and `let ys:int[] = { if c { 1 } 2 }`
- **THEN** type checking SHALL accept both
- **AND** SHALL apply the same rule to the conditional in each, the number of items written
  alongside it making no difference

### Requirement: The join of an item type and a sequence type is a sequence type
Where the join of two types is taken and one is a sequence type and the other is not, the result
SHALL be a sequence whose element type is the join of the sequence's element type and the other
type. This follows from an item being a sequence of one.

This SHALL apply wherever the join is taken, and SHALL be what gives a conditional with no `else`
its type, the missing branch being `{}`. An arm written as `{}` SHALL therefore take its element
type from the arm it is joined with, rather than the two joining to the top type.

#### Scenario: An empty arm takes its element type from the other arm
- **WHEN** a file binds `{<A/>{if c { <A n=2 /> } else { }}}` at a content property declared `A[]`
- **THEN** type checking SHALL accept the binding
- **AND** SHALL NOT report the type found as `object[]`

#### Scenario: A missing branch joins as an empty arm would
- **WHEN** a file contains `let v = { if c { <A/> } }` and `let w = { if c { <A/> } else { } }`
- **THEN** the inferred types of `v` and `w` SHALL both be `A[]`

### Requirement: A nullable value requires an explicit `else`
A conditional with no `else` SHALL NOT satisfy a nullable declared type by virtue of being able to
take no branch. Where a nullable value is wanted, the `else` branch SHALL be written.

#### Scenario: A conditional with no else is rejected at a nullable annotation
- **WHEN** a file contains a record field declared `v:string?` bound to `{if c { "x" }}`
- **THEN** type checking SHALL reject the binding
- **AND** the message SHALL name `string[]` as the type found

#### Scenario: An explicit null else produces a nullable
- **WHEN** a file contains a record field declared `v:string?` bound to `{if c { "x" } else { null }}`
- **THEN** type checking SHALL accept the binding
- **AND** evaluation with `c` false SHALL produce null for that field

#### Scenario: A conditional with an else is unaffected
- **WHEN** a file contains a record field declared `v:string` bound to
  `{if { c => "x" else => "y" }}`
- **THEN** type checking SHALL accept the binding
- **AND** the expression's type SHALL be `string`, not a sequence

#### Scenario: An exhaustive union match is unaffected
- **WHEN** a file matches a two-case union with both cases covered and no `else` arm, binding the
  result at a non-nullable annotation
- **THEN** type checking SHALL accept the binding
- **AND** the expression's type SHALL NOT be a sequence

#### Scenario: A match with an uncovered path follows the same rule as a missing else
- **WHEN** a file matches a two-case union covering one case, with no `else` arm, and binds the
  result at a `string[]` annotation
- **THEN** type checking SHALL report that the match is not exhaustive
- **AND** SHALL NOT additionally report a type mismatch against `string[]`

### Requirement: No diagnostic names the unit type
No diagnostic message SHALL contain `void` as the rendering of a type inferred by the system. Where a
message would otherwise name the type of a conditional with no `else`, it SHALL name the sequence
type that conditional has.

Where an internal type with no source spelling must still be rendered in a message, its rendering
SHALL NOT be a legal identifier, so that no user declaration can take the same name and produce a
message naming two different types identically.

This governs message text only, and does not govern code generation: a target language's own `void`
in generated output is unaffected.

#### Scenario: A conditional mismatch names the sequence type
- **WHEN** type checking rejects a conditional with no `else` bound at a scalar annotation
- **THEN** the message SHALL name the sequence type
- **AND** SHALL NOT contain `void`

#### Scenario: A user-declared type named `void` is unambiguous in a message
- **WHEN** a file declares `type void = { n:int }` and a diagnostic names that type
- **THEN** the message SHALL name only the user-declared type
- **AND** SHALL NOT be capable of naming a different type by the same rendering
