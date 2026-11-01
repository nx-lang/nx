## MODIFIED Requirements

### Requirement: Canonical encoding of an occurrence-typed value
The canonical value encoding SHALL encode a `+` or `*` value as an array, including the empty array
for an empty `*`. It SHALL encode a `?` value that holds an item as that item. It SHALL encode an
empty `?` value by omitting the key when the value is a record field, and as `null` when it is the
result of an entry call — a host evaluating a function whose result type, declared or inferred, is
a standalone `T?` — since `null` is how hosts spell an absent single value, and typegen maps a
standalone `T?` to `T | null` in TypeScript and a nullable `T` in C#. Elsewhere an empty value
encodes as the empty array; inside the program the empty value is always the empty array, and how
a runtime represents it is its own choice. Decoding host input at a `?` or `*` site SHALL accept a missing key, `null` and an empty
array as the empty value, SHALL accept a one-element array at a `?` site as its element, and SHALL
reject a longer array at a `?` site. Decoding at a `+` site SHALL reject a missing key, `null` and an
empty array. Decoding at an exactly-one site SHALL reject `null`. First-party NX-syntax output SHALL
follow `unbraced-literal-forms`: an empty optional attribute is omitted, an empty top-level value is
`{}`.

#### Scenario: An entry call's empty optional result is null
- **WHEN** a host evaluates `let root(): string? = { if false { "a" } }`, `let root() = { if false { 1 } }` and `let root(): int* = { if false { 1 } }`
- **THEN** the first two SHALL return `null`
- **AND** the third SHALL return `[]`
- **AND** a .NET host deserializing the first result into a nullable `string` SHALL read `null`

#### Scenario: A present optional encodes as its item and an absent one as a missing key
- **WHEN** a file contains `type Book = { title:string author?:string }` and `let a = <Book title="A" author="X" />`, `let b = <Book title="B" />`
- **THEN** the canonical JSON of `a` SHALL contain `"author": "X"`
- **AND** the canonical JSON of `b` SHALL NOT contain an `author` key

#### Scenario: Host null and an empty array both decode to the empty value
- **WHEN** a host constructs `Book` from JSON `{ "title": "A", "author": null }` and from `{ "title": "A", "author": [] }`
- **THEN** both SHALL construct with `author` empty
- **AND** both SHALL be equal to `<Book title="A" />`

#### Scenario: An empty array is rejected where at least one is required
- **WHEN** a host constructs `type Box = { items:string+ }` from JSON `{ "items": [] }`
- **THEN** construction SHALL fail with a runtime error naming `items`
