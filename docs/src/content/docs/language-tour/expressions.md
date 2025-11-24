---
title: 'Expressions & Control Flow'
description: 'All constructs produce values: conditionals, loops, calls, and operators.'
---

NX treats everything as an expression, so you can compose logic inline without switching syntaxes.

## Conditionals

```nx
let statusBadge = if isActive {
  <Badge tone="success">Active</Badge>
} else {
  <Badge tone="neutral">Inactive</Badge>
}
```

### Match-style conditions

```nx
let badge = if plan is {
  "free" => <Badge tone="neutral">Free</Badge>
  "pro" => <Badge tone="success">Pro</Badge>
  else => <Badge tone="info">Enterprise</Badge>
}
```

The first matching arm wins. Omit `else` only when you are sure all cases are covered.

### Condition-list form

```nx
let banner = if {
  hasErrors => <Alert tone="danger">Fix errors</Alert>
  isLoading => <Alert tone="info">Loading…</Alert>
  else => null
}
```

## Loops (`for`)

```nx
let items = for index, user in users {
  <li key={index}>{user.name}</li>
}
```

`for` yields a sequence; include the index when you need position.

## Calls and operators

```nx
let total = add(2, 3) * 4
let hasAccess = user.role == "admin" || user.role == "editor"
```

Use standard arithmetic/comparison/logical operators; see the Reference for precedence.

## See also (Reference/Grammar)
- Reference: [if](/reference/syntax/if)
- Reference: [for](/reference/syntax/for)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
