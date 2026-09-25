# implicit-primitive-conversions Specification

## Purpose
Defines which conversions between primitive types happen without the author writing anything: the
rule that admits a conversion, the numeric widening lattice, string `+` with a primitive operand,
text body content at a `string` content property, and the canonical text form every backend prints
a primitive in.

## Requirements

### Requirement: A conversion is implicit only when it is total, exact, and has one obvious result
The system SHALL apply a conversion without an explicit request from the author only when the
conversion is defined for every value of the source type, preserves the value exactly, and has a
single result the author would predict. Every implicit conversion this capability admits SHALL
satisfy all three tests, and the system SHALL NOT admit an implicit conversion that fails any one of
them. A conversion that fails a test SHALL be a type error at the site, with a diagnostic that names
the source type and the target type.

#### Scenario: A lossy numeric conversion is not implicit
- **WHEN** a value of type `int64` is written at a site whose declared type is `float64`
- **THEN** analysis SHALL reject the binding
- **AND** the diagnostic SHALL name `int64` and `float64`

#### Scenario: A conversion with more than one plausible result is not implicit
- **WHEN** a value of type `string` is written at a site whose declared type is `int`
- **THEN** analysis SHALL reject the binding

#### Scenario: A conversion that is undefined for some values is not implicit
- **WHEN** a value of type `int?` is written at a site whose declared type is `int`
- **THEN** analysis SHALL reject the binding

### Requirement: Numeric types widen in one direction
The system SHALL accept a value of one numeric primitive type at a site of another only when the
source type widens to the target. The widenings SHALL be exactly: `int32` to `int`, `int32` to
`int64`, `int` to `int64`, `float32` to `float64`, `int32` to `float64`, and `int` to `float64`.
Widening SHALL be transitive along those edges and reflexive. No other pair of distinct numeric
types SHALL be accepted: in particular `int64` SHALL NOT bind at an `int` or `int32` site, `int`
SHALL NOT bind at an `int32` site, `float64` SHALL NOT bind at a `float32` site, `int64` SHALL NOT
bind at any floating-point site, and no integer type SHALL bind at a `float32` site.

The two crossings into floating point are admitted because they are exact: `int` is specified as
exact over ±(2^53−1), which is precisely the integer range a `float64` represents without loss, and
`int32` lies inside it. `int64` exceeds that range and `float32` is exact only to ±2^24, so those
crossings are lossy and are not implicit.

Widening SHALL apply to an expression of any form, not only to a literal, and at every site that
declares a type for the value written there: component and external component property bindings,
property defaults, record field defaults and record field values, annotated `let` bindings, declared
return types, arguments at a typed parameter, each item of a sequence written at a `+` or `*` site,
a content property, and any of those through an occurrence. Widening SHALL apply to the item type
and SHALL leave the occurrence alone: `int?` widens to `float64?`, `int+` to `float64+` and `int*`
to `float64*`, item by item, and SHALL NOT change how many items there are.

#### Scenario: An int expression binds at a float64 site
- **WHEN** a file declares `external component <B v:float64 />` and a component parameter `n:int`,
  and binds `<B v={n} />`
- **THEN** analysis SHALL accept the binding
- **AND** the value of `v` SHALL be a `float64` when the program is evaluated

#### Scenario: An int32 expression binds at an int site
- **WHEN** a file declares `let f(n:int32): int = n`
- **THEN** analysis SHALL accept the declaration

#### Scenario: An int64 expression is rejected at an int32 site
- **WHEN** a file declares `let f(n:int64): int32 = n`
- **THEN** analysis SHALL reject the declaration
- **AND** the diagnostic SHALL name `int64` and `int32`

#### Scenario: An int expression is rejected at an int32 site
- **WHEN** a file declares `external component <B v:int32 />` and a component parameter `n:int`, and
  binds `<B v={n} />`
- **THEN** analysis SHALL reject the binding

