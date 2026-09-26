---
title: 'Expressions & Control Flow'
description: 'All constructs produce values: conditionals, loops, calls, and operators.'
---

NX treats everything as an expression, so you can compose logic inline without switching syntaxes.

## Conditionals

```nx
type Badge = { tone:string content text:string }

let <StatusBadge isActive:boolean /> = {
  if isActive {
    <Badge tone="success">Active</Badge>
  } else {
    <Badge tone="neutral">Inactive</Badge>
  }
}
```

There is no `c ? a : b` operator; `if c { a } else { b }` is the expression form. An `if` with no
`else` is fine too: it is empty when the condition is false, so its type gains a `?` —
`if isActive { "on" }` is a `string?`.

### Match-style conditions

```nx
type DealStage = draft | pending_review | approved
type Badge = { tone:string content text:string }

let <StageBadge stage:DealStage /> = {
  if stage is {
    DealStage.draft => <Badge tone="neutral">Draft</Badge>
    DealStage.pending_review => <Badge tone="warning">Pending review</Badge>
    else => <Badge tone="success">Approved</Badge>
  }
}
```

The first matching arm wins. Cases without payloads are useful for scalar choices; cases with them add
case-specific payloads and narrow the matched identifier inside each arm.

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

For union-typed scrutinees, omit `else` only when every union case is covered. Add `else` when you
intentionally want a fallback for the remaining cases.

### Condition-list form

```nx
type Alert = { tone:string content text:string }

let <StatusBanner hasErrors:boolean isLoading:boolean /> = {
  if {
    hasErrors => <Alert tone="danger">Fix errors</Alert>
    isLoading => <Alert tone="info">Loading…</Alert>
  }
}
```

With no `else`, `<StatusBanner>` is empty when neither condition holds.

## Values that may be empty

An optional field, `author?:Person`, reads as a `Person?`: a person or the empty value `{}`. So
does an `if` without an `else`. Such a value is not a `Person` until you say what to do when it is
empty, and three operators do that:

```nx
type Person = { name:string }
type Book = { title:string author?:Person }

let hasAuthor(b:Book) = { b.author? }                          // true when present
let authorName(b:Book) = { b.author?.name }                    // string?: empty when the author is
let byline(b:Book) = { "by " + b.author?.name ?? "anonymous" } // string: the fallback fills the gap
```

- `x?` tests presence and is a `boolean`.
- `x?.m` reads a member through a value that may be empty, and is empty when the value is. A plain
  `b.author.name` is an error here; the message shows the `?.` to write.
- `x ?? y` is `x` when present and `y` otherwise. It binds tighter than `+`, so `byline` reads as
  `"by " + (b.author?.name ?? "anonymous")` — the opposite of C#, where `??` would take the whole
  concatenation as its left side.

Inside an `if` whose condition is a presence test, the tested value reads as present, so the plain
forms work there:

```nx
type Person = { name:string }
type Book = { title:string author?:Person }

let credit(b:Book) = {
  if b.author? { "by " + b.author.name } else { "anonymous" }
}

let describe(b:Book) = {
  if b.author is {
    {} => "anonymous"
    else => b.author.name
  }
}
```

`{}` is a pattern that matches the empty value, and the arms after it read the value as present.
See [Values that may be empty](/reference/syntax/expressions#values-that-may-be-empty) in the
reference for the full rules.

## Loops (`for`)

```nx
type User = { name:string }

let <UserItems users:User+ /> = {
  for user, index in users {
    <li key={index}>{user.name}</li>
  }
}
```

`for` yields a sequence. Add a second name after the item, as in `for user, index`, when you need
its zero-based position. A body that may yield nothing, such as an `if` with no `else`, is how a
`for` filters: the iterations it skips contribute no items.

## Calls and operators

```nx
type User = { role:string }

let add(a:int, b:int): int = { a + b }
let total = { add(2, 3) * 4 }
let hasAccess(user:User) = { user.role == "admin" || user.role == "editor" }
```

Use standard arithmetic/comparison/logical operators; see the Reference for precedence.

## See also (Reference/Grammar)
- Reference: [if](/reference/syntax/if)
- Reference: [for](/reference/syntax/for)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
