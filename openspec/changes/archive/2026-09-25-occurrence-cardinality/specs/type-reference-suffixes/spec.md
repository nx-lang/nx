## REMOVED Requirements

### Requirement: Type references compose one sequence suffix with nullable suffixes
**Reason**: The two composing suffixes `[]` and `?` are replaced by one occurrence suffix with four
cardinalities. There is no longer a sequence of nullables or a nullable sequence to keep distinct,
so the ordering, same-layer and at-most-one-`[]` rules have nothing to govern.
**Migration**: `occurrence-types`, "A type reference carries at most one occurrence suffix".
`T[]` becomes `T*` or, where the sequence cannot be empty, `T+`; `T?` keeps its spelling with the
meaning zero-or-one; `T[]?` and `T?[]` become `T*`; `T??` and `T[][]` were already rejected and
stay rejected under the one-suffix rule. In a property's type slot a `?` or `*` moves to the name:
`queries:string[]?` becomes `queries?:string+`.

### Requirement: Explicit null values bind to nullable type references
**Reason**: `null` is removed from the language. The absent value is `{}`.
**Migration**: `occurrence-types`, "The empty sequence is the absent value", and
`optional-properties`. `initialExperience={null}` becomes `initialExperience={}` or is omitted;
`let none(): InitialExperience? = { null }` becomes `{ {} }` or `{}`; a `null` at a non-optional
site was an error and `{}` there remains one.
