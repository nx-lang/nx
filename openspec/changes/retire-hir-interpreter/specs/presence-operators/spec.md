## MODIFIED Requirements

### Requirement: Postfix `?` tests presence
The expression `x?` SHALL be a postfix operator binding as tightly as member access, with type
`boolean`, that evaluates to `true` when `x` holds at least one item and `false` when `x` is the
empty value. Its operand SHALL have an occurrence that admits zero (`?` or `*`); on an exactly-one or
`+` operand the system SHALL report that the value is always present. The operand SHALL be evaluated
once. The Rust IR runtime, the TypeScript IR runtime and every code generation target SHALL agree.

#### Scenario: A presence test is a boolean
- **WHEN** a file contains `type Book = { title:string author?:Person }` and `let has(b:Book): boolean = { b.author? }`
- **THEN** type checking SHALL accept the function
- **AND** `has(<Book title="A" />)` SHALL evaluate to `false` and `has(<Book title="A" author={p} />)` to `true`

#### Scenario: A test on a value that cannot be empty is reported
- **WHEN** a file contains `let f(n:int, xs:int+) = { n? && xs? }`
- **THEN** type checking SHALL report both operands as always present

#### Scenario: A test on zero-or-more is non-emptiness
- **WHEN** a file contains `let any(xs?:int+): boolean = { xs? }`
- **THEN** `any({})` SHALL evaluate to `false` and `any({1})` to `true`

#### Scenario: A test composes with negation and conjunction
- **WHEN** a file contains `let f(a?:int, b?:int): boolean = { !a? || (a? && b?) }`
- **THEN** type checking SHALL accept the function

### Requirement: `??` supplies a fallback for an empty value
The expression `x ?? y` SHALL evaluate to `x` when `x` holds at least one item and to `y` otherwise;
`y` SHALL be evaluated only in the second case. The left operand SHALL have an occurrence that
admits zero, and on an exactly-one or `+` left operand the system SHALL report that the fallback is
never taken. The result's item type SHALL be the join of the operands' item types, and its
occurrence SHALL be the join of the left occurrence with zero removed (`?` becomes exactly one, `*`
becomes `+`) and the right occurrence. When the result admits many, an operand that does not SHALL
produce a sequence of its item, as a branch of any join does (`sequence-model`), so `xs ?? 5`
typed `int+` evaluates to `[5]` rather than `5`. `??` SHALL be right-associative and SHALL bind more tightly
than every binary arithmetic, comparison and logical operator, and less tightly than prefix `-` and
`!`, postfix `?`, `?.` and member access, so `-x ?? 1` is `(-x) ?? 1`. Every engine SHALL agree.

#### Scenario: A fallback makes an optional exactly one
- **WHEN** a file contains `type Book = { subtitle?:string }` and `let sub(b:Book): string = { b.subtitle ?? "none" }`
- **THEN** type checking SHALL accept the function
- **AND** `sub(<Book />)` SHALL evaluate to `"none"` and `sub(<Book subtitle="S" />)` to `"S"`

#### Scenario: A fallback binds tighter than concatenation
- **WHEN** a file contains `type Book = { author?:Person }` and `let byline(b:Book): string = { "Author: " + b.author?.name ?? "Anonymous" }`
- **THEN** type checking SHALL accept the function, parsing the body as `"Author: " + (b.author?.name ?? "Anonymous")`
- **AND** `byline(<Book />)` SHALL evaluate to `"Author: Anonymous"`

#### Scenario: The right operand decides how many the result admits
- **WHEN** a file contains `let a(xs?:int+, ys:int+) = { xs ?? ys }`, `let b(xs?:int+, ys?:int+) = { xs ?? ys }`, `let c(o?:int, p?:int) = { o ?? p }`
- **THEN** `a` SHALL infer `int+`, `b` SHALL infer `int*` and `c` SHALL infer `int?`

#### Scenario: A fallback that admits many produces a sequence from an item
- **WHEN** a file contains `let orFive(xs?:int+) = { xs ?? 5 }`, `let bumped(xs?:int+) = { for x in xs ?? 5 { x + 1 } }` and `let itemOr(ys:int+, o?:int) = { o ?? ys }`
- **THEN** `orFive({})` SHALL evaluate to the sequence `5`, `bumped({})` to the sequence `6` and `itemOr({2 3}, 1)` to the sequence `1`, each a one-item sequence rather than a bare item
- **AND** the Rust IR runtime, the TypeScript IR runtime and generated JavaScript SHALL agree

#### Scenario: A fallback on a value that cannot be empty is reported
- **WHEN** a file contains `let f(n:int) = { n ?? 0 }`
- **THEN** type checking SHALL report that the fallback is never taken

#### Scenario: The right operand is evaluated only when needed
- **WHEN** a file contains `let f(o?:int): int = { o ?? fail() }` where `fail` raises a runtime error
- **THEN** `f(1)` SHALL evaluate to `1` without error

#### Scenario: Fallback chains associate to the right
- **WHEN** a file contains `let f(a?:int, b?:int, c:int): int = { a ?? b ?? c }`
- **THEN** type checking SHALL accept the function as `a ?? (b ?? c)`
