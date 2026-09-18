---
title: 'Expressions'
description: 'Expression syntax and evaluation rules in NX.'
---

Every construct in NX yields a value: literals, conditionals, loops, object creation, and function calls. For formal production rules, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions).

## Literals and Basic Forms
- Numbers, strings, booleans, null, and sequences use familiar literal syntax.
- Object creation reuses element syntax.

```nx
let numbers = [1, 2, 3, 4, 5]
let user = <User id="123" name="Ada"/>
let empty: string[] = []
```

## Operators
| Operators | Operands | Result |
| --- | --- | --- |
| `+` `-` `*` `/` `%` | two numbers | the narrowest numeric type both operands widen to |
| `+` | a `string` and a `string`, number or `boolean` | `string` |
| `==` `!=` | two values of compatible types | `boolean` |
| `<` `<=` `>` `>=` | two numbers, or two strings | `boolean` |
| `&&` `\|\|` | two `boolean`s | `boolean`; the right operand runs only when the left does not decide |
| `-` (prefix) | a number | the operand's type |
| `!` | a `boolean` | `boolean` |

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

Only values with one obvious text are converted. `null`, a nullable type such as `string?`, a
record, a list, a union case and a function are rejected with a diagnostic naming the type, so
`"value: " + maybeName` is an error until the null case is handled.

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
- `if` expressions always return a value; both branches must produce compatible types.
- Use braces for clarity, even for single-line branches.

```nx
let banner = if user.isAuthenticated {
  <Welcome user={user}/>
} else {
  <LoginPrompt/>
}

let cssClass = if (isActive) { "active" } else { "inactive" }
```

### Pattern Matching
Match-style `if` evaluates arms in order; the first match wins.

```nx
type AccessLevel = read_only | member | admin

let dashboard = if accessLevel is {
  AccessLevel.admin => <AdminPanel/>
  AccessLevel.member => <MemberHome/>
  else => <ReadOnlyView/>
}
```

- Arms never fall through; each case is independent.
- Multiple patterns can share a result: `"saturday", "sunday" => <Weekend/>`.
- Constant-case and scalar matches may use `else` for fallbacks. For union scrutinees, omitting
  `else` requires every union case to be covered.
- A union case arm narrows the matched identifier inside that arm.

```nx
type LoadState =
  | idle
  | failed { message:string }
  | loaded { count:int }

let state: LoadState =
  <LoadState.failed message={"Offline"} />

let label = if state is {
  LoadState.idle => "Idle"
  LoadState.failed => state.message
  LoadState.loaded => "Loaded"
}
```

## Iteration Expressions
`for` yields a new sequence by looping over an input sequence. Both item-only and item/index forms exist; the item comes first and the optional
second name is its zero-based index.

```nx
let cards = for user in users {
  <UserCard user={user}/>
}

let stripedRows = for item, index in items {
  <Row key={index} className={if (index % 2 == 0) { "even" } else { "odd" }}>
    {item.label}
  </Row>
}
```

You can nest `if` inside a `for` to implement filter-like projections.

```nx
let evens = for n in numbers {
  if n % 2 == 0 { n }
}
```

## Object and Element Creation
Because objects share the same syntax as components, you can inline structured data anywhere an expression is expected.

```nx
let payload = <User id="456" name="Grace" email="grace@example.com"/>

<UserCard user={payload}/>
```

## Error Handling and Guarding
Use helper expressions to provide safe fallbacks and intentional error states.

```nx
let avatarUrl = if user.avatarUrl {
  user.avatarUrl
} else {
  "/default-avatar.svg"
}
```

## See also
- Reference: [Types – Numeric Types and Conversions](/reference/syntax/types#numeric-types-and-conversions)
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [if](/reference/syntax/if), [for](/reference/syntax/for)
- Grammar: [nx-grammar.md – Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
