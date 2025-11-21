---
title: 'Expressions'
description: 'Expression syntax and evaluation rules in NX.'
---

Every construct in NX is an expression that yields a value. That includes conditionals, loops, object creation, and control-flow helpers. The examples below illustrate the most common forms and how they compose.

## Literals and Basic Forms
- Numbers, strings, booleans, and sequences use familiar literal syntax.
- Object creation reuses element syntax so data looks like the components that consume it.

```nx
let numbers = [1, 2, 3, 4, 5]
let user = <User id="123" name="Ada"/>
let empty: string[] = []
```

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
NX extends `if` to support match-style pattern blocks. Each arm is evaluated in order and the first match wins.

```nx
let dashboard = if user.role is {
  "admin" => <AdminPanel/>
  "member" => <MemberHome/>
  else => <ReadOnlyView/>
}
```

- Arms never fall through; each case is independent.
- Multiple patterns can share a result: `"saturday", "sunday" => <Weekend/>`.
- Omitting `else` is allowed but failing to match will raise an error at runtime, so reserve it for exhaustive sets.

## Iteration Expressions
`for` yields a new sequence by looping over an input sequence. Both value-only and index/value forms exist.

```nx
let cards = for user in users {
  <UserCard user={user}/>
}

let stripedRows = for index, item in items {
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

Future iterations of the language will expand the expression set with advanced pattern matching, async orchestration, and exhaustive exhaustiveness checking, but the core principle remains: everything evaluates to a value and composes predictably.