#### Scenario: A float64 expression is rejected at a float32 site
- **WHEN** a file declares `external component <B v:float32 />` and a component parameter
  `x:float64`, and binds `<B v={x} />`
- **THEN** analysis SHALL reject the binding

#### Scenario: An int expression is rejected at a float32 site
- **WHEN** a file declares `external component <B v:float32 />` and a component parameter `n:int`,
  and binds `<B v={n} />`
- **THEN** analysis SHALL reject the binding

#### Scenario: Widening applies to each element of a list
- **WHEN** a file declares a property of type `float64+` and binds it to a braced sequence whose
  items are typed `int` and `int32`
- **THEN** analysis SHALL accept the binding
- **AND** every item of the bound sequence SHALL be a `float64` when the program is evaluated

#### Scenario: Widening applies at a nullable site
- **WHEN** a file declares `external component <B v?:float64 />` and a component parameter `n:int`,
  and binds `<B v={n} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: A nullable value widens at a nullable site
- **WHEN** a file declares `let f(n?:int): float64? = { n }` and
  `let g(ns:int+): float64+ = { ns }`
- **THEN** analysis SHALL accept both declarations
- **AND** evaluating `f` with `n` holding `3` SHALL produce the `float64` `3.0`, and with `n` empty
  SHALL produce the empty value
- **AND** `let h(n?:int64): float64? = { n }` SHALL be rejected, naming `int64?` and `float64?`

### Requirement: Mixed numeric operands take the narrowest common widening
When the two operands of an arithmetic operator (`+`, `-`, `*`, `/`, `%`) or a comparison operator
(`==`, `!=`, `<`, `<=`, `>`, `>=`) have different numeric types, the system SHALL type the operation
at the narrowest numeric type both operands widen to, and SHALL widen each operand to it. An
arithmetic operation's result SHALL be that common type, and a comparison's result SHALL be
`boolean`. When no such type exists the system SHALL reject the operation with a diagnostic naming
both operand types.

Under the widening lattice the common types are: `int32` with `int` is `int`; either with `int64`
is `int64`; `float32` with `float64` is `float64`; `int` or `int32` with `float64` is `float64`;
`int` or `int32` with `float32` is `float64`, since neither widens to `float32` but both, and
`float32`, widen exactly to `float64`; `int64` with any floating-point type has no common type.

#### Scenario: int plus float64 is float64
- **WHEN** a file declares `let f(n:int, x:float64) = n + x`
- **THEN** analysis SHALL infer the return type `float64`
- **AND** evaluating `f(1, 1.5)` SHALL produce the `float64` value `2.5`

#### Scenario: int compared with float64 is boolean
- **WHEN** a file declares `let f(n:int, x:float64) = n == x`
- **THEN** analysis SHALL infer the return type `boolean`
- **AND** evaluating `f(2, 2.0)` SHALL produce `true`

#### Scenario: int32 plus float32 is float64
- **WHEN** a file declares `let f(n:int32, x:float32) = n + x`
- **THEN** analysis SHALL infer the return type `float64`

#### Scenario: int64 plus float64 is rejected
- **WHEN** a file declares `let f(n:int64, x:float64) = n + x`
- **THEN** analysis SHALL reject the declaration
- **AND** the diagnostic SHALL name `int64` and `float64`

#### Scenario: Integer division stays integer
- **WHEN** a file declares `let f(n:int32, m:int) = n / m`
- **THEN** analysis SHALL infer the return type `int`
- **AND** evaluating `f(7, 2)` SHALL produce the `int` value `3`

### Requirement: A numeric literal takes the numeric type of its site within its category
An integer literal written at a site whose declared type is an integer primitive SHALL be typed as
that primitive, and a real literal written at a site whose declared type is a floating-point
primitive SHALL be typed as that primitive. The rule SHALL apply at every site that supplies an
expected type, on the same terms as the existing contextual typing of an integer literal at a
floating-point site, and to a negated literal on the same terms.

An integer literal SHALL be rejected at an integer site whose type cannot represent its value, with
a diagnostic naming the literal and the type. A real literal at a `float32` site SHALL be rounded to
the nearest `float32`, as a `float32` literal is in any language, and SHALL NOT be subject to an
exactness check; the existing exactness rule for an integer literal at a floating-point site is
unchanged.

A literal operand of a binary arithmetic or comparison operator whose other operand has a numeric
type SHALL be typed as that operand's type, on the same terms, so that a width the author chose for
one operand is not lost to the literal's default type. A literal operand SHALL take that type
before any promotion, so a literal the other operand's type cannot hold is rejected, not compared or
computed at a wider type. A literal with nothing expecting a type of it SHALL continue to infer
`int` or `float64` by its spelling.

A constant expression, an arithmetic expression (`+`, `-`, `*`, `/`, `%` or negation, nested
freely) whose operands are all numeric literals, SHALL take a site's numeric type on the terms of a
single literal, wherever a literal would. Its value SHALL first be computed at the types its
literals infer on their own, as evaluation would compute it, so an integer `/` truncates at every
site; the result SHALL then be checked and converted as a literal of that value written at the
site. A constant expression that divides by zero, or whose integer result an `int` cannot hold,
SHALL be rejected when a site gives it a type other than the one it infers. A constant expression
that no site gives another type SHALL be left for evaluation.

#### Scenario: Integer literal binds at an int32 site as int32
- **WHEN** a file declares `external component <B v:int32 />` and binds `<B v=1 />`
- **THEN** analysis SHALL accept the binding
- **AND** the type recorded for the literal SHALL be `int32`

#### Scenario: Integer literal out of range for int32 is rejected
- **WHEN** a file declares `external component <B v:int32 />` and binds `<B v=2147483648 />`
- **THEN** analysis SHALL reject the binding
- **AND** the diagnostic SHALL name `int32`

#### Scenario: Real literal binds at a float32 site as float32
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=1.5 />`
- **THEN** analysis SHALL accept the binding
- **AND** the type recorded for the literal SHALL be `float32`

