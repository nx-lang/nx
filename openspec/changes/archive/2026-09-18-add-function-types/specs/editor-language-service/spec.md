## MODIFIED Requirements

### Requirement: Hover content is markdown written in NX
Hover content SHALL be markdown. Where the content includes a signature, a type, or any other
fragment of NX, that fragment SHALL be emitted in a fenced code block tagged `nx`, so that a client
which highlights NX renders it as code rather than as prose.

Every such fragment SHALL be spelled the way NX spells it. A hover SHALL NOT introduce a keyword,
punctuation, or ordering that does not appear in the language — the fragment shown for a declaration
is the declaration as an author would write it. A function type SHALL be spelled as source spells
it, `<function Name:Type ... />: Result`, parenthesized under a suffix, and never in an arrow form.

A hover SHALL state what kind of thing the position is exactly once. Where the kind is not evident
from the NX fragment itself, it SHALL be written as a parenthesized prefix on the fragment's own
line rather than as a separate sentence repeating the fragment.

#### Scenario: A function declaration's hover shows an NX signature
- **WHEN** a client requests hover on the name of a function declared
  `let add(count:int): int = ...`
- **THEN** the hover content SHALL contain a fenced code block tagged `nx`
- **AND** the fenced content SHALL spell the declaration as `let add(count:int): int`
- **AND** the hover content SHALL NOT describe the declaration with the word `function`, which is
  a keyword only inside a function type

#### Scenario: A type alias declaration's hover shows an NX signature
- **WHEN** a client requests hover on the name of a declaration `type Size = int`
- **THEN** the fenced content SHALL spell it as `type Size = int`

#### Scenario: An element-style function's hover shows an NX signature
- **WHEN** a client requests hover on the name of a declaration
  `let <Panel title:string /> = <div />`
- **THEN** the fenced content SHALL spell it as `let <Panel title:string />`

#### Scenario: A position whose kind is not evident states it once
- **WHEN** a client requests hover on a function parameter `count` of declared type `int`
- **THEN** the fenced content SHALL read `(parameter) count: int`
- **AND** the hover content SHALL state the word `parameter` exactly once

#### Scenario: A function-typed property's hover shows the function type
- **WHEN** a client requests hover on the property `Row` of a declaration
  `component <Section Row:(<function Item:Contact Index:int />: DrawnNode)? /> = { ... }`
- **THEN** the fenced content SHALL read `(property) Section.Row: (<function Item:Contact Index:int />: DrawnNode)?`, qualified by the
  declaring component as every property hover is
- **AND** the hover content SHALL NOT contain `=>`

#### Scenario: A function name used as a value hovers as its declaration
- **WHEN** a client requests hover on `ContactRow` in `ItemTemplate={ContactRow}` where
  `let <ContactRow Item:Contact Index:int />: DrawnNode = ...` is declared
- **THEN** the fenced content SHALL spell the declaration as `let <ContactRow Item:Contact Index:int />: DrawnNode`
