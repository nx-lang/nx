## ADDED Requirements

### Requirement: A function type is scoped in every type position
In any type position — a signature property, a record property, a function parameter, a return
annotation, a value definition, or a type alias — the grammar SHALL recognize a function type and
SHALL scope the `function` keyword `keyword.other.function.nx`, its `<` and `/>` as the same
punctuation as an element-shaped declaration signature, each parameter by the same rules as a
signature property (`variable.other.property.nx`, the annotation colon, the type), the `:`
before the result `punctuation.separator.type.annotation.nx`, and the result type by the shared
type rule. Parentheses around a type SHALL be scoped `punctuation.definition.type.group.nx`, and
a `?` or `[]` after the closing parenthesis SHALL be scoped `keyword.operator.type-modifier.nx`.
Recognition SHALL NOT depend on the function type fitting on one line. The word `function`
outside a type position SHALL NOT be scoped as a keyword.

#### Scenario: A function-typed property is scoped
- **WHEN** a signature contains `ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`
- **THEN** `function` SHALL be scoped `keyword.other.function.nx`
- **AND** `Item` and `Index` SHALL be scoped `variable.other.property.nx`
- **AND** `TItem`, `int` and `DrawnNode` SHALL be scoped as types by the shared rule
- **AND** the trailing `?` SHALL be scoped `keyword.operator.type-modifier.nx`

#### Scenario: A function type alias is scoped
- **WHEN** a file contains `type RowTemplate = <function Item:Contact Index:int />: DrawnNode`
- **THEN** `RowTemplate` SHALL be scoped `entity.name.type.nx`
- **AND** the right-hand side SHALL be scoped as a function type

#### Scenario: function as an identifier is not a keyword
- **WHEN** a file contains `let function = 1`
- **THEN** `function` SHALL NOT be scoped `keyword.other.function.nx`
