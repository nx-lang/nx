## Purpose

Defines the operators that work with a value that may be absent — the presence test `x?`, the step
`x?.m`, the fallback `x ?? y` and the `{}` pattern — their typing, the narrowing they induce, and the
runtime behaviour every engine shares. The conditional operator `c ? a : b` is not part of the
language.

## ADDED Requirements

### Requirement: Postfix `?` tests presence
The expression `x?` SHALL be a postfix operator binding as tightly as member access, with type
`boolean`, that evaluates to `true` when `x` holds at least one item and `false` when `x` is the
empty value. Its operand SHALL have an occurrence that admits zero (`?` or `*`); on an exactly-one or
`+` operand the system SHALL report that the value is always present. The operand SHALL be evaluated
once. The interpreter, the TypeScript IR runtime and every code generation target SHALL agree.

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

### Requirement: A presence test narrows the tested path
Where the condition of an `if` is a presence test `p?`, a conjunction whose operands include
presence tests, or the negation `!p?`, the system SHALL narrow each tested path within the branch
the test proves present: the branch taken when `p?` is true, and the `else` branch of `!p?`. A
narrowed `T?` SHALL read as `T` and a narrowed `T*` as `T+`. A path SHALL be an identifier followed
by zero or more member steps, written with `.` or `?.`; a test of a chain SHALL narrow every
receiver along it, so a step written `?.` in the test may be written `.` in the branch. Narrowing
SHALL be scoped to the branch and SHALL NOT escape it. No other expression form SHALL narrow; in
particular a disjunction SHALL NOT. Because NX values are immutable, a narrowing SHALL NOT be
invalidated by anything in the branch.

#### Scenario: A tested field is read as present in the branch
- **WHEN** a file contains `type Book = { title:string author?:Person }` and `let byline(b:Book): string = { if b.author? { "by " + b.author.name } else { "anonymous" } }`
- **THEN** type checking SHALL accept the function
- **AND** `b.author` SHALL have type `Person` inside the first branch and `Person?` in the second

#### Scenario: A negated test narrows the else branch
- **WHEN** a file contains `let f(o?:string): string = { if !o? { "none" } else { o } }`
- **THEN** type checking SHALL accept the function

#### Scenario: A conjunction narrows each tested operand
- **WHEN** a file contains `let f(a?:string, b?:string): string = { if a? && b? { a + b } else { "" } }`
- **THEN** type checking SHALL accept the function

#### Scenario: A disjunction does not narrow
- **WHEN** a file contains `let f(a?:string, b?:string): string = { if a? || b? { a } else { "" } }`
- **THEN** type checking SHALL reject `a` in the branch, naming `string?` at a `string` site

#### Scenario: A tested chain narrows each receiver
- **WHEN** a file contains `type Person = { name?:Name } type Name = { first:string } type Book = { author?:Person }` and `let f(b:Book): string = { if b.author?.name? { b.author.name.first } else { "" } }`
- **THEN** type checking SHALL accept the function

#### Scenario: Narrowing does not escape its branch
- **WHEN** a file contains `let f(o?:string): string = { if o? { "x" } o }`
- **THEN** type checking SHALL reject the body, because the trailing `o` reads as `string?` outside the branch
- **AND** the diagnostic SHALL name the body's type, `string*`, at a `string` site

#### Scenario: A narrowed zero-or-more is one-or-more
- **WHEN** a file contains `let first(xs?:int+): int+ = { if xs? { xs } else { 0 } }`
- **THEN** type checking SHALL accept the function

### Requirement: `?.` steps through an optional receiver
The expression `x?.m` SHALL be a member access whose receiver has occurrence `?`. It SHALL evaluate
to the empty value when `x` is empty and to `x.m` otherwise, evaluating `x` once. Its type SHALL be
the member's item type with occurrence equal to the join of `?` and the member's occurrence: a member
declared `m:U` reads `U?`, `m?:U` reads `U?`, `m:U+` reads `U*` and `m?:U+` reads `U*`. A plain `.`
on a receiver typed `?` SHALL be rejected with a diagnostic that shows the `?.` spelling. A `?.` on
an exactly-one receiver SHALL be reported as unnecessary. A `?.` or `.` on a `+` or `*` receiver
SHALL be rejected, since a sequence has no members; the diagnostic SHALL suggest `for`. Every engine
SHALL agree.

#### Scenario: A step through an absent receiver is empty
- **WHEN** a file contains `type Book = { author?:Person }` and `let name(b:Book): string? = { b.author?.name }`
- **THEN** type checking SHALL accept the function
- **AND** `name(<Book />)` SHALL evaluate to the empty value and `name(<Book author={p} />)` to `p.name`

