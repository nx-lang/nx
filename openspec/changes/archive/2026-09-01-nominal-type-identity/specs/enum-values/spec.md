## MODIFIED Requirements

### Requirement: Constant cases are referenceable without naming the union type
At a binding site whose declared type is a discriminated union, the system SHALL accept a bare case
name and SHALL resolve it against that union's constant cases, without requiring the union type to
be in lexical scope at the use site. A declaration's property types SHALL be resolved in the
namespace of the module that declares them, so the union a bare name resolves against is the one the
declaring module named, not whatever the use site happens to spell the same way.

A resolved case SHALL lower and evaluate whether or not the using module can name its union. The
system SHALL NOT require an import of the union type beyond what the binding site itself already
needs.

#### Scenario: Union-valued property resolves against the declaring module
- **WHEN** a module imports a component whose property is typed by a union declared in another
  module, and does not import that union type
- **THEN** the bare case name SHALL resolve against the declaring module's union
- **AND** an unknown case SHALL be reported against that union's cases, not against any same-named
  type visible at the use site

#### Scenario: A same-named local union does not stand in for the declaring module's union
- **WHEN** a module declares its own union sharing a name with the union that types an imported
  component's property
- **THEN** neither a bare case of the local union nor its qualified form SHALL be accepted at that
  property
- **AND** the diagnostic SHALL list the declaring module's cases and distinguish the two unions by
  their declaring modules

#### Scenario: A case whose union is not nameable here reports the needed import
- **WHEN** a bare name resolves against a union that the using module cannot name
- **THEN** the bare case name SHALL be accepted
- **AND** it SHALL evaluate to the same value as the qualified form of that case
- **AND** no diagnostic SHALL require the union type to be imported

#### Scenario: Union case is reachable at a typed site under a wildcard import alias
- **WHEN** a module contains `import "../ui" as ui` and sets a `ui.TextVariant`-typed property on an
  imported component
- **THEN** the bare case name SHALL be accepted at that property
- **AND** authors SHALL NOT be required to switch to the selective import form to reach the case