#### Scenario: Literal operand takes the other operand's width
- **WHEN** a file declares `let f(w:float32) = w * 1.5` and `let g(n:int32) = n + 1`
- **THEN** analysis SHALL infer the return type of `f` as `float32`
- **AND** the return type of `g` as `int32`

#### Scenario: A constant expression takes its site's width after folding
- **WHEN** a file declares `type Sizes = { gap:int32 ratio:float32 half:float32 }` and binds
  `<Sizes gap={2 + 3} ratio={1.5 * 2} half={7 / 2} />`
- **THEN** analysis SHALL accept the binding
- **AND** evaluation SHALL bind `gap` to the `int32` `5`, `ratio` to the `float32` `3.0`, and
  `half` to the `float32` `3.0`
- **AND** `let f(w:float32) = w * (1.5 * 2)` SHALL infer the return type `float32`

#### Scenario: A constant expression its site cannot hold is rejected
- **WHEN** a file contains `let big: int32 = { 1000000 * 3000 }`
- **THEN** analysis SHALL reject it with a diagnostic naming `3000000000` and `int32`
- **AND** `let broken: float32 = { 1 / 0 }` SHALL be rejected as a division by zero

#### Scenario: A literal operand out of the other operand's range is rejected
- **WHEN** a file contains `let f(n:int32) = n > 3000000000`
- **THEN** analysis SHALL reject the comparison with a diagnostic naming `int32`

#### Scenario: Unannotated literals keep their default types
- **WHEN** a file contains `let n = 42` and `let x = 1.5`
- **THEN** analysis SHALL infer `n` as `int` and `x` as `float64`

### Requirement: `+` concatenates when either operand is a string
When either operand of `+` has type `string`, the system SHALL type the operation as string
concatenation with result type `string`, provided the other operand's type is `string` or a
stringifiable primitive: `int`, `int32`, `int64`, `float32`, `float64`, or `boolean`. The
non-string operand SHALL be converted to its canonical text form and the two strings joined, the
left operand first. `+` SHALL remain left-associative, so the operands of an outer `+` are the
results of the inner ones.

