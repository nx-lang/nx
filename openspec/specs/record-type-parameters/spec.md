# record-type-parameters Specification

## Purpose

Lets a record declare type parameters with the same property syntax a component uses, and lets any
type position name one instantiation of that record as an applied type such as `<Range T=int/>`, so
that container records like `Range`, `Pair` and `Page` are checked per argument rather than erased.

## Requirements
### Requirement: A record declares type parameters as leading `type`-typed properties
A plain `type` record declaration SHALL accept a property definition whose declared type is the
keyword `type`, and SHALL treat each such definition as a type parameter of the record. The rules
of a component type parameter apply unchanged: type parameter definitions MUST precede every other
property definition, MUST NOT carry a default value, MUST NOT carry a modifier such as `content`,
MUST NOT carry an occurrence suffix on `type` or the `?` mark on the name, MUST NOT take the name of
a primitive type or of the built-in `Element` type, and MUST have a name distinct from every other
type parameter and field of the record. The system SHALL reject a violation of any of these rules
with a diagnostic that names the offending definition. A type parameter SHALL be a type in the
record's own field types, SHALL be rigid there — satisfied only by itself — and SHALL NOT be a field
of the record, a value, or a case of the record's derived property union.

#### Scenario: Record declares a type parameter and uses it in its fields
- **WHEN** a file contains `type Range = { T:type start:T end:T endInclusive:boolean }`
- **THEN** analysis SHALL accept the declaration
- **AND** SHALL treat `T` as a type parameter of `Range` and `start`, `end` and `endInclusive` as its only fields

#### Scenario: Record declares several type parameters
- **WHEN** a file contains `type Pair = { TKey:type TValue:type key:TKey value:TValue }`
- **THEN** analysis SHALL accept the declaration with type parameters `TKey` and `TValue` in that order

#### Scenario: A type parameter composes with suffixes and function types
- **WHEN** a file contains `type Page = { T:type items:T+ next?:T render?:<function item:T />: string }`
- **THEN** analysis SHALL accept the declaration
- **AND** `next` SHALL read as `T?` and `render` as `(<function item:T />: string)?`

#### Scenario: A type parameter after a field is rejected
- **WHEN** a file contains `type Bad = { start:int T:type }`
- **THEN** analysis SHALL reject `T` with a diagnostic saying a type parameter must be declared before every field

#### Scenario: A type parameter with a default, a modifier, or a reserved name is rejected
- **WHEN** a file contains `type A = { T:type = int }` and `type B = { content T:type }` and `type C = { string:type }` and `type D = { Element:type }` and `type E = { T?:type }` and `type F = { T:type+ }`
- **THEN** analysis SHALL reject each declaration with a diagnostic that names the offending definition

#### Scenario: Duplicate names are rejected
- **WHEN** a file contains `type A = { T:type T:type }` and `type B = { T:type T:string }`
- **THEN** analysis SHALL reject the second `T` in each declaration

#### Scenario: A parameter-typed field rejects a concrete default
- **WHEN** a file contains `type Bad = { T:type value:T = "text" }`
- **THEN** analysis SHALL reject the default because `string` does not satisfy the rigid parameter `T`

#### Scenario: A type parameter is not a value or a field
- **WHEN** a file contains `type Box = { T:type value:T }` and `let b = <Box T=int value={1} />` and `let x = b.T`
- **THEN** analysis SHALL reject `b.T` as an unknown field of `Box`

### Requirement: Type parameters remain rejected outside records and component signatures
The system SHALL continue to reject a `Name:type` definition in an `action` declaration, an emitted
action, a state group, a function parameter list and a function type, with a diagnostic that names
the definition and the position. A record that declares a type parameter SHALL NOT be `abstract`
and SHALL NOT have an `extends` clause, and the system SHALL reject either with a diagnostic that
names the record and says generic records do not take part in inheritance.

#### Scenario: An action cannot declare a type parameter
- **WHEN** a file contains `action Picked = { T:type item:T }`
- **THEN** analysis SHALL reject `T` with a diagnostic saying a type parameter is not supported in an action

