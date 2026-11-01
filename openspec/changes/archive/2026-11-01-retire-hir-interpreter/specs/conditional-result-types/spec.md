## MODIFIED Requirements

### Requirement: A conditional with no `else` carries an implicit empty sequence
An `if` or `if … is` expression with no `else` branch SHALL behave as though it had an `else`
branch whose body is the empty value `{}`. The rule governs the expression itself, not the
position it is written in: the expression has one type and one value wherever it appears.

When no branch is taken, it SHALL evaluate to the empty value. It SHALL NOT evaluate to a value of
any other kind.

Its type SHALL be the join of its present branches with the type of `{}`, by the ordinary rules of
the join in `occurrence-types`: the item type is the branches' item type, and the occurrence is the
branches' occurrence with zero admitted. A conditional with no `else` whose branches produce `T`
SHALL therefore have type `T?`, one whose branches produce `T+` SHALL have type `T*`, and one whose
branches already admit zero SHALL have that type unchanged.

The Rust IR runtime, the TypeScript IR runtime and every code generation target SHALL agree on this.

#### Scenario: An untaken conditional is the empty sequence
- **WHEN** a file binds `let x:int? = { if c { 1 } }` and `c` evaluates to false
- **THEN** evaluation SHALL produce the empty value

#### Scenario: A taken conditional is its branch
- **WHEN** the same file is evaluated with `c` true
- **THEN** evaluation SHALL produce `1`

#### Scenario: The type is a sequence of what the branch produces
- **WHEN** a file contains `let v = { if c { 1 } }`
- **THEN** the inferred type of `v` SHALL be `int?`

#### Scenario: A branch that is already a sequence does not nest
- **WHEN** a file contains `let xs:string+ = {"a"}` and `let v = { if c { xs } }`
- **THEN** the inferred type of `v` SHALL be `string*`
- **AND** SHALL NOT be a sequence of sequences

#### Scenario: A conditional nested in a conditional is still a sequence of items
- **WHEN** a file binds an element body containing a sibling element and
  `{if c { if d { <A/> } }}` at a content property declared `A+`, with `c` true and `d` false
- **THEN** type checking SHALL accept the binding
- **AND** evaluation SHALL produce the content property holding only the sibling element

### Requirement: A conditional with no `else` contributes no items in a collecting position
An `if` or `if … is` expression with no `else` branch SHALL contribute no items when no branch is
taken and it appears in a collecting position as `sequence-model` defines it — an element body, a
content-property binding, a braced value list, or the body of a `for`. It SHALL NOT contribute an
item of any kind.

This SHALL follow from the requirement above together with the splice rule, and SHALL NOT be a
separate rule that an implementation applies to conditionals: the conditional's value admits zero,
and an item whose value is a sequence contributes its items.

The item type of the surrounding sequence SHALL be determined by the branches that are present. A
conditional with no `else` SHALL NOT widen that type. Its contribution to the occurrence SHALL be
that of an item admitting zero, so a braced value that is only such conditionals admits zero, and
one that also holds an item that is always present does not.

#### Scenario: A false conditional child contributes nothing
- **WHEN** a file binds an element body containing a sibling element and `{if c { <A/> }}`, and `c`
  evaluates to false
- **THEN** evaluation SHALL produce the content property holding only the sibling element

#### Scenario: A true conditional child contributes its items
- **WHEN** the same file is evaluated with `c` true
- **THEN** evaluation SHALL produce the content property holding the sibling element and the
  conditional's element, in source order

#### Scenario: Generated code and the Rust IR runtime agree
- **WHEN** a source containing a conditional child with no `else` is evaluated by the Rust IR runtime and
  by generated TypeScript, with the condition false
- **THEN** both SHALL produce the same content property value

#### Scenario: A conditional child does not widen the declared element type
- **WHEN** a file binds `{<A/>{if c { <A/> }}}` at a content property declared `A+`
- **THEN** type checking SHALL accept the binding, the body's occurrence being at least one

#### Scenario: An explicit null item is not an untaken conditional
- **WHEN** a file contains `let xs:string* = { "a" {} }`
- **THEN** evaluation SHALL produce a one-item sequence, since there is no null value to write and a
  written `{}` contributes nothing, exactly as an untaken conditional does

#### Scenario: A conditional that is the whole body produces the empty list when not taken
- **WHEN** a file binds an element body that is exactly `{if c { <A/> }}` at a content property declared `A+`
- **THEN** type checking SHALL reject the binding, naming `A?` and `A+`
- **AND** the same body at a content property declared `items?:A+` SHALL be accepted

#### Scenario: A conditional as a `for` body contributes nothing on the iterations it is not taken
- **WHEN** a file contains `let evens(ns:int+): int* = {for n in ns { if (n % 2 == 0) { n } }}`
- **THEN** type checking SHALL accept the return type, the body's item type being `int`
- **AND** evaluation over `1 2 3 4` SHALL produce `2 4`, with no empty item between them

#### Scenario: A one-item braced value is the same expression as a braced value of several
- **WHEN** a file contains `let xs:int? = { if c { 1 } }` and `let ys:int+ = { if c { 1 } 2 }`
- **THEN** type checking SHALL accept both
- **AND** SHALL apply the same rule to the conditional in each, the number of items written
  alongside it making no difference
