# drawnui-nx-catalog Specification

## Purpose
Defines the NX external component catalog that mirrors the DrawnUI control set, so NX authors can
describe a drawn user interface using the same control and property names DrawnUI itself uses.

## Requirements

### Requirement: Catalog declares the full DrawnUI control set
The catalog SHALL declare one NX external component for every control DrawnUI exposes as a
renderable tag, and SHALL declare an abstract external component for every shared base in DrawnUI's
control hierarchy so that inherited properties are declared once.

#### Scenario: Every renderable control is authorable
- **WHEN** an author writes an element whose name matches any DrawnUI renderable control
- **THEN** the catalog SHALL declare a matching external component
- **AND** the component name SHALL be spelled exactly as DrawnUI spells it

#### Scenario: Shared properties are declared on abstract bases
- **WHEN** two or more controls share a property because they share a DrawnUI base class
- **THEN** the catalog SHALL declare that property on an abstract external component
- **AND** the concrete components SHALL extend that abstract component rather than redeclaring the
  property

#### Scenario: Inherited properties are accepted at call sites
- **WHEN** an author sets a property that a control inherits from a base rather than declaring itself
- **THEN** compilation SHALL succeed
- **AND** the property SHALL appear in the evaluated output

### Requirement: Catalog is derived from DrawnUI sources rather than hand-maintained
The catalog SHALL be generated from the vendored DrawnUI TypeScript sources by resolving each
control's public property surface through the TypeScript type system, and SHALL be regenerable on
demand so that a DrawnUI sync is followed by a catalog refresh rather than manual editing.

#### Scenario: Accessor-defined properties are captured
- **WHEN** a DrawnUI control exposes a property through a getter and setter pair rather than a field
- **THEN** the generated catalog SHALL declare that property

#### Scenario: Regeneration is reproducible
- **WHEN** the catalog is generated twice from the same DrawnUI sources
- **THEN** both runs SHALL produce identical catalog output

#### Scenario: Generated catalog is committed
- **WHEN** a contributor checks out the repository without running the generator
- **THEN** the generated catalog SHALL already be present
- **AND** building and running the application SHALL NOT require regenerating it

### Requirement: Catalog excludes members that cannot be authored
The catalog SHALL exclude DrawnUI members that are not author-settable inputs: engine-internal and
computed state, and — for this change — every callback or event property, because the JavaScript IR
runtime cannot dispatch NX actions.

#### Scenario: Callback properties are omitted
- **WHEN** a DrawnUI control declares a property whose type is a function
- **THEN** the catalog SHALL NOT declare that property

#### Scenario: Engine state is omitted
- **WHEN** a DrawnUI member exists to carry measured, cached, or otherwise engine-computed state
- **THEN** the catalog SHALL NOT declare that member as a property

#### Scenario: Setting an excluded property is reported
- **WHEN** an author sets a property the catalog excludes
- **THEN** compilation SHALL fail with a diagnostic naming the unknown property

### Requirement: DrawnUI types map onto NX types by documented rules
The catalog SHALL map each DrawnUI property type onto an NX type by a fixed, documented set of
rules, so that an evaluated NX value carries enough information for a renderer to reconstruct the
value DrawnUI expects.

#### Scenario: String-literal unions become NX unions
- **WHEN** a DrawnUI property's type is a union of string literals
- **THEN** the catalog SHALL declare an NX union type whose case names are spelled exactly as those
  string literals
- **AND** an author SHALL be able to write a case name unqualified at that property

#### Scenario: Structured values become NX record types
- **WHEN** a DrawnUI property's type is a structured value such as a thickness, corner radius,
  shadow, point, or gradient
- **THEN** the catalog SHALL declare an NX record type with the same field names
- **AND** the evaluated value SHALL identify which record type it is

#### Scenario: Colors are strings
- **WHEN** a DrawnUI property's type is a color
- **THEN** the catalog SHALL declare it as a string

#### Scenario: Grid track sizes are strings for now
- **WHEN** a DrawnUI property expresses a grid track size
- **THEN** the catalog SHALL declare it as a string
- **AND** the catalog SHALL record that a discriminated union is the intended eventual model

#### Scenario: Container children are typed by the control hierarchy
- **WHEN** a DrawnUI control accepts child controls
- **THEN** the catalog SHALL declare a content property typed as a sequence of the abstract control
  base
- **AND** an author SHALL be able to nest any catalog control inside it

### Requirement: Catalog compiles to NX IR
The catalog SHALL be expressible in the subset of NX that NX IR code generation supports, so that
the fiddle's compile pipeline never fails because of the catalog itself.

#### Scenario: Union-valued property defaults survive IR generation
- **WHEN** a catalog property's default value is a union case
- **THEN** NX IR generation SHALL succeed

#### Scenario: Catalog alone is valid NX
- **WHEN** the catalog is compiled with no author source
- **THEN** compilation SHALL report no errors

### Requirement: Catalog divergence from DrawnUI is recorded
Where the catalog deliberately differs from DrawnUI — because a DrawnUI type has no NX equivalent,
or because the vendored DrawnUI source was edited to suit NX — the divergence SHALL be recorded in
the sample app's documentation.

#### Scenario: Simplified property types are documented
- **WHEN** a DrawnUI property's type is narrowed or simplified in the catalog
- **THEN** the documentation SHALL name the property and state what was simplified

#### Scenario: Edits to vendored DrawnUI are documented
- **WHEN** the vendored DrawnUI source is edited rather than copied verbatim
- **THEN** the documentation SHALL record what was changed and why
