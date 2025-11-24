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

## See also
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [if](/reference/syntax/if), [for](/reference/syntax/for)
- Grammar: [nx-grammar.md – Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions)
