---
title: 'Sequences & Object Duality'
description: 'How NX treats sequences as the core collection type and keeps objects in sync with components.'
---

Sequences are the primary collection type in NX. They underpin iteration, list rendering, and body
content. Three sentences describe all of it:

- **An item is a sequence of one.** Wherever a sequence is expected, a single item will do.
- **A sequence never contains a sequence.** `T+` and `T*` are defined for an exactly-one item type
  `T`, and no type that carries an occurrence is an item type.
- **An item in a collecting position contributes its items.** A sequence written where items are
  being collected contributes its items in order, not itself.

See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions) for formal
rules, and [Occurrences](/reference/syntax/types#occurrences) for the four cardinalities `T`, `T?`,
`T+` and `T*`.

## Writing a sequence

A sequence is written with braces, with no delimiter between items, exactly as elements are written
side by side in markup. `{}` is the empty sequence, and a `for` yields one. A sequence that cannot
be empty is a `T+`; one that can is a `T*`.

```nx
let numbers:int+ = { 1 2 3 4 5 }
let names:string+ = { "Alice" "Bob" "Carol" }
let none:string* = { }

let squares:int+ = { for n in numbers { n * n } }
let evens:int* = { for n in numbers { if (n % 2 == 0) { n } } }

let root() = { squares }
```

The last two are worth reading twice. A `for` concatenates what its body yields, so a body that
yields one item per iteration gives one item per iteration, and a body that yields nothing on an
iteration contributes nothing at all. An `if` with no `else` is read as having an `else { }`, so
when it is not taken it yields the empty sequence — which is nothing. That is the filter idiom, and
it needs no operator of its own. It is also why `evens` is an `int*` where `squares` is an `int+`:
a `for` multiplies the occurrence of what it iterates by the occurrence of its body, and a body
that may yield nothing may leave the whole result empty.

## The empty sequence is the absent value

`{}` is the one way to write "no value" in NX; there is no `null`. An optional property that was
not written, an `if` that took no branch and a `for` that yielded nothing all produce the same
empty sequence, and it satisfies every `?` and `*` site. A value that may be empty is used as one
value through the presence test `x?`, the step `x?.m` and the fallback `x ?? y`, described under
[Values that may be empty](/reference/syntax/expressions#values-that-may-be-empty).

```nx
type Person = { name:string }
type Book = { title:string author?:Person }

let b = <Book title="B" />                          // author is {}
let name(b:Book): string = { b.author?.name ?? "anonymous" }

let root() = { name(b) }
```

## Items splice

Where items are collected — a braced sequence, a call argument, body content, the yields of a `for`
— an item whose own type is a sequence contributes its items.

```nx
let xs:string+ = { "a" "b" }
let ys:string+ = { "c" }

// One sequence of three strings, not a sequence of two sequences.
let all:string+ = { xs ys }
// The rule does not care whether the neighbours are sequences or items.
let more:string+ = { xs "d" }

let root() = { all }
```

Body content follows the same rule, which is what lets a helper that returns several children sit
beside a single one:

```nx
type Badge = { n:int = 1 }
type Row = { content items:Badge+ }

let some:Badge+ = { <Badge/> <Badge n=2 /> }

let root() = { <Row>{some}<Badge n=3 /></Row> }
```

`items` holds three badges. The same three arrive through a property binding, `<Row items={some
<Badge n=3 />} />`, because a property is a collecting position too.

A conditional child that does not fire contributes nothing, for the same reason — its missing
`else` is an empty sequence:

```nx
type Item = { label:string }
type List = { content items:Item+ }

let showExtra = false

let root() = {
  <List>
    <Item label="always" />
    if showExtra { <Item label="sometimes" /> }
  </List>
}
```

`items` holds one item. An untaken conditional contributes no items — never an empty item, and
never a widening of the element type. It is not a special case: the conditional's value may be
empty, so it splices like any other item that may be, alone or beside others. As a child it needs no
braces of its own, though `{if showExtra { ... }}` means the same thing. The same holds in a
position that supplies one value: `{if showExtra { "yes" }}` has type `string?`, one string or
none.

The occurrence of what is collected is the sum of its items' occurrences. `{ <Item/> if c { <Item/> } }`
is an `Item+`, because the first item is always there; `{ if c { <Item/> } if d { <Item/> } }` is an
`Item*`, because both may be missing. A body that is only an untaken conditional is therefore an
`Item?`, and binds to `content items:Item+` only when it is written `items?:Item+`.

## One suffix

A type carries at most one occurrence suffix, so there is no optional sequence and no sequence of
optionals. Neither is needed: a sequence with no items *is* the absent value, and `T*` says both
"may be empty" and "may be many". A property that may be left out takes the `?` mark on its name
rather than in its type:

```nx
type SearchState = {
  queries?:string+
  aliases:string+
}

let root() = { <SearchState queries={ } aliases={ "ali" } /> }
```

`queries` reads as a `string*` and `aliases` as a `string+`. A second suffix is rejected wherever it
is written: `string??`, `string?+` and `(string+)*` each fail because the type already carries an
occurrence. The rejection follows an alias too — given `type Names = string+`, `Names*` is not a
type — and a type argument must be an exactly-one type, so `<Page T=int+/>` is rejected where
`<Page T=int/>` is fine.

## Nesting data

A record is how data nests. Where a matrix or a bucket list is wanted, name the row:

```nx
type Row = { cells:int+ }

let grid:Row+ = { <Row cells={ 1 2 } /> <Row cells={ 3 4 } /> }

let root() = { grid }
```

The row type is a name worth having: `Row` says what a row is, where a nested sequence of `int`
would say only that something is nested. Iterating `grid` gives rows, and iterating a row gives
cells.

There is one place the static and dynamic views differ. A value of type `object` may hold anything,
including a sequence, and it is opaque: it counts as one item where it is typed as one, and it
contributes its own items where it is collected. Because the empty sequence is also the absent
value, an empty sequence held by a value of type `object?` reads as absent; `object?` is the one
type at which a held sequence and absence meet. Data arriving from outside NX with genuinely
nested arrays has no NX type to land in today; a record around each level is the answer, and a
distinct array type for external data — as XPath added one for JSON — is the likely answer later.

## Objects and Components Share Syntax
NX reuses element syntax for record definitions and construction so that data and UI stay aligned.

```nx
type User = { id:string name:string email:string avatarUrl?:string }
type Point = { x:int y:int }
type Color = { r:int g:int b:int a:float64 = 1.0 }

let user =
  <User
    id="123"
    name="John Doe"
    email="john@example.com"
    avatarUrl="/avatars/john.jpg"
  />

let origin = <Point x=0 y=0 />
let red = <Color r=255 g=0 b=0 />
let transparentBlue = <Color r=0 g=0 b=255 a=0.5 />

let root() = { user }
```

Because the syntax aligns, assembling records from components (and vice versa) feels natural, and a
sequence of records is written the way a sequence of children is:

```nx
type User = { id:string name:string email:string }

let users:User+ = {
  <User id="1" name="Alice" email="alice@example.com" />
  <User id="2" name="Bob" email="bob@example.com" />
  <User id="3" name="Carol" email="carol@example.com" />
}

let root() = { users }
```

This duality simplifies data modelling, component authoring, and tooling: the same grammar powers
both structures.

## See also
- Language Tour: [Types](/language-tour/types)
- Reference: [Expressions](/reference/syntax/expressions), [Types](/reference/syntax/types)
- Grammar: [nx-grammar.md – Types/Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