Both operands of `+` SHALL be exactly-one values. The system SHALL reject `+` when one operand is a
string and the other carries an occurrence — `?`, `+` or `*`, `string?` included — a record, a
union case, a function, `object`, or any other type that is not a stringifiable primitive, with a
diagnostic naming the operand type. For an operand whose occurrence admits zero, the diagnostic
SHALL name `??` as the way to supply a value, as `presence-operators` defines it. Whether a `+`
adds or concatenates SHALL be decided from the checked operand types, and SHALL NOT depend on the
syntactic form of the operands: a field access, a call result, and a parameter of type `string`
concatenate exactly as a string literal does.

#### Scenario: String plus int concatenates
- **WHEN** a file declares `let f(count:int) = "Total: " + count`
- **THEN** analysis SHALL infer the return type `string`
- **AND** evaluating `f(3)` SHALL produce `"Total: 3"`

#### Scenario: Primitive on the left concatenates
- **WHEN** a file declares `let f(x:float64) = x + " px"`
- **THEN** evaluating `f(1.5)` SHALL produce `"1.5 px"`

#### Scenario: Boolean concatenates
- **WHEN** a file declares `let f(on:boolean) = "enabled: " + on`
- **THEN** evaluating `f(true)` SHALL produce `"enabled: true"`

#### Scenario: Concatenation is left-associative
- **WHEN** a file declares `let f() = 1 + 2 + " items"` and `let g() = "n=" + 1 + 2`
- **THEN** evaluating `f()` SHALL produce `"3 items"`
- **AND** evaluating `g()` SHALL produce `"n=12"`

#### Scenario: A string field access concatenates
- **WHEN** a file declares `type Item = { title:string }` and `let f(item:Item) = "Reorder " + item.title`
- **THEN** analysis SHALL infer the return type `string`
- **AND** evaluating `f(<Item title="Rust" />)` SHALL produce `"Reorder Rust"`

#### Scenario: String plus a record is rejected
- **WHEN** a file declares `type Item = { title:string }` and `let f(item:Item) = "Item: " + item`
- **THEN** analysis SHALL reject the declaration
- **AND** the diagnostic SHALL name `Item`

#### Scenario: String plus null is rejected
- **WHEN** a file declares `let f(s?:string) = "value: " + s`
- **THEN** analysis SHALL reject the declaration, naming `string?`
- **AND** the diagnostic SHALL name `??` as the fix
- **AND** `let g(s?:string) = "value: " + (s ?? "")` SHALL be accepted

#### Scenario: String plus a list is rejected
- **WHEN** a file declares `let f(xs:int+) = "items: " + xs`
- **THEN** analysis SHALL reject the declaration, naming `int+`

### Requirement: Text body content binds to a string content property as one string
When element body content binds to a content property declared `text:string` or `text?:string` and
the body consists of text runs and braced values, the system SHALL bind the property to a single
string: the pieces in source order, each text run as written and each braced value in its canonical
text form. Line breaks SHALL be treated as layout: each stretch of whitespace that contains a line
break, whether between two pieces or inside a text run, SHALL become one space, and whitespace
within a line SHALL be kept as written. A text run that begins the body SHALL lose its leading
whitespace and a text run that ends it its trailing whitespace. A braced value SHALL keep its text
as is wherever it stands, and so SHALL raw text (`<Tag:raw>`). A typed text body (`<Tag:markdown>`)
SHALL join its text runs and its `@{}` values on the same terms as a plain body, and SHALL NOT lose
its text, except that its line breaks SHALL be kept as written rather than read as layout: a typed
body is text for a processor the host supplies, which reads those line breaks itself. What SHALL
come off a typed body is the indentation its lines share, the line break that follows the open tag,
and a whitespace-only last line before the close tag. An escape in a text run (`\@`, `\{`, `\}`)
SHALL bind as the character it escapes. A braced value SHALL be accepted only when its type is
`string` or a stringifiable primitive, exactly one; any other type SHALL be rejected with a
diagnostic naming the type. The rule SHALL apply to a body that is a single text run or a single
braced number or boolean as well. A body that is a single braced `string` SHALL bind as that string.