#### Scenario: A step joins the member's occurrence
- **WHEN** a file contains `type Person = { nick?:string tags:string+ }`, `type Book = { author?:Person }` and `let f(b:Book) = { b.author?.nick }`, `let g(b:Book) = { b.author?.tags }`
- **THEN** `f` SHALL infer `string?` and `g` SHALL infer `string*`

#### Scenario: A plain dot on an optional receiver is rejected with the fix
- **WHEN** a file contains `type Book = { author?:Person }` and `let f(b:Book) = { b.author.name }`
- **THEN** type checking SHALL reject the access
- **AND** the diagnostic SHALL show `b.author?.name`

#### Scenario: A step on a receiver that is always present is reported
- **WHEN** a file contains `type Book = { author:Person }` and `let f(b:Book) = { b.author?.name }`
- **THEN** type checking SHALL report the `?.` as unnecessary

#### Scenario: A step on a sequence is rejected
- **WHEN** a file contains `let f(ps:Person+) = { ps?.name }`
- **THEN** type checking SHALL reject the access, explaining that a sequence has no members and suggesting `for`

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
- **AND** the interpreter, the TypeScript IR runtime and generated JavaScript SHALL agree

#### Scenario: A fallback on a value that cannot be empty is reported
- **WHEN** a file contains `let f(n:int) = { n ?? 0 }`
- **THEN** type checking SHALL report that the fallback is never taken

#### Scenario: The right operand is evaluated only when needed
- **WHEN** a file contains `let f(o?:int): int = { o ?? fail() }` where `fail` raises a runtime error
- **THEN** `f(1)` SHALL evaluate to `1` without error

#### Scenario: Fallback chains associate to the right
- **WHEN** a file contains `let f(a?:int, b?:int, c:int): int = { a ?? b ?? c }`
- **THEN** type checking SHALL accept the function as `a ?? (b ?? c)`

### Requirement: `{}` is a pattern that matches the empty value
In an `if x is { … }` match, the pattern `{}` SHALL match exactly when the scrutinee is the empty
value. It SHALL be permitted only when the scrutinee's occurrence admits zero; on an exactly-one or
`+` scrutinee it SHALL be reported as never matching. Within the arms that follow a `{}` arm, and
within an `else` arm of a match that has a `{}` arm, a scrutinee that is a path SHALL be narrowed as
a presence test narrows it. A match over a `?`-typed union whose arms cover `{}` and every case SHALL
be exhaustive.

#### Scenario: The empty pattern matches absence
- **WHEN** a file contains `type Book = { author?:Person }` and `let f(b:Book): string = { if b.author is { {} => "anonymous" else => b.author.name } }`
- **THEN** type checking SHALL accept the function, narrowing `b.author` in the `else` arm
- **AND** `f(<Book />)` SHALL evaluate to `"anonymous"`

#### Scenario: The empty pattern completes exhaustiveness over an optional union
- **WHEN** a file contains `type State = idle | busy` and `let f(s?:State): string = { if s is { {} => "none" idle => "idle" busy => "busy" } }`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** omitting the `{}` arm SHALL report the match as not exhaustive

#### Scenario: The empty pattern on a scrutinee that cannot be empty is reported
- **WHEN** a file contains `let f(n:int) = { if n is { {} => 0 else => n } }`
- **THEN** type checking SHALL report the `{}` arm as never matching

### Requirement: There is no conditional operator
The form `c ? a : b` SHALL NOT be an expression. Where a `?` follows an expression, it SHALL be the
presence test; a following `:` SHALL be a syntax error whose diagnostic points the author to `if`.
The diagnostic SHALL fill in the author's operands only when the rewrite parses where the ternary
stood; otherwise it SHALL name the `if condition { a } else { b }` form without filling it in.

#### Scenario: A ternary is a syntax error naming the replacement
- **WHEN** a file contains `let ratio = ready ? 1 : 2`
- **THEN** parsing SHALL reject the expression
- **AND** the diagnostic SHALL show `{ if ready { 1 } else { 2 } }`, braced because an unbraced `let`
  value cannot be an `if`

#### Scenario: A rewrite that would not parse is not suggested
- **WHEN** a file contains `let f(a?:int): boolean = { a? ) : 3 }`
- **THEN** the diagnostic SHALL name the `if condition { a } else { b }` form
- **AND** it SHALL NOT suggest `if a { ) } else { 3 }`
