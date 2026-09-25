---
title: 'if'
description: 'Conditional logic in NX.'
---

NX uses `if` expressions for classic two-branch conditionals, match-style dispatch, and condition lists. Because every `if` is an expression, it can appear wherever a value is expected. See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions) for syntax.

## Basic Form

```nx
let greeting(isMorning:boolean) = {
  if (isMorning) {
    "Good morning"
  } else {
    "Hello there"
  }
}
```

- Parentheses around the condition are optional but improve readability when expressions grow complex.
- Braces are mandatory, even for single statements, to avoid accidental fall-through.
- There is no `c ? a : b` operator: `if c { a } else { b }` is the expression form. A `?` written
  after a value is the [presence test](#presence-tests), and a `:` after that is a syntax error
  whose diagnostic shows the `if` to write.

## Inline Conditions
Use compact expressions for attributes and property assignments.

```nx
let submit(isPrimary:boolean) = {
  <Button className={if isPrimary { "btn btn-primary" } else { "btn" }}>
    Submit
  </Button>
}
```

## Pattern Matching
`if <value> is { ... }` evaluates the arms in order and returns the first match. It is useful for union cases or simple value dispatch.

```nx
type ReviewStatus = pending_review | approved | rejected
type Badge = { tone:string content text:string }

let statusBadge(status:ReviewStatus) = {
  if status is {
    ReviewStatus.pending_review => <Badge tone="info">Pending review</Badge>
    ReviewStatus.approved => <Badge tone="success">Approved</Badge>
    ReviewStatus.rejected => <Badge tone="danger">Rejected</Badge>
    else => <Badge tone="neutral">Unknown</Badge>
  }
}
```

- Multiple patterns can share a body: `"saturday", "sunday" => <WeekendIcon/>`.
- Constant-case and literal matches may omit `else`; a match with no arm taken is the empty value,
  as [described below](#when-there-is-no-else). Use `else` when a fallback is meaningful.
- Union matches without `else` must cover every case in the union.
- `{}` is a pattern that matches the empty value; see [Matching the empty value](#matching-the-empty-value).

### Discriminated Union Narrowing

When the scrutinee is a local identifier with a union type, each union case arm narrows that
identifier to the matched case for the arm body. NX does not introduce a separate `as` binding in
this version.

```nx
type LoadState =
  | idle
  | failed { message:string }
  | loaded { count:int }

let label(state: LoadState) = {
  if state is {
    LoadState.idle => "Idle"
    LoadState.failed => state.message
    LoadState.loaded => "Loaded"
  }
}
```

`state.message` is valid only in the `LoadState.failed` arm. Outside that arm, an unnarrowed
`LoadState` value exposes only fields shared through an abstract base record, if the union extends
one.

### Matching the empty value

A scrutinee whose type is `T?` or `T*` may be the empty value `{}`, and `{}` is a pattern that
matches exactly that. The arms after it — including `else` — read a path scrutinee as present, the
way a [presence test](#presence-tests) does, so a plain `.` works there. Over a `?`-typed union,
covering `{}` and every case makes the match exhaustive without an `else`.

```nx
type Person = { name:string }
type Book = { title:string author?:Person }
type State = idle | busy

let credit(b:Book): string = {
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

A `{}` arm on a scrutinee that cannot be empty is reported, since it never matches.

## Condition-list form

```nx
let banner(hasErrors:boolean, isLoading:boolean) = {
  if {
    hasErrors => <Alert tone="danger">Fix errors</Alert>
    isLoading => <Alert tone="info">Loading…</Alert>
    else => {}
  }
}
```

- Arms are evaluated in order; the first true condition wins.
- `else` is optional. Writing `else => {}` and leaving it out mean the same thing: the empty value
  when no condition holds.

## When there is no `else`

An `if` with no `else`, in any of the forms above, behaves as though its `else` were `{}` — the
empty value. So it produces the branch it takes, or nothing at all, and its type is its branches'
type with zero admitted: `T?` when the branches produce a `T`, `T*` when they produce a `T+`.

```nx
let c = true
let tag:string? = { if c { "new" } }

let root() = { tag }
```

`tag` holds one string when `c` is true and is empty when it is false. That is exactly what an
optional site expects, so no `else` is needed to bind one:

```nx
let hasErrors = false
let hint:string? = { if hasErrors { "Fix the errors above" } }

let root() = { hint }
```

The same `if` at an exactly-one site is rejected as `expects string, found string?`; give it an
`else`, or a fallback with `??`:

```nx
// Rejected: the if may be empty, and `label` cannot be.
let hasErrors = false
let label:string = { if hasErrors { "Fix the errors above" } }
```

Because the missing branch is the empty value, the same conditional works wherever items are
collected — an element body, a content property, a braced sequence, the body of a `for` — and
contributes nothing there when no branch is taken:

```nx
type Item = { label:string }
type List = { content items:Item+ }

let showExtra = false

let root() = {
  <List>
    <Item label="always" />
    if showExtra { <Item label="sometimes" /> }
  </List>
}
```

The list holds one item when `showExtra` is false: the conditional never contributes an empty item,
and never widens the item type the other children determine. The body's occurrence is still `+`,
because the first child is always there. A body that is *only* an untaken conditional is `Item?`,
which a `content items:Item+` property rejects; declare it `items?:Item+` when the list may be
empty. The same rule is what makes `for n in ns { if (n % 2 == 0) { n } }` a filter, and it holds
at any depth — a conditional nested in one is still a sequence of items, never a sequence of
sequences.

An `if` written as a child needs no braces around it. `{if showExtra { ... }}` means the same thing,
but the bare form is preferred. A property value is different: there the braces are required, as in
`className={if ... }` above.

## Presence tests

A value of type `T?` or `T*` may be empty, and the postfix `?` asks: `x?` is `true` when `x` holds
an item. Used as the condition of an `if`, it also *narrows*: inside the branch the test proves,
the tested path reads as present — `T?` as `T`, `T*` as `T+` — so a plain `.` and an exactly-one
site both accept it there.

```nx
type Person = { name:string }
type Book = { title:string author?:Person tags?:string+ }

let byline(b:Book): string = { if b.author? { "by " + b.author.name } else { "anonymous" } }
let tagList(b:Book): string+ = { if b.tags? { b.tags } else { "untagged" } }
```

`!p?` narrows the `else` branch, and `&&` narrows every operand that is a test. A `||` narrows
nothing, and no narrowing outlives its branch:

```nx
let orNone(o?:string): string = { if !o? { "none" } else { o } }
let joined(a?:string, b?:string): string = { if a? && b? { a + b } else { "" } }
```

Testing a chain narrows each receiver along it, so `?.` in the test becomes `.` in the branch:

```nx
type Name = { first:string }
type Person = { name?:Name }
type Book = { author?:Person }

let first(b:Book): string = { if b.author?.name? { b.author.name.first } else { "" } }
```

When the branch only needs a default, `??` is shorter: `b.author?.name ?? "anonymous"`. See
[Values that may be empty](/reference/syntax/expressions#values-that-may-be-empty) for the three
operators.

## Property-list form

Inside an element opening tag, `if` can select property groups instead of a single value.

```nx
external component <Button label:string />
external component <Badge tone:string />
external component <Notice message:string />
type LoadState = | idle | failed { message:string }

let save(primary:boolean) = <Button if primary { label="Save" } else { label="Cancel" } />

let badge(isError:boolean, isWarning:boolean) = <Badge if {
  isError => tone="danger"
  isWarning => tone="warning"
  else => tone="neutral"
} />

let notice(state:LoadState) = <Notice if state is {
  LoadState.failed => message={state.message}
  else => message=""
} />
```

Property-list matches share the same union narrowing and exhaustiveness checks as value matches.
Required properties must be present on every reachable branch, and duplicate properties are
rejected only when the duplicates can occur on the same execution path.

## Best Practices
- Keep cases small and consider extracting functions or components for large branches.
- Use explicit return types when inference becomes ambiguous, especially when mixing markup and scalar values.
- Prefer pattern matching over nested `if/else` chains when dispatching on known sets of values.

## See also
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Expressions/if](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
