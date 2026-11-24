## ADDED Requirements

### Requirement: `with` patches a record with a field list or an update value
The language SHALL accept `left with right` wherever an expression is, where `right` is a field
list or an operand. A field list SHALL be `{`, zero or more entries in the attribute form
(`name=value` or `name.path=value`), then `}`; a brace after `with` whose first entry is `name=` or
`name.path=` SHALL be a field list, and `{}` SHALL be an empty field list. A field list SHALL NOT
hold conditional entries or spreads in this change. When `left` is of a record type `T` that is not
an update type, the value SHALL be what applying the entries in order to `left` gives: a field list
without paths SHALL mean `apply(left, <T.Update …/>)` with those entries, and an operand SHALL be
required to be of type `T.Update` and give `apply(left, right)`. The type SHALL be `T`. Each entry's
value SHALL be typed by the field it sets, a present empty value SHALL clear an optional field, and
an unknown field SHALL be rejected as it is in `<T.Update … />`. The same field or path written
twice in one field list SHALL be rejected as a duplicate property is. A field named `with` SHALL be
usable as an entry (`x with { with=3 }`) and as an element attribute (`<T with=3 />`).

#### Scenario: A field list replaces fields and keeps the rest
- **WHEN** a module declares `type User = { name: string email?: string }` and
  `let u = <User name="Ada" email="ada@example.com" />` and `let v = { u with { email={} } }`
- **THEN** `v` SHALL be a `User` whose `name` is `"Ada"` and whose `email` is empty
- **AND** the type of `v` SHALL be `User`

#### Scenario: An update value on the right
- **WHEN** a module declares `let rename = <User.Update name="Bo" />` and
  `let v = { <User name="Ada" /> with rename }`
- **THEN** `v` SHALL be a `User` whose `name` is `"Bo"`

#### Scenario: A field value is typed by its field
- **WHEN** a module declares `type Mode = light | dark`, `type Settings = { mode: Mode }` and
  `let v = { <Settings mode=light /> with { mode=dark } }`
- **THEN** `v`'s `mode` SHALL be `dark`

#### Scenario: An unknown field is rejected
- **WHEN** a module contains `let v = { <User name="Ada" /> with { age=3 } }`
- **THEN** the module SHALL fail with the error for an unknown `User.Update` field `age`

#### Scenario: An update for another type is rejected
- **WHEN** a module declares `type Team = { name: string }` and contains
  `let v = { <User name="Ada" /> with <Team.Update name="Core" /> }`
- **THEN** the module SHALL fail because `Team.Update` is not `User.Update`

### Requirement: A path in a field list patches a nested record
A field-list entry `a.b.c=value` on a record `left` SHALL mean `a=(left.a with { b.c=value })`, with
`left` evaluated once, so that the nested record keeps every field the path doesn't name. Each step
of a path except the last SHALL name a required field of a record type, or the entry SHALL be
rejected with `with-path`, whose message suggests setting the whole field. Entries SHALL apply in the
order written, each on the result of the one before.

#### Scenario: A nested field is replaced and its siblings kept
- **WHEN** a module declares `type Text = { normal: string muted: string }`,
  `type Colors = { accent: string text: Text }`, `let c = <Colors accent="blue"
  text=<Text normal="black" muted="gray" /> />` and `let v = { c with { text.muted="slate" } }`
- **THEN** `v.text.muted` SHALL be `"slate"`, `v.text.normal` `"black"` and `v.accent` `"blue"`

#### Scenario: A path through an optional field is rejected
- **WHEN** a module declares `type Book = { author?: User }` and contains
  `let v = { <Book /> with { author.name="Ada" } }`
- **THEN** the module SHALL fail with `with-path` at `author.name`

#### Scenario: Later entries apply on earlier ones
- **WHEN** a module contains `let v = { c with { text=<Text normal="white" muted="gray" />
  text.muted="silver" } }`
- **THEN** `v.text.normal` SHALL be `"white"` and `v.text.muted` `"silver"`

