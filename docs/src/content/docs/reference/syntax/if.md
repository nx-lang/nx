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
`if <value> is { ... }` evaluates the arms in order and returns the first match. It is useful for enums, discriminated unions, or simple value dispatch.

```nx
enum ReviewStatus = pending_review | approved | rejected

let statusBadge = if status is {
  ReviewStatus.pending_review => <Badge: tone="info">Pending review</Badge>
  ReviewStatus.approved => <Badge: tone="success">Approved</Badge>
  ReviewStatus.rejected => <Badge: tone="danger">Rejected</Badge>
  else => <Badge: tone="neutral">Unknown</Badge>
}
```

- Multiple patterns can share a body: `"saturday", "sunday" => <WeekendIcon/>`.
- Enum and literal matches may omit `else`, but a missed match fails at runtime. Use `else` when a
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

## Best Practices
- Keep cases small and consider extracting functions or components for large branches.
- Use explicit return types when inference becomes ambiguous, especially when mixing markup and scalar values.
- Prefer pattern matching over nested `if/else` chains when dispatching on known sets of values.

## See also
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Expressions/if](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
