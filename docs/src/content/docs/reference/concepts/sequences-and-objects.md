---
title: 'Sequences & Object Duality'
description: 'How NX treats sequences as the core collection type and keeps objects in sync with components.'
---

Sequences are the primary collection type in NX. They underpin iteration, comprehensions, and list rendering. See [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#expressions) for formal rules.

## Working with Sequences

```nx
let numbers: int[] = [1, 2, 3, 4, 5]
let names: string[] = ["Alice", "Bob", "Carol"]
let users: User[] = [user1, user2, user3]

let empty: string[] = []

let squares = for n in numbers { n * n }
let evens = for n in numbers { if (n % 2 == 0) { n } }
```

- Sequences may be eager or lazy depending on the host runtime; syntax is unchanged.
- Nested sequences are straightforward: `int[][]` represents a matrix, and `(string, User[])[]` models grouped buckets.

```nx
let matrix: int[][] = [[1, 2], [3, 4], [5, 6]]
let grouped: (string, User[])[] = [
  ("admins", [admin1, admin2]),
  ("users", [user1, user2, user3])
]
```

## Objects and Components Share Syntax
NX reuses element syntax for object definitions and instantiation so that data and UI stay aligned.

```nx
type <User id:string name:string email:string avatarUrl:string?/>
type <Point x:int y:int/>
type <Color r:int g:int b:int a:float = 1.0/>

let user =
  <User
    id="123"
    name="John Doe"
    email="john@example.com"
    avatarUrl="/avatars/john.jpg"
  />

let origin = <Point x=0 y=0/>
let red = <Color r=255 g=0 b=0/>
let transparentBlue = <Color r=0 g=0 b=255 a=0.5/>
```

Because the syntax aligns, assembling objects from components (and vice versa) feels natural.

```nx
let <UserProfile userId:string/> = {
  let user = <User
    id={userId}
    name="John Doe"
    email="john@example.com"
  />

  <div>
    <img src={if user.avatarUrl { user.avatarUrl } else { "/default-avatar.jpg" }}/>
    <h2>{user.name}</h2>
    <span>{user.email}</span>
  </div>
}
```

Inline object usage works the same way.

```nx
<UserCard user={<User id="456" name="Jane" email="jane@example.com"/>}/>

let users = [
  <User id="1" name="Alice" email="alice@example.com"/>,
  <User id="2" name="Bob" email="bob@example.com"/>,
  <User id="3" name="Carol" email="carol@example.com"/>
]

type <StringContainer value:string metadata:string created:string/>

let stringContainer = <StringContainer
  value="hello world"
  metadata="text data"
  created="2023-01-01"
/>
```

This duality simplifies data modelling, component authoring, and tooling: the same grammar powers both structures.

## See also
- Language Tour: [Types](/language-tour/types)
- Reference: [Expressions](/reference/syntax/expressions), [Types](/reference/syntax/types)
- Grammar: [nx-grammar.md – Types/Expressions](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#types)