#### Scenario: A generic record cannot be abstract
- **WHEN** a file contains `abstract type Base = { T:type value:T }`
- **THEN** analysis SHALL reject the declaration with a diagnostic naming `Base`

#### Scenario: A generic record cannot extend a base
- **WHEN** a file contains `abstract type Shape = { name:string } type Tagged extends Shape = { T:type tag:T }`
- **THEN** analysis SHALL reject the declaration with a diagnostic naming `Tagged`

### Requirement: An applied type names one instantiation of a generic record with item-type arguments
Every type position SHALL accept an applied type, written as a self-closing element whose tag is
the name of a generic record and whose properties are `Name=Type` type arguments:
`<Range T=int/>`. The tag MAY be a qualified name. A type argument SHALL be an exactly-one type — a
primitive, a record, a union, an alias, a function type, a type parameter in scope, or another
applied type. A type argument SHALL NOT carry an occurrence, whether spelled with `?`, `+` or `*` or
reached through an alias, and the system SHALL reject one with a diagnostic explaining that a type
argument must be exactly one value, as `occurrence-types` requires; optionality belongs on the
field that holds the value (`value?:T`). An applied type SHALL take an occurrence suffix like any
other type reference, SHALL be usable as the target of a type alias, and SHALL be usable as a field
type inside the generic record it applies. Type arguments MAY be written in any order and SHALL be
matched to parameters by name.

#### Scenario: An applied type annotates a binding
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r:<Range T=int/> = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Suffixes compose after an applied type
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type Schedule = { slots:<Range T=int/>+ override?:<Range T=int/> }`
- **THEN** analysis SHALL treat `slots` as one or more `<Range T=int/>` and `override` as reading `<Range T=int/>?`

#### Scenario: An alias names an instantiation
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type IntRange = <Range T=int/>` and `let r:IntRange = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Applied types nest and take nullable arguments
- **WHEN** a file contains `type Box = { T:type value:T } type IntBox = <Box T=int/> type MaybeInt = int?` and `let a:<Box T=<Box T=int/>/> = <Box T=IntBox value={<Box T=int value={1} />} />` and `let b:<Box T=int?/> = <Box T=MaybeInt value={} />`
- **THEN** analysis SHALL accept `a`, the alias standing in at the construction site for an argument that is not a bare name
- **AND** SHALL reject the annotation `<Box T=int?/>` and the argument `T=MaybeInt`, each explaining that a type argument must be exactly one value and naming `MaybeInt` where the alias was written
- **AND** `type OptBox = { T:type value?:T }` with `let c = <OptBox T=int />` SHALL be accepted, `c.value` reading `int?`

#### Scenario: A sequence type is not a type argument
- **WHEN** a file contains `type Box = { T:type value:T } type Ints = int+` and `let a:<Box T=int+/> = <Box T=Ints value={ 1 2 } />` and `type Page = { T:type items:T+ } let p:<Page T=Ints/> = <Page T=Ints items={ 1 2 } />`
- **THEN** analysis SHALL reject the annotation `<Box T=int+/>`, the argument `T=Ints` at both construction sites, and the annotation `<Page T=Ints/>`
- **AND** each diagnostic SHALL explain that a type argument must be exactly one value, naming the alias where one was written

#### Scenario: A type parameter in scope is a type argument
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `component <Slider TValue:type range:<Range T=TValue/> /> = { <Label /> }` and `type Span = { T:type bounds:<Range T=T/> }`
- **THEN** analysis SHALL accept both declarations

#### Scenario: A generic record refers to itself
- **WHEN** a file contains `type Node = { T:type value:T next?:<Node T=T/> }`
- **THEN** analysis SHALL accept the declaration

#### Scenario: Arguments are matched by name
- **WHEN** a file contains `type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `let p:<Pair TValue=int TKey=string/> = <Pair TKey=string TValue=int key="a" value={1} />`
- **THEN** analysis SHALL accept the binding

