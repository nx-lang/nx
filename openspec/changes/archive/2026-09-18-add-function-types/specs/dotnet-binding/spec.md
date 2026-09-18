## ADDED Requirements

### Requirement: A function value is read through a managed function reference type
The managed NX binding SHALL provide a function reference type that a host types a function-valued
property of a rendered element with, exposing the declaring module's identity and the function's
declared name — the two fields of the rendered `Function` record. It SHALL serialize in both output
formats the binding supports, MessagePack and JSON, including when the property carries no value,
and generated C# SHALL type a function-typed member with it rather than with a host delegate, which
neither format can serialize. Calling a function value from .NET is out of scope: a host reads
which function it was handed and passes the record on.

#### Scenario: A rendered function-typed property is read as a function reference
- **WHEN** a C# caller evaluates a module `templates.nx` that declares `external component <List ItemTemplate:(<function Item:object Index:int />: string)? /> let <Row Item:object Index:int />: string = "r" let root() = <List ItemTemplate={Row} />` into a typed element whose `ItemTemplate` property is typed as the managed function reference type
- **THEN** the property SHALL expose the module `templates.nx` and the name `Row`
- **AND** the same evaluation as JSON SHALL render `ItemTemplate` as a `Function` record with those
  two fields

#### Scenario: A contract with a function-typed member serializes in both formats
- **WHEN** a typed element with a function-typed property is serialized and deserialized through
  MessagePack and through JSON
- **THEN** both round trips SHALL preserve the module and the name
- **AND** both SHALL succeed when the property is null, because a member type that cannot be
  resolved would otherwise make the whole containing contract unserializable

#### Scenario: Generated C# types a function-typed member as the function reference
- **WHEN** C# types are generated for `export external component <DataTable RowTemplate:(<function Item:object Index:int />: string)? />`
- **THEN** the generated member SHALL be the managed function reference type
- **AND** SHALL NOT be a host delegate type