This rule SHALL apply only where body content binds to a declared content property. Body content
under a tag that resolves to no declaration is not type-checked today and SHALL be unchanged by this
requirement.

#### Scenario: Text and a braced int bind as one string
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(count:int) = <Label>Total: {count}</Label>`
- **THEN** analysis SHALL accept the body
- **AND** evaluating `f(3)` SHALL bind `text` to `"Total: 3"`

#### Scenario: Interior whitespace between braced values is kept
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(first:string, last:string) = <Label>{first} {last}</Label>`
- **THEN** evaluating `f("Ada", "Lovelace")` SHALL bind `text` to `"Ada Lovelace"`

#### Scenario: Surrounding whitespace from layout is removed
- **WHEN** a `Label` body is written across lines as an indented `Total: {count}` between the open
  and close tags
- **THEN** evaluating it with `count` as `3` SHALL bind `text` to `"Total: 3"`

#### Scenario: Line breaks between pieces read as one space
- **WHEN** a `Label` body is written as `Total:`, `{count}` and `items` on three indented lines
  between the open and close tags
- **THEN** evaluating it with `count` as `3` SHALL bind `text` to `"Total: 3 items"`, the same as
  `<Label>Total: {count} items</Label>`

#### Scenario: Braced values on separate lines are joined with a space
- **WHEN** a `Label` body is written as `{first}` and `{last}` on two indented lines
- **THEN** evaluating it with `"Ada"` and `"Lovelace"` SHALL bind `text` to `"Ada Lovelace"`

#### Scenario: A text run wrapped across lines reads as one line
- **WHEN** a `Label` body is the text `Two  spaces` on one line and `and a wrapped line` indented on
  the next
- **THEN** evaluating it SHALL bind `text` to `"Two  spaces and a wrapped line"`

#### Scenario: A line break inside a braced string is kept
- **WHEN** a `Label` body is `first`, then a braced string literal containing a line break, then
  `second`
- **THEN** evaluating it SHALL bind `text` to `"first\nsecond"`

#### Scenario: A single braced number binds as its text
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(count:int) = <Label>{count}</Label>`
- **THEN** analysis SHALL accept the body
- **AND** evaluating `f(3)` SHALL bind `text` to `"3"`

#### Scenario: A braced string at the edge of a body keeps its spaces
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(n:int) = <Label>{"  pad "}{n}</Label>`
- **THEN** evaluating `f(2)` SHALL bind `text` to `"  pad 2"`

#### Scenario: A typed text body keeps its line breaks and loses its indentation
- **WHEN** a file contains `type Label = { content text:string }` and a `let` whose body is a
  `<Label:markdown>` element holding `# Title`, a blank line, `Total: @{count}`, a blank line, and
  the two list items `- one` and `- two`, each line indented by four spaces
- **THEN** evaluating it with `count` holding `3` SHALL bind `text` to
  `"# Title\n\nTotal: 3\n\n- one\n- two"`
- **AND** a line indented past the shared indentation SHALL keep the rest of its indentation