### Requirement: `with` binds tighter than `??` and arithmetic and chains left to right
`with` SHALL be a left-associative infix operator that binds less tightly than prefix unary
operators, member access and calls, and more tightly than `??`, `*`, `/` and `%`. An operand on its
right SHALL be a name, member access, call, element or braced expression, so `x with y ?? z` SHALL
be `(x with y) ?? z` and `a ?? b with c` SHALL be `a ?? (b with c)`.

#### Scenario: Chained patches apply in order
- **WHEN** a module contains `let v = { <User name="Ada" /> with { name="Bo" } with { name="Cy" } }`
- **THEN** `v.name` SHALL be `"Cy"`

#### Scenario: A member access on the right is the update
- **WHEN** a module declares `type Edits = { rename: User.Update }`,
  `let edits = <Edits rename=<User.Update name="Bo" /> />` and
  `let v = { <User name="Ada" /> with edits.rename }`
- **THEN** `v.name` SHALL be `"Bo"`

#### Scenario: `with` beside `??`
- **WHEN** a module declares `let saved: User? = {}` and `let fallback = <User name="Ada" />`, and
  contains `let v = { saved ?? fallback with { name="Bo" } }`
- **THEN** `v.name` SHALL be `"Bo"`

#### Scenario: A comparison of a patched value
- **WHEN** a module contains `let same = { <User name="Ada" /> with { name="Bo" } == <User name="Bo"
  /> }`
- **THEN** `same` SHALL be `true`

### Requirement: `with` merges update records and extends bundles
`left with right` SHALL be `merge(left, right)` when `left` is of an update type `T.Update`, which
takes this rule before the record rule, with a field list typed as a `T.Update` and an
operand required to be of type `T.Update`; a path entry SHALL be rejected with `with-path`. When
`left` is of type `env.Update`, the value SHALL be a bundle whose overrides are `left`'s followed by
`right`'s, with a field list read as the fields of an `<env.Update … />`, paths included, and an
operand required to be of type `env.Update`.

#### Scenario: Two update records merge
- **WHEN** a module contains `let u = { <User.Update name="Ada" /> with { email="a@b" } }`
- **THEN** `u` SHALL be a `User.Update` with `name` `"Ada"` and `email` `"a@b"` present

#### Scenario: A bundle derived from a bundle
- **WHEN** a module declares `environment space: int = 4` and `environment radius: int = 2`,
  `let brand: env.Update = <env.Update space=8 radius=6 />` and
  `let dense: env.Update = { brand with { space=4 } }`, and the main module contains
  `provide env with dense`
- **THEN** a read of `env.space` with no override around it SHALL be `4` and of `env.radius` `6`

### Requirement: `with` on another type and `with` as a name are rejected
A `with` whose left side is not of a record type, an update type or `env.Update` SHALL be rejected
with `with-target`, naming the type. A left side of an optional or sequence type SHALL be rejected
the same way, with a hint to unwrap it first with `??` or `if`. Since `with` after an operand always
begins a `with` expression, a declaration, parameter, `for` item or binding named `with` SHALL be
rejected with `reserved-name`; a record field named `with`, read as `x.with`, SHALL keep working.

#### Scenario: A `with` on a string is rejected
- **WHEN** a module contains `let v = { "Ada" with { length=3 } }`
- **THEN** the module SHALL fail with `with-target` naming `string`

#### Scenario: An optional left side is rejected
- **WHEN** a module declares `type Book = { author?: User }` and contains
  `let v = { <Book />.author with { name="Bo" } }`
- **THEN** the module SHALL fail with `with-target` naming `User?`

#### Scenario: `with` can't be a binding name
- **WHEN** a module contains `let with = 3`
- **THEN** the module SHALL fail with `reserved-name`

### Requirement: Record `with` runs the same in every engine
A record or update-record `with` SHALL lower to the `apply` and `merge` intrinsics in NX IR, so
that a program written with `with` and the same program written with those intrinsics SHALL produce
the same results and operation counts in the Rust and TypeScript IR runtimes and the same results
in generated JavaScript.

#### Scenario: `with` and `apply` agree
- **WHEN** a conformance program evaluates `u with { name="Bo" }` and `apply(u, <User.Update
  name="Bo" />)`
- **THEN** both SHALL give the same value and the same operation count in both IR runtimes
- **AND** generated JavaScript SHALL give the same value
