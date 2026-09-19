## ADDED Requirements

### Requirement: Generated contracts carry a generic record as a generic type in both languages
The C# and TypeScript `typegen` emitters SHALL declare an exported generic record with one generic
parameter per record type parameter, in declaration order and under the parameter's authored name,
SHALL type each field that mentions a parameter with that generic parameter, and SHALL NOT emit a
member for a type parameter. Both emitters SHALL render an applied type as the instantiation of
that generic type, with arguments in the record's declaration order regardless of the order the
source wrote them, and with each argument mapped as that language maps the same type anywhere else.
Unlike a component contract, a C# generic record SHALL NOT be erased, because a host names the
concrete instantiation at its own deserialization site. An applied type whose argument is a
component type parameter SHALL follow the component rule for that argument: `object` in C#, the
generic parameter in TypeScript. A generic record's `<Name>_update` companion SHALL declare the
same generic parameters the record does and SHALL type each of its fields as the record types the
field it patches, so that a patch of one instantiation is usable at that instantiation; in C# it
SHALL extend `NxUpdate<Name<T, …>>` and its `<Name>Properties` key table SHALL declare the same
parameters. A component's state companion SHALL continue to erase the *component's* type
parameters, because the type it patches is concrete. Neither companion SHALL put a type argument
on the wire: the discriminator and the field names are unchanged by the parameters.

#### Scenario: C# declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests C# output
- **THEN** the generated record SHALL be declared as `Range<T>` with `Start` and `End` typed `T` and `EndInclusive` typed `bool`
- **AND** SHALL NOT declare a `T` property

#### Scenario: TypeScript declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `Range<T>` with `start` and `end` typed `T`
- **AND** SHALL NOT declare a `T` property

#### Scenario: An applied type is rendered as the instantiation
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }`
- **THEN** the generated C# `Slider` SHALL declare `Range` as `Range<double>` and `Marks` as a nullable list of `Range<long>`
- **AND** the generated TypeScript `Slider` SHALL declare `range` as `Range<number>` and `marks` as a nullable array of `Range<number>`

#### Scenario: Arguments are emitted in declaration order
- **WHEN** NX source declares `export type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `export type Entry = { p:<Pair TValue=int TKey=string/> }`
- **THEN** the generated C# SHALL type `P` as `Pair<string, long>` and the generated TypeScript SHALL type `p` as `Pair<string, number>`

#### Scenario: A component type parameter as an argument follows the component rule
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export external component <Slider TValue:type range:<Range T=TValue/>? />`
- **THEN** the generated C# `Slider` SHALL declare `Range` as a nullable `Range<object>`
- **AND** the generated TypeScript `Slider<TValue = unknown>` SHALL declare `range` as a nullable `Range<TValue>`

#### Scenario: The update companion of a generic record carries the parameter
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }`
- **THEN** the generated TypeScript SHALL declare `Range_update<T>` with `start` typed as optional `T`
- **AND** the generated C# SHALL declare `Range_update<T> : NxUpdate<Range<T>>` with `Start` typed `NxOptional<T>`, `Diff(Range<T>, Range<T>)` returning `Range_update<T>`, and a `RangeProperties<T>` key table
- **AND** the serialized patch SHALL carry `"$type": "Range.Update"` and the field names alone, with no type argument

#### Scenario: A patch of a generic record applies to the instantiation it came from
- **WHEN** a C# host diffs two `Range<long>` values, serializes the patch and reads it back through either serializer
- **THEN** applying the patch SHALL produce a `Range<long>` carrying the changed fields
- **AND** reading a patched field SHALL give the record's field type rather than an untyped value

#### Scenario: A component's state companion still erases the component's parameters
- **WHEN** NX source declares `export external component <Picker TItem:type /> = { state { sel:TItem? } <Label /> }`
- **THEN** the generated `Picker_update` SHALL declare no generic parameter, and SHALL type `sel` as optional `unknown` in TypeScript and `NxOptional<object?>` in C#
