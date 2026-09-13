## MODIFIED Requirements

### Requirement: The bottom type is inference-internal and has no source spelling
The system SHALL provide a bottom type that is below every type: it SHALL satisfy every expected
type, and joining it with any type SHALL yield that type. It has two sources: the empty list, whose
type is a list of it, and which is usable at every list-typed site for exactly that reason; and a
component type parameter that a use site leaves unspecified, which is checked at that element as
the bottom type so that a site binding nothing typed by the parameter needs no argument.

That type SHALL NOT be nameable in NX source. `never` SHALL NOT be a primitive type name, SHALL NOT
be offered as a completion, and SHALL NOT be highlighted as a primitive, so the primitive set stays
the eight names above. A user declaration MAY take the name `never`, resolved by the same rules that
govern any non-primitive name.

The system SHALL render it as `never` in a diagnostic that names a type it inferred, on the same
terms as the unit type. Where a diagnostic reports a type the author wrote as `{}`, it SHALL spell
it `{}` rather than naming the bottom type, because that is the form the author can act on. Where a
diagnostic reports a type that an unspecified type parameter fixed, it SHALL name the parameter and
the `Name=` form rather than naming the bottom type, for the same reason.

No value SHALL have the bottom type, so it SHALL NOT appear in any runtime representation, in a
value crossing the host boundary, or in a runtime type test. Its surface is therefore smaller than
the unit type's, which does appear in each of those.

Code generation SHALL render it in every target it can reach. It can reach only the targets that
render *inferred* types; a target that maps from source type annotations SHALL NOT be able to reach
it at all, because it has no source spelling to map from. An unspecified type parameter is a
use-site fact and SHALL NOT reach code generation through a declaration.

#### Scenario: An empty list is usable at every list-typed site
- **WHEN** type inference analyzes an empty braced list
- **THEN** its type SHALL be a list of the bottom type
- **AND** it SHALL satisfy `string[]`, `int[]`, and any other list type, without an expected type
  having been supplied to determine an element type

#### Scenario: The bottom type cannot be written in source
- **WHEN** a file annotates a binding as `never` and no type named `never` is declared
- **THEN** analysis SHALL NOT resolve the annotation to the bottom type
- **AND** SHALL treat it as a reference to an undeclared name

#### Scenario: A user declaration may take the name `never`
- **WHEN** a file contains `type never = { value:int }` and a field declared `n:never`
- **THEN** analysis SHALL resolve `n` to the user-defined record type

#### Scenario: The primitive set is unchanged by the bottom type
- **WHEN** an editor requests primitive type completions, or syntax highlighting classifies a name
  in type position
- **THEN** `never` SHALL NOT be among the names offered or highlighted

#### Scenario: An unspecified type parameter is reported by name
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type items:TItem[]? /> let contacts:Contact[] = {} let v = <List items={contacts} />`
- **THEN** analysis SHALL reject `items`
- **AND** the diagnostic SHALL name `TItem` and show `TItem=`, and SHALL NOT spell the expected type as `never[]?`
