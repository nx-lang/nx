---
title: 'Expressions'
description: 'Expression syntax and evaluation rules in NX.'
---

Every construct in NX yields a value: literals, conditionals, loops, object creation, and function calls. For formal production rules, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions).

## Literals and Basic Forms
- Numbers, strings and booleans use familiar literal syntax; a sequence is written in braces with
  no delimiter between items.
- `{}` is the empty value: the empty sequence, and the one way to write "no value". There is no
  `null`. It can stand anywhere a value can, including as an item or an operand:
  `{ "a" {} }`, `x == {}`, `if c { {} } else { 1 }`.
- Object creation reuses element syntax.

```nx
type User = { id:string name:string }

let numbers = { 1 2 3 4 5 }
let user = <User id="123" name="Ada"/>
let empty: string* = {}
```

## Operators
The table is in precedence order, tightest first.

| Operators | Operands | Result |
| --- | --- | --- |
| `x.m` `x?.m` `x?` (postfix) | a value with a member; a `?` value with a member; a `?` or `*` value | the member's type; the member's type made optional; `boolean` |
| `-` (prefix) `!` | a number; a `boolean` | the operand's type |
| `??` | a `?` or `*` value and a fallback | the left operand with zero removed, joined with the right |
| `*` `/` `%` | two numbers | the narrowest numeric type both operands widen to |
| `+` `-` | two numbers | the narrowest numeric type both operands widen to |
| `+` | a `string` and a `string`, number or `boolean` | `string` |
| `..` `..=` | two numbers | a [`Range`](/reference/syntax/types#ranges) of the operands' common numeric type |
| `<` `<=` `>` `>=` | two numbers, or two strings | `boolean` |
| `==` `!=` | two values of compatible types | `boolean` |
| `&&` | two `boolean`s | `boolean`; the right operand runs only when the left does not decide |
| `\|\|` | two `boolean`s | `boolean`; the right operand runs only when the left does not decide |

So the postfix forms bind tightest, then prefix `-` and `!`, then `??`, then `*` `/` `%`, then `+`
`-`, then `..` `..=`, then the comparisons, then `&&`, then `||`. `0..n + 1` is `0..(n + 1)` and
needs no parentheses, and `1..5 == 1..=4` compares the two ranges. Every binary operator except `??`
is left-associative, so `1..5..9` parses — and is then rejected, because a `Range` is not a numeric
bound. `??` is right-associative, so `a ?? b ?? c` is `a ?? (b ?? c)`.

`??` binds *above* arithmetic, which is the opposite of C#, Swift and JavaScript, where it binds
below `+`. In NX, `"by " + b.author?.name ?? "anonymous"` is `"by " + (b.author?.name ?? "anonymous")`:
the fallback applies to the optional name, and the concatenation sees a `string`. The other parse
would hand `+` an operand that may be empty, which is a type error in NX every time, so the tight
binding is the one that reads as written. See [Values that may be empty](#values-that-may-be-empty).

### Ranges
`a..b` is the range from `a` up to but excluding `b`; `a..=b` includes `b`. Both are sugar for
constructing the built-in [`Range`](/reference/syntax/types#ranges) record, so `1..5` is exactly
`<Range T=int start={1} end={5} endInclusive={false} />`, and the two compare equal. Building one
never fails: `5..2` is a valid, empty range.

An integer range is what [`for`](/reference/syntax/for#counting-with-a-range) counts over. A range
is a binary expression, so `{1..5}` is one braced value; a braced list of several takes each range in
parentheses: `{ (0..5) (5..=9) }`.

### Arithmetic and comparison
Operands of different numeric types are widened to the narrowest type both widen to, as described
under [Numeric Types and Conversions](/reference/syntax/types#numeric-types-and-conversions). With
`n: int` and `x: float64`, `n + x` is a `float64` and `n == x` is a `boolean` that compares
numerically, so `2 == 2.0` is `true`. A pair with no common type, `int64` with either float, is
rejected naming both types. Division of two integers is integer division (`7 / 2` is `3`); with a
floating-point operand it is not (`7 / 2.0` is `3.5`). Division or remainder by zero is a runtime
error.

### String concatenation
`+` concatenates when either operand is a `string`. The other operand may be a `string`, any
numeric type, or a `boolean`, and is rendered in its canonical text form:

```nx
type Item = { title:string }

let total(count: int) = { "Total: " + count }          // "Total: 3"
let width(x: float64) = { x + " px" }                  // "1.5 px"
let state(on: boolean) = { "enabled: " + on }          // "enabled: true"
let label(item: Item) = { "Reorder " + item.title }    // a field access is a string like any other
```

Whether a `+` adds or concatenates follows from the operand types, never from how an operand is
written. `+` is left-associative, so the operands of an outer `+` are the results of the inner
ones:

```nx
let a = { 1 + 2 + " items" }   // "3 items": the addition happens first
let b = { "n=" + 1 + 2 }       // "n=12": once there is a string, every `+` to its right joins
```

Only values with one obvious text are converted. A value that may be empty, such as a `string?`, a
sequence, a record, a union case and a function are rejected with a diagnostic naming the type, so
`"value: " + maybeName` is an error while `maybeName` is a `string?`. Give it a fallback,
`"value: " + maybeName ?? "none"`, or test it first.

### Canonical text forms
Every backend — the interpreter, the NX IR runtime and generated code — prints a primitive the same
way, wherever a value becomes text:

| Type | Text |
| --- | --- |
| `int`, `int32`, `int64` | Decimal digits, with a leading `-` when negative: `3`, `-42`. |
| `float64` | As JavaScript prints a number: an integral value has no fraction (`1`, not `1.0`), a fractional value prints the shortest digits that round-trip (`0.1`), magnitudes at or above 10^21 or below 10^-6 use an exponent (`1e+21`, `1e-7`), negative zero prints as `0`, and the non-finite values are `NaN`, `Infinity` and `-Infinity`. |
| `float32` | The same rules, with the shortest digits that round-trip as a `float32`, so a `float32` holding `0.1` prints `0.1`. |
| `boolean` | `true` or `false`. |

## Conditional Expressions
- `if` expressions always return a value; the branches are joined to one type.
- Braces are required around every branch.
- An `if` with no `else` is read as having an `else { }`: over a branch of type `T` it is a `T?`,
  the empty value when the condition is false. See [if](/reference/syntax/if#when-there-is-no-else).
- There is no `c ? a : b` operator. `if c { a } else { b }` is the expression form, and a `?` after
  a value is the [presence test](#values-that-may-be-empty).

```nx
type Welcome = { name:string }
type LoginPrompt = { message:string = "Sign in" }

let banner(isAuthenticated:boolean, name:string) = {
  if isAuthenticated {
    <Welcome name={name} />
  } else {
    <LoginPrompt />
  }
}

let cssClass(isActive:boolean) = { if (isActive) { "active" } else { "inactive" } }
let badge(isNew:boolean) = { if isNew { "new" } }    // string?
```

### Pattern Matching
Match-style `if` evaluates arms in order; the first match wins.

```nx
type AccessLevel = read_only | member | admin

let dashboard(accessLevel:AccessLevel) = {
  if accessLevel is {
    AccessLevel.admin => <AdminPanel/>
    AccessLevel.member => <MemberHome/>
    else => <ReadOnlyView/>
  }
}
```

- Arms never fall through; each case is independent.
- Multiple patterns can share a result: `"saturday", "sunday" => <Weekend/>`.
- Constant-case and scalar matches may use `else` for fallbacks. For union scrutinees, omitting
  `else` requires every union case to be covered.
- A union case arm narrows the matched identifier inside that arm.
- `{}` is a pattern: it matches the empty value, and the arms after it read the scrutinee as
  present. See [Matching the empty value](#matching-the-empty-value).

```nx
type LoadState =
  | idle
  | failed { message:string }
  | loaded { count:int }

let label(state:LoadState) = {
  if state is {
    LoadState.idle => "Idle"
    LoadState.failed => state.message
    LoadState.loaded => "Loaded"
  }
}
```

## Values that may be empty
A value whose type is `T?` or `T*` may be the empty value `{}`. An optional property reads as one —
`author?:Person` reads as `Person?` — and so does an `if` with no `else`. Such a value does not
satisfy an exactly-one site, and a plain `.` cannot read a member through it, so three operators
and one pattern handle the empty case. Each is defined the same way in the interpreter, the NX IR
runtime and generated code.

```nx
type Person = { name:string nick?:string }
type Book = { title:string author?:Person tags?:string+ }
```

The examples in this section build on these two types.

### Presence test: `x?`
Postfix `?` tests whether a value holds an item. It is a `boolean`, binds as tightly as member
access, and evaluates its operand once. On a `*` value it is non-emptiness.

```nx fragment
let hasAuthor(b:Book): boolean = { b.author? }
let tagged(b:Book): boolean = { b.tags? }
let both(a?:int, b?:int): boolean = { !a? || (a? && b?) }
```

A test on a value that cannot be empty — an exactly-one or `+` value — is reported, since it is
always `true`.

### Step: `x?.m`
`x?.m` reads a member through a `?` receiver: it is `{}` when `x` is empty and `x.m` otherwise. The
result is the member's type made optional, joined with the member's own occurrence: a member
declared `m:U` or `m?:U` reads `U?`, and one declared `m:U+` or `m?:U+` reads `U*`.

```nx fragment
let authorName(b:Book): string? = { b.author?.name }   // string?
let nick(b:Book): string? = { b.author?.nick }         // string?, not string??
```

A plain `.` on a `?` receiver is an error whose diagnostic shows the `?.` spelling, and `?.` on a
receiver that cannot be empty is reported as unnecessary. Neither `.` nor `?.` reads a member of a
`+` or `*` value: a sequence has no members, and a `for` visits its items.

```nx invalid
type Person = { name:string nick?:string }
type Book = { title:string author?:Person tags?:string+ }

// Rejected: `b.author` may be empty; write `b.author?.name`.
let bad(b:Book) = { b.author.name }
```

### Fallback: `x ?? y`
`x ?? y` is `x` when `x` holds an item and `y` otherwise; `y` is evaluated only in that case. Its
type joins `x` with zero removed and `y`, so `string? ?? string` is a `string` and
`string* ?? string+` is a `string+`. When that type admits many, an item from either side becomes a
sequence of one, so `tagList` below gives `["untagged"]` for a book with no tags. The operator is right-associative and binds above arithmetic
and every comparison, so a fallback at the end of a concatenation applies to the last operand, not
to the whole expression:

```nx fragment
let subtitle(b:Book, fallback?:string): string = { b.author?.name ?? fallback ?? "none" }
let byline(b:Book): string = { "by " + b.author?.name ?? "anonymous" }
let tagList(b:Book): string+ = { b.tags ?? "untagged" }
```

`byline` reads as `"by " + (b.author?.name ?? "anonymous")`. This is the reverse of C#, where `??`
binds below `+` and the same line would mean `("by " + b.author?.name) ?? "anonymous"`; in NX that
parse would hand `+` a `string?`, which is a type error, so the tight binding is the only one that
type checks. A fallback whose left operand cannot be empty is reported, since it is never taken.

### Narrowing
When the condition of an `if` is a presence test, the tested path is read as present in the branch
the test proves: `T?` reads as `T` and `T*` as `T+`. `!p?` narrows the `else` branch, `&&` narrows
every tested operand, and a test of a chain such as `b.author?.name?` narrows every receiver along
it, so a step written `?.` in the test may be written `.` in the branch. A `||` narrows nothing, and
a narrowing never escapes its branch.

```nx fragment
let credit(b:Book): string = { if b.author? { "by " + b.author.name } else { "anonymous" } }
let orNone(o?:string): string = { if !o? { "none" } else { o } }
let joined(a?:string, b?:string): string = { if a? && b? { a + b } else { "" } }
let firstTag(b:Book): string+ = { if b.tags? { b.tags } else { "untagged" } }
```

```nx invalid
// Rejected: `||` does not narrow, so `a` is still a string? in the branch.
let either(a?:string, b?:string): string = { if a? || b? { a } else { "" } }
```

A path is an identifier followed by member steps. Nothing else narrows — not a call, not a `let`
bound to a test's result — and because NX values are immutable, nothing inside the branch can undo
a narrowing.

### Matching the empty value
In `if x is { … }`, the pattern `{}` matches exactly when `x` is empty. It is allowed only when `x`
may be empty, and the arms after it, including `else`, read a path scrutinee as present. Over a
`?`-typed union, a match whose arms cover `{}` and every case is exhaustive.

```nx fragment
type State = idle | busy

let describe(b:Book): string = {
  if b.author is {
    {} => "anonymous"
    else => b.author.name
  }
}

let stateLabel(s?:State): string = {
  if s is {
    {} => "none"
    idle => "idle"
    busy => "busy"
  }
}
```

## Iteration Expressions
`for` yields a new sequence by looping over an input sequence. Both item-only and item/index forms exist; the item comes first and the optional
second name is its zero-based index.

```nx
type User = { name:string }
type Item = { label:string }

let cards(users:User+) = {
  for user in users {
    <UserCard user={user}/>
  }
}

let stripedRows(items:Item+) = {
  for item, index in items {
    <Row key={index} className={if (index % 2 == 0) { "even" } else { "odd" }}>
      {item.label}
    </Row>
  }
}
```

The result's occurrence multiplies the input's by the body's: `cards` over a `User+` with a body
that yields one card is a `UserCard+`. A `for` over a `?` value yields at most one item, and a body
that may yield nothing makes the result a `*`.

You can nest `if` inside a `for` to implement filter-like projections. The `if` has no `else`, so
on the iterations it is not taken it contributes nothing:

```nx
let evens(numbers:int+): int* = {
  for n in numbers {
    if n % 2 == 0 { n }
  }
}
```

## Object and Element Creation
Because objects share the same syntax as components, you can inline structured data anywhere an expression is expected.

```nx
type User = { id:string name:string email:string }

let payload = <User id="456" name="Grace" email="grace@example.com"/>

<UserCard user={payload}/>
```

## Error Handling and Guarding
Use a fallback, or a presence test with an `if`, to supply a default where a value may be empty.

```nx
type User = { name:string avatarUrl?:string }

let avatarUrl(user:User) = { user.avatarUrl ?? "/default-avatar.svg" }

let avatarUrlIf(user:User) = {
  if user.avatarUrl? {
    user.avatarUrl
  } else {
    "/default-avatar.svg"
  }
}
```

## See also
- Reference: [Types – Numeric Types and Conversions](/reference/syntax/types#numeric-types-and-conversions)
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [if](/reference/syntax/if), [for](/reference/syntax/for)
- Grammar: [nx-grammar.md – Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
