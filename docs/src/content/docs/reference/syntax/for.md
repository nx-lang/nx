---
title: 'for'
description: 'Iteration patterns available in NX.'
---

`for` transforms a source sequence into a new sequence by evaluating the body per element. See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements) for formal syntax.

## Value Form

```nx
let gallery = for image in images {
  <img src={image.url} alt={image.title}/>
}
```

- `for` returns a sequence.
- The loop variable is scoped to the body.

## Index + Value Form

```nx
let stripedRows = for index, row in rows {
  <tr className={if (index % 2 == 0) { "even" } else { "odd" }}>
    <td>{row.label}</td>
  </tr>
}
```

- The first identifier receives the zero-based index.
- Use this form when you need stable keys or different styling per position.

## Filtering While Iterating
Because `if` is also an expression, you can yield optional values inside the loop. Returning nothing from a branch omits that item.

```nx
let adults = for person in people {
  if person.age >= 18 { person }
}
```

## Nested Loops
Compose loops by nesting `for` expressions without reaching for imperative constructs.

```nx
let cells = for row in grid {
  for column in row {
    <Cell value={column}/>
  }
}
```

## See also
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [if](/reference/syntax/if)
- Grammar: [nx-grammar.md – Elements/for](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
