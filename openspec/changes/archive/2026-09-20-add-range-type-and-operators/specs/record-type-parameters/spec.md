## MODIFIED Requirements

### Requirement: Applied types are distinct per argument and invariant
Two applied types SHALL be the same type exactly when they apply the same record declaration and
bind the same type to every parameter, after aliases are resolved. An applied type SHALL satisfy
an expected applied type only when the two are the same type: a type argument SHALL NOT widen,
narrow, or convert, even where the argument types themselves convert implicitly. Every applied
type SHALL satisfy `object`. Reading a field of a value of an applied type SHALL give the field's
declared type with each parameter replaced by its argument. Invariance SHALL decide the common type
of two applied types as it decides assignment: two of one instantiation SHALL have that
instantiation as their common type, arguments and all, and two that are not the same type SHALL
have `object`. No inferred type SHALL ever be a generic record's bare name, which is not a type a
type position accepts.

#### Scenario: Different arguments are different types
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let ints:<Range T=int/> = <Range T=int start={1} end={5} />` and `let bad:<Range T=string/> = ints`
- **THEN** analysis SHALL reject `bad` with a type mismatch that spells both applied types

#### Scenario: Arguments do not convert
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let ints:<Range T=int/> = <Range T=int start={1} end={5} />` and `let bad:<Range T=float64/> = ints`
- **THEN** analysis SHALL reject `bad` even though `int` converts implicitly to `float64`

#### Scenario: An alias argument is the same type as its target
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type Count = int` and `let a:<Range T=Count/> = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Field access substitutes the argument
- **WHEN** a file contains `type Page = { T:type items:T[] next:T? }` and `let p:<Page T=string/> = <Page T=string items={ "a" } />` and `let first:string[] = p.items` and `let bad:int? = p.next`
- **THEN** analysis SHALL accept `first`
- **AND** SHALL reject `bad` because `p.next` is `string?`

#### Scenario: An applied type satisfies object
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let o:object = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: A list of one instantiation keeps its arguments
- **WHEN** a file contains `type Box = { T:type value:T }` and `let a = <Box T=int value={1} />` and `let b = <Box T=int value={2} />` and `let bad:int = { a b }`
- **THEN** analysis SHALL reject `bad` with a mismatch naming a list of `<Box T=int/>`

#### Scenario: A list of two instantiations is a list of object
- **WHEN** a file contains `type Box = { T:type value:T }` and `let a = <Box T=int value={1} />` and `let b = <Box T=string value="x" />` and `let bad:int = { a b }` and `let mixed:object[] = { a b }`
- **THEN** analysis SHALL reject `bad` with a mismatch naming a list of `object`, never a list of a bare `Box`
- **AND** SHALL accept `mixed`
