---
title: 'for'
description: 'Iteration patterns available in NX.'
---

`for` transforms a source sequence into a new sequence by evaluating the body per element. See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements) for formal syntax.

## Value Form

```nx
type Image = { url:string title:string }

let gallery(images:Image+) = {
  for image in images {
    <img src={image.url} alt={image.title}/>
  }
}
```

- `for` returns a sequence.
- The loop variable is scoped to the body.

## Item + Index Form

```nx
type Item = { label:string }

let stripedRows(rows:Item+) = {
  for row, index in rows {
    <tr className={if (index % 2 == 0) { "even" } else { "odd" }}>
      <td>{row.label}</td>
    </tr>
  }
}
```

- The item comes first; the optional second identifier receives its zero-based index.
- Use this form when you need stable keys or different styling per position.

## Counting with a range

`for` also iterates a **range**, so counting needs no list:

```nx
let squares = { for i in 0..4 { i * i } }        // 0 1 4 9
let pages = { for page in 1..=3 { page } }       // 1 2 3
let <Stars count:int /> = { for i in 0..count { <Star /> } }
```

- `a..b` counts from `a` up to but **excluding** `b`; `a..=b` includes `b`. Both build the built-in
  [`Range`](/reference/syntax/types#ranges) type.
- The item is each integer in turn, at the range's own integer type, and the optional index counts
  from zero as it does over a list.
- An empty or reversed range yields nothing: `0..0`, `5..2` and `1..=0` each produce no items.
- Only a range of an integer type — `int`, `int32` or `int64` — iterates. `for x in 0..2.5` is a type
  error rather than a guessed step.
- The range is never turned into a list first, so `for i in 0..n` costs one body evaluation per item
  and nothing else.

A range value from anywhere iterates, not only one written with the operators:

```nx
let span:<Range T=int/> = <Range T=int start={2} end={4} endInclusive={true} />
let items = { for i in span { i } }              // 2 3 4
```

## Filtering While Iterating
Because `if` is also an expression, you can yield optional values inside the loop. Returning nothing from a branch omits that item.

```nx
type Person = { name:string age:int }

let adults(people:Person+) = {
  for person in people {
    if person.age >= 18 { person }
  }
}
```

## Nested Loops
Compose loops by nesting `for` expressions without reaching for imperative constructs. A sequence
never holds another sequence, so a grid is a sequence of records that each hold a row's cells:

```nx
type GridRow = { cells:int+ }

let cells(grid:GridRow+) = {
  for row in grid {
    for cell in row.cells {
      <Cell value={cell}/>
    }
  }
}
```

The result is flat: each inner `for` contributes its items to the outer one's sequence.

## See also
- Language Tour: [Expressions & Control Flow](/language-tour/expressions)
- Reference: [if](/reference/syntax/if)
- Grammar: [nx-grammar.md – Elements/for](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
