## ADDED Requirements

### Requirement: An applied type names one instantiation of a generic record with item-type arguments
Every type position SHALL accept an applied type, written as a self-closing element whose tag is
the name of a generic record and whose properties are `Name=Type` type arguments:
`<Range T=int/>`. The tag MAY be a qualified name. A type argument SHALL be any item type — a
primitive, a record, a union, an alias, a function type, a type parameter in scope, a nullable
type, or another applied type. A type argument SHALL NOT be a sequence type, whether spelled with
`[]` or reached through an alias, and the system SHALL reject one with a diagnostic explaining that
a type argument must not be a sequence. An applied type SHALL compose with the `?` and `[]` suffixes
in source order like any other type reference, SHALL be usable as the target of a type alias, and
SHALL be usable as a field type inside the generic record it applies. Type arguments MAY be written
in any order and SHALL be matched to parameters by name.

#### Scenario: An applied type annotates a binding
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r:<Range T=int/> = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Suffixes compose after an applied type
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type Schedule = { slots:<Range T=int/>[] override:<Range T=int/>? }`
- **THEN** analysis SHALL treat `slots` as a list of `<Range T=int/>` and `override` as a nullable `<Range T=int/>`

#### Scenario: An alias names an instantiation
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type IntRange = <Range T=int/>` and `let r:IntRange = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Applied types nest and take nullable arguments
- **WHEN** a file contains `type Box = { T:type value:T } type IntBox = <Box T=int/> type MaybeInt = int?` and `let a:<Box T=<Box T=int/>/> = <Box T=IntBox value={<Box T=int value={1} />} />` and `let b:<Box T=int?/> = <Box T=MaybeInt value={null} />`
- **THEN** analysis SHALL accept both bindings, the aliases standing in at the construction sites for arguments that are not bare names

#### Scenario: A sequence type is not a type argument
- **WHEN** a file contains `type Box = { T:type value:T } type Ints = int[]` and `let a:<Box T=int[]/> = <Box T=Ints value={ 1 2 } />` and `type Page = { T:type items:T[] } let p:<Page T=Ints/> = <Page T=Ints items={ 1 2 } />`
- **THEN** analysis SHALL reject the annotation `<Box T=int[]/>`, the argument `T=Ints` at both construction sites, and the annotation `<Page T=Ints/>`
- **AND** each diagnostic SHALL explain that a type argument must not be a sequence, naming the alias where one was written

#### Scenario: A type parameter in scope is a type argument
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `component <Slider TValue:type range:<Range T=TValue/> /> = { <Label /> }` and `type Span = { T:type bounds:<Range T=T/> }`
- **THEN** analysis SHALL accept both declarations

#### Scenario: A generic record refers to itself
- **WHEN** a file contains `type Node = { T:type value:T next:<Node T=T/>? }`
- **THEN** analysis SHALL accept the declaration

#### Scenario: Arguments are matched by name
- **WHEN** a file contains `type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `let p:<Pair TValue=int TKey=string/> = <Pair TKey=string TValue=int key="a" value={1} />`
- **THEN** analysis SHALL accept the binding

## REMOVED Requirements

### Requirement: An applied type names one instantiation of a generic record
**Reason**: A type argument must be an item type, so the scenario that bound `T=int[]` no longer describes accepted behaviour.
**Migration**: Bind the element type instead, and take the sequence where the record declares `T[]`; where a field must hold a sequence, declare a record around it.
