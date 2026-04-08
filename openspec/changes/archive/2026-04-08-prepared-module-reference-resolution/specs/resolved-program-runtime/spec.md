## MODIFIED Requirements

### Requirement: Runtime references are module-qualified
Any runtime-owned reference to executable NX code SHALL identify both the owning module and the
stable local item, expression, or element within that module. Item references in a resolved program
SHALL preserve the exact local definition identity of the target declaration and SHALL NOT require
top-level runtime lookup to rediscover that declaration by visible string name once the program has
been resolved.

#### Scenario: Function references identify the owning lowered module and exact local definition
- **WHEN** runtime resolution records a callable reference for a function exported from
  `forms/input.nx`
- **THEN** that callable reference SHALL include the identity of `forms/input.nx`
- **AND** that callable reference SHALL include the exact local function item reference within that
  module

#### Scenario: Imported component references do not require a name rescan at execution time
- **WHEN** a root source file imports a component exported from another module and runtime
  resolution records a reference to that component
- **THEN** the resolved component reference SHALL include the owning module identity and exact local
  component item reference
- **AND** runtime execution SHALL use that resolved reference without rescanning the owning module
  for the visible component name

#### Scenario: Captured handler references identify the owning lowered module
- **WHEN** a component action handler captures an expression from `search-box.nx`
- **THEN** the runtime-owned handler reference SHALL include the identity of `search-box.nx`
- **AND** the runtime-owned handler reference SHALL include the local expression reference within
  that module