### Requirement: A malformed applied type is rejected by name
The system SHALL reject, each with a diagnostic that names the record and the argument involved: a
generic record's name used as a type with no arguments; an applied type that omits a type
parameter; an applied type that binds a name the record does not declare as a type parameter; an
applied type that binds the same parameter twice; an applied type whose tag is not a generic
record; and a type argument that does not resolve to a type. The diagnostic for a missing argument
SHALL show the `<Name Param=.../>` form.

#### Scenario: A bare generic record name is not a type
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type Slider = { range:Range }`
- **THEN** analysis SHALL reject `Range` with a diagnostic saying `T` was not specified and showing `<Range T=.../>`

#### Scenario: A missing argument is rejected
- **WHEN** a file contains `type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `type Bad = { p:<Pair TKey=string/> }`
- **THEN** analysis SHALL reject the applied type with a diagnostic naming `TValue`

#### Scenario: An unknown or repeated argument is rejected
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type A = { r:<Range U=int/> }` and `type B = { r:<Range T=int T=string/> }`
- **THEN** analysis SHALL reject `U` as not a type parameter of `Range`
- **AND** SHALL reject the second `T` as already bound

#### Scenario: Applying a record with no type parameters is rejected
- **WHEN** a file contains `type Contact = { name:string }` and `type Bad = { c:<Contact T=int/> }`
- **THEN** analysis SHALL reject the applied type with a diagnostic saying `Contact` has no type parameters

#### Scenario: An unresolved type argument is rejected
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `type Bad = { r:<Range T=Contatc/> }`
- **THEN** analysis SHALL reject `Contatc` as an unresolved type

### Requirement: Applied types are distinct per argument and invariant
Two applied types SHALL be the same type exactly when they apply the same record declaration and
bind the same type to every parameter, after aliases are resolved. An applied type SHALL satisfy
an expected applied type only when the two are the same type: a type argument SHALL NOT widen,
narrow, or convert, even where the argument types themselves convert implicitly. Every applied
type SHALL satisfy `object`. Reading a field of a value of an applied type SHALL give the field's
declared type with each parameter replaced by its argument, and with the field's own occurrence
applied as `optional-properties` defines. Invariance SHALL decide the common type of two applied
types as it decides assignment: two of one instantiation SHALL have that instantiation as their
common type, arguments and all, and two that are not the same type SHALL have `object`. No
inferred type SHALL ever be a generic record's bare name, which is not a type a type position
accepts.

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
- **WHEN** a file contains `type Page = { T:type items:T+ next?:T }` and `let p:<Page T=string/> = <Page T=string items={ "a" } />` and `let first:string+ = p.items` and `let bad:int? = p.next`
- **THEN** analysis SHALL accept `first`
- **AND** SHALL reject `bad` because `p.next` is `string?`

#### Scenario: An applied type satisfies object
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let o:object = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: A list of one instantiation keeps its arguments
- **WHEN** a file contains `type Box = { T:type value:T }` and `let a = <Box T=int value={1} />` and `let b = <Box T=int value={2} />` and `let bad:int = { a b }`
- **THEN** analysis SHALL reject `bad` with a mismatch naming `<Box T=int/>+`

#### Scenario: A list of two instantiations is a list of object
- **WHEN** a file contains `type Box = { T:type value:T }` and `let a = <Box T=int value={1} />` and `let b = <Box T=string value="x" />` and `let bad:int = { a b }` and `let mixed:object+ = { a b }`
- **THEN** analysis SHALL reject `bad` with a mismatch naming `object+`, never a sequence of a bare `Box`
- **AND** SHALL accept `mixed`

### Requirement: Constructing a generic record binds every type argument
An element whose tag is a generic record SHALL bind each of the record's type parameters as a
property `Name=TypeName`, where the argument is a bare type name under the same rules as a
component use site: a primitive type name, or a record, union, alias or type parameter in scope. A
braced or quoted argument and a conditional argument SHALL be rejected as they are for a
component. An element that leaves a type parameter unbound SHALL be rejected with a diagnostic
that names the parameter and shows the `Name=` form; there is no inference and no bottom-type
fallback. The element SHALL have the applied type its arguments name, each field binding SHALL be
checked against the field's type with the arguments substituted — so a literal converts to the
argument type where a literal of that kind would convert to it anywhere else — and a type-argument
binding SHALL NOT be a missing, unknown or duplicate field.

#### Scenario: Construction produces the applied type
- **WHEN** a file contains `type Range = { T:type start:T end:T endInclusive:boolean = false }` and `let r = <Range T=int start={1} end={5} />` and `let typed:<Range T=int/> = r`
- **THEN** analysis SHALL accept both bindings

#### Scenario: A field binding is checked against the substituted type
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let bad = <Range T=int start="a" end={5} />`
- **THEN** analysis SHALL reject `start` because `string` does not satisfy `int`

