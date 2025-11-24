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
let statusBadge = if status is {
  "pending"  => <Badge: tone="info">Pending</Badge>
  "approved" => <Badge: tone="success">Approved</Badge>
  "rejected" => <Badge: tone="danger">Rejected</Badge>
  else       => <Badge: tone="neutral">Unknown</Badge>
}
```

- Multiple patterns can share a body: `"saturday", "sunday" => <WeekendIcon/>`.
- Omitting `else` is allowed. If no case matches, evaluation fails to surface incorrect assumptions.

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