#### Scenario: A typed text body keeps its text
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(n:int) = <Label:markdown>a @{n} b</Label>`
- **THEN** evaluating `f(2)` SHALL bind `text` to `"a 2 b"`

#### Scenario: An escape in a text body binds as the character it escapes
- **WHEN** a file contains `type Label = { content text:string }` and
  `let f(n:int) = <Label:markdown>at \@ sign @{n}</Label>`
- **THEN** evaluating `f(2)` SHALL bind `text` to `"at @ sign 2"`

#### Scenario: A braced record in a string body is rejected
- **WHEN** a file contains `type Label = { content text:string }`, `type Item = { title:string }`,
  and `let f(item:Item) = <Label>Item: {item}</Label>`
- **THEN** analysis SHALL reject the body
- **AND** the diagnostic SHALL name `Item`

#### Scenario: A text body binds at an optional string content property
- **WHEN** a file contains `type Label = { content text?:string }` and
  `let f(count:int) = <Label>Total: {count}</Label>`
- **THEN** analysis SHALL accept the body
- **AND** evaluating `f(3)` SHALL bind `text` to `"Total: 3"`, and `<Label />` SHALL leave `text`
  empty

#### Scenario: An Element content property is unchanged
- **WHEN** a file declares `component <Panel content body:Element /> = { <section>{body}</section> }`
  and a call site writes `<Panel>Total: {count}</Panel>`
- **THEN** the body SHALL bind as it does today, without stringification

### Requirement: Every primitive has one canonical text form
The system SHALL define one text form for each stringifiable primitive, and every backend — the
interpreter, the NX IR runtime, and generated code — SHALL produce that same text wherever this
capability converts a value to a string. The forms SHALL be:

- `int`, `int32`, `int64`: the value's decimal digits with no grouping, no leading zeros, and a
  leading `-` when negative.
- `float64`: the ECMAScript `Number::toString` form. An integral value prints without a fraction
  (`1`, not `1.0`), a fractional value prints the shortest digits that round-trip to the same
  `float64`, magnitudes at or above 10^21 or below 10^-6 use the exponent form (`1e+21`, `1e-7`),
  negative zero prints as `0`, and the non-finite values print as `NaN`, `Infinity` and `-Infinity`.
- `float32`: the same rules, with the shortest digits that round-trip to the same `float32`, so a
  `float32` holding `0.1` prints as `0.1` and not as the expansion of its `float64` widening.
- `boolean`: `true` or `false`.

Backends SHALL agree on an integer's text wherever they agree on its value: over the ±(2^53−1)
range `int` is specified to (`primitive-type-names`). An `int64` beyond that range is carried as a
JavaScript `number` until the `int64` carrier change, and its text is outside this guarantee.

#### Scenario: Integral floats print without a fraction
- **WHEN** a `float64` holding `1.0` is concatenated to a string
- **THEN** the text SHALL be `1`

#### Scenario: Fractional floats print the shortest round-trip digits
- **WHEN** a `float64` holding `0.1` is concatenated to a string
- **THEN** the text SHALL be `0.1`

#### Scenario: Large magnitudes use the exponent form
- **WHEN** a `float64` holding `1e21` is concatenated to a string
- **THEN** the text SHALL be `1e+21`

#### Scenario: A float32 prints as a float32
- **WHEN** a `float32` holding the nearest representation of `0.1` is concatenated to a string
- **THEN** the text SHALL be `0.1`

#### Scenario: Backends agree
- **WHEN** the same program concatenates the same `float64` value to a string
- **THEN** the interpreter, the NX IR runtime, and generated JavaScript SHALL produce identical text

### Requirement: A widened value takes the site's type at run time
Where analysis accepted a value by widening, evaluation SHALL produce a value of the site's declared
type, not a value of the source type that a consumer is expected to widen. Mixed-type arithmetic
and comparison SHALL operate on the widened operands, so a comparison between an `int` and a
`float64` SHALL compare numerically. The canonical output of a program SHALL reflect the site's
type wherever the two are distinguishable.

#### Scenario: An int bound at a float64 property evaluates as a float
- **WHEN** a program binds a parameter `n:int` holding `3` at a `float64` property and is evaluated
- **THEN** the property's value SHALL be the `float64` `3.0`
- **AND** it SHALL NOT be an integer value

#### Scenario: Mixed comparison compares numerically
- **WHEN** a program evaluates `n < x` with `n:int` holding `2` and `x:float64` holding `2.5`
- **THEN** the result SHALL be `true`

### Requirement: A number the host supplies takes the width of its site
A number that enters evaluation from the host, as a component property, a state field, an action
payload or an argument of an entry call, SHALL take the numeric type the site declares, on the terms
a literal written at that site does, because a host format such as JSON has no way to spell
`int32` or `float32`. An integer SHALL be accepted at an `int32` site when it is in range and at a
`float32` site when a `float32` represents it exactly, and a real SHALL be rounded to the nearest
`float32` at a `float32` site. A number that cannot take the site's type SHALL fail evaluation with
a diagnostic naming the value and the type. Values that checked NX produces are not affected, since
analysis already guarantees they never narrow.

#### Scenario: A JSON number binds at an int32 and a float32 property
- **WHEN** a component declares `count:int32` and `ratio:float32` and the host supplies the props
  `{ "count": 7, "ratio": 0.1 }`
- **THEN** evaluation SHALL bind `count` to the `int32` `7` and `ratio` to the `float32` nearest
  `0.1`

#### Scenario: A host number out of range is refused
- **WHEN** the host supplies `3000000000` for an `int32` property
- **THEN** evaluation SHALL fail with a diagnostic saying the value is out of range for `int32`

### Requirement: A join takes the narrowest type its branches widen to, and its branches widen to it
The branches of an `if` with an `else`, the arms of a match, and the items of a braced sequence
SHALL be joined into one type, whose item type SHALL be the narrowest type every branch's item type
widens to. A branch whose numeric type is narrower than the join's SHALL evaluate to a value of the
join's numeric type, on the same terms as a value bound at a declared site, including item by item
through an occurrence. The widening SHALL apply to the branch's result: the branch SHALL compute at
its own type, so its operators SHALL be the ones its own operands select.

The join's occurrence SHALL be the least upper bound of the branches' occurrences, as
`occurrence-types` defines: joining `A?` with `B` or with `B?` SHALL yield the join of `A` and `B`
with the `?` occurrence, and a branch that is the empty value SHALL contribute its zero-or-one
occurrence and no item type, so `if c { n }` with no `else` is `int?`. A join whose item types have
no common type other than `object` SHALL yield `object` with the joined occurrence, so `string?`
joined with `float64` is `object?`.

#### Scenario: A narrower branch of a join evaluates at the join's type
- **WHEN** `let pick(b:boolean, n:int, x:float64) = { if b { n } else { x } }` is evaluated with `b`
  true and `n` holding `3`
- **THEN** the result SHALL be the `float64` `3.0`
- **AND** dividing that result by an `int` holding `2` SHALL yield `1.5` on every backend
- **AND** the same SHALL hold for the arms of a match and the items of a braced sequence, item by
  item through an occurrence

#### Scenario: A narrower branch computes at its own type before it widens
- **WHEN** `let half(b:boolean, n:int, x:float64) = { if b { n / 2 } else { x } }` is evaluated
  with `b` true and `n` holding `7`
- **THEN** the result SHALL be the `float64` `3.0` on every backend, not `3.5`
- **AND** a `float32` product in a branch joined with a `float64` SHALL be rounded to a `float32`
  before it widens, on every backend

#### Scenario: A branch joined with null is nullable
- **WHEN** type inference analyzes `if b { n } else { }` with `n:int`
- **THEN** the expression SHALL take the type `int?`
- **AND** `if b { } else { x }` with `x:float64` SHALL take the type `float64?`
- **AND** `if b { n }` with no `else` SHALL take the type `int?` on the same terms

#### Scenario: Nullability is lifted out of a numeric join in either order
- **WHEN** type inference joins an `int?` branch with a `float64` branch, or an `int` branch with a
  `float64?` branch
- **THEN** the join SHALL be `float64?` in both cases
- **AND** `if a { n } else { if b { x } }` with `n:int` and `x:float64`, evaluated with `a` true
  and `n` holding `3`, SHALL yield the `float64` `3.0`

#### Scenario: A join with no common type stays object
- **WHEN** type inference joins a `string?` branch with a `float64` branch
- **THEN** the join SHALL be `object?`
- **AND** joining a `string` branch with a `float64` branch SHALL be `object`