#### Scenario: A literal converts to the argument type
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r:<Range T=float64/> = <Range T=float64 start={0} end={1} />`
- **THEN** analysis SHALL accept the binding with `start` and `end` as `float64` values

#### Scenario: An unbound type parameter is rejected
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let bad = <Range start={1} end={5} />`
- **THEN** analysis SHALL reject the element with a diagnostic saying `T` was not specified and showing `T=`
- **AND** SHALL NOT report `start` or `end` as a mismatch against `never`

#### Scenario: A braced, quoted or conditional argument is rejected
- **WHEN** a file contains `type Box = { T:type value:T } let flag = true` and `let a = <Box T={int} value={1} />` and `let b = <Box T="int" value={1} />` and `let c = <Box if flag { T=int } value={1} />`
- **THEN** analysis SHALL reject each element, directing the author to the bare form `T=int`

#### Scenario: A generic record from another module is constructed and applied
- **WHEN** a library module declares `export type Range = { T:type start:T end:T }` and another module imports it and contains `let r:<Range T=int/> = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the binding

### Requirement: A generic record's derived companions follow its type parameters
The derived update record `R.Update` of a generic record `R` SHALL declare the same type
parameters as `R` and SHALL be named in a type position as an applied type whose tag is the
qualified companion name, `<R.Update T=.../>`. Constructing it SHALL bind every type argument as
constructing `R` does. The update intrinsics SHALL carry type arguments from a record to its
companion: for a value of type `<R T=A/>`, `apply` SHALL expect and `diff` SHALL produce
`<R.Update T=A/>`, and `merge` SHALL require both operands to be the same applied update type.
The derived property union `R.Property` SHALL have one case per field, SHALL NOT have a case for a
type parameter, and SHALL NOT take type arguments.

#### Scenario: The update companion is applied like its record
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let u:<Range.Update T=int/> = <Range.Update T=int end={9} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Intrinsics carry the argument
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r = <Range T=int start={1} end={5} />` and `let moved:<Range T=int/> = apply(r, <Range.Update T=int end={9} />)` and `let bad = apply(r, <Range.Update T=string end="z" />)`
- **THEN** analysis SHALL accept `moved`
- **AND** SHALL reject `bad` because `<Range.Update T=string/>` is not the update type of `<Range T=int/>`

#### Scenario: The property union ignores type parameters
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let k:Range.Property = {Range.Property.start}`
- **THEN** analysis SHALL accept the binding
- **AND** SHALL reject `Range.Property.T`

### Requirement: Type arguments leave no trace at runtime
A type-argument binding SHALL be removed from a record construction once type checking has
consumed it, so that the constructed value has no field for a type parameter and interpretation,
code generation and every other consumer below the type checker observe a construction with no
such binding. Two values of a generic record built with different type arguments and equal fields
SHALL be equal values, and a runtime that validates a host-supplied value against a field typed by
a type parameter SHALL treat that field as `object`.

#### Scenario: The constructed value has no type-parameter field
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r = <Range T=int start={1} end={5} />`
- **AND** `r` is evaluated
- **THEN** the resulting record SHALL have the fields `start` and `end` and SHALL NOT have a field `T`

#### Scenario: Evaluation agrees across backends
- **WHEN** a program constructs a generic record, reads one of its fields, and applies an update to it
- **THEN** the interpreter, the generated executable code and the TypeScript IR runtime SHALL produce the same value
