---
title: 'if'
description: 'Conditional logic in NX.'
---

NX uses `if` expressions for classic two-branch conditionals, match-style dispatch, and condition lists. Because every `if` is an expression, it can appear wherever a value is expected. See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions) for syntax.

## Basic Form

```nx
let greeting = if (isMorning) {
  "Good morning"
} else {
  "Hello there"
}
```

- Parentheses around the condition are optional but improve readability when expressions grow complex.
- Braces are mandatory, even for single statements, to avoid accidental fall-through.

## Inline Conditions
Use compact expressions for attributes and property assignments.

```nx
<Button className={if isPrimary { "btn btn-primary" } else { "btn" }}>
  Submit
</Button>
```

## Pattern Matching
`if <value> is { ... }` evaluates the arms in order and returns the first match. It is useful for union cases or simple value dispatch.

```nx
type ReviewStatus = pending_review | approved | rejected

let statusBadge = if status is {
  ReviewStatus.pending_review => <Badge: tone="info">Pending review</Badge>
  ReviewStatus.approved => <Badge: tone="success">Approved</Badge>
  ReviewStatus.rejected => <Badge: tone="danger">Rejected</Badge>
  else => <Badge: tone="neutral">Unknown</Badge>
}
```

- Multiple patterns can share a body: `"saturday", "sunday" => <WeekendIcon/>`.
- Constant-case and literal matches may omit `else`, but a missed match fails at runtime. Use `else` when a
  fallback is meaningful.
- Union matches without `else` must cover every case in the union.

### Discriminated Union Narrowing

When the scrutinee is a local identifier with a union type, each union case arm narrows that
identifier to the matched case for the arm body. NX does not introduce a separate `as` binding in
this version.

```nx
type LoadState =
  | idle
  | failed { message:string }
  | loaded { count:int }

let label(state: LoadState) = if state is {
  LoadState.idle => "Idle"
  LoadState.failed => state.message
  LoadState.loaded => "Loaded"
}
```

`state.message` is valid only in the `LoadState.failed` arm. Outside that arm, an unnarrowed
`LoadState` value exposes only fields shared through an abstract base record, if the union extends
one.

## Condition-list form

```nx
let banner = if {
  hasErrors => <Alert tone="danger">Fix errors</Alert>
  isLoading => <Alert tone="info">Loading…</Alert>
  else => null
}
```

- Arms are evaluated in order; the first true condition wins.
- `else` is optional but recommended for clarity.

## When there is no `else`

An `if` with no `else`, in any of the forms above, behaves as though its `else` were `{}` — the
empty sequence. So it produces the branch it takes, or nothing at all, and its type is a sequence
of what its branches produce:

```nx
let c = true
let tags:string[] = { if c { "new" } }

let root() = { tags }
```

`tags` holds one string when `c` is true and none when it is false. Because the missing branch is
an empty sequence rather than a null, the same conditional works wherever items are collected — an
element body, a content property, a braced sequence, the body of a `for` — and contributes nothing
there when no branch is taken:

```nx
type Item = { label:string }
type List = { content items:Item[] }

let showExtra = false

let root() = {
  <List>
    <Item label="always" />
    if showExtra { <Item label="sometimes" /> }
  </List>
}
```

The list holds one item when `showExtra` is false: the conditional never contributes a null item,
and never widens the element type the other children determine. The same rule is what makes
`for n in ns { if (n % 2 == 0) { n } }` a filter, and it holds at any depth — a conditional nested
in one is still a sequence of items, never a sequence of sequences.

An `if` written as a child needs no braces around it. `{if showExtra { ... }}` means the same thing,
but the bare form is preferred. A property value is different: there the braces are required, as in
`className={if ... }` above.

Where you want a nullable value instead, write the `else`:

```nx
let hasErrors = false
let hint:string? = { if hasErrors { "Fix the errors above" } else { null } }

let root() = { hint }
```

An `if` with no `else` is a sequence, not a nullable, so without the `else { null }` that binding is
rejected as `expects string?, found string[]`.

## Property-list form

Inside an element opening tag, `if` can select property groups instead of a single value.

```nx
<Button if primary { label="Save" } else { label="Cancel" } />

<Badge if {
  isError => tone="danger"
  isWarning => tone="warning"
  else => tone="neutral"
} />

<Notice if state is {
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
