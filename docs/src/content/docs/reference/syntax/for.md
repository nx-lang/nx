---
title: 'for'
description: 'Iteration patterns available in NX.'
---

`for` is NX’s primary way to transform sequences. It produces a new sequence by evaluating the body for each item in the input.

## Value Form

```nx
let gallery = for image in images {
  <img src={image.url} alt={image.title}/>
}
```

- `for` returns a sequence, so `gallery` above is the list of `<img/>` elements.
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

## Performance Notes
- Sequences may be lazy in future runtimes, so avoid side effects in loop bodies.
- Prefer computing derived data once and passing it into the loop instead of recomputing in each iteration.
- When you only need aggregation (folding into a single value), consider helper functions from the standard library once available.
