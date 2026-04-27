## ADDED Requirements

### Requirement: Property-list fragments preserve conditional property bindings
The system SHALL represent property-list entries as ordered property fragments rather than only as
a flat list of direct key/value properties. Direct property values, simple conditional fragments,
condition-list fragments, and match-style fragments MUST be preserved through parsing and lowering
with source spans suitable for diagnostics.

#### Scenario: Simple conditional property fragment lowers
- **WHEN** a file contains `<Button if showLabel { label="Save" } />`
- **THEN** lowering SHALL preserve a conditional property fragment whose then branch contains
  property `label`
- **AND** the fragment SHALL NOT be silently dropped

#### Scenario: Condition-list property fragment lowers
- **WHEN** a file contains `<Badge if { isError => tone="danger" isWarning => tone="warning" else => tone="neutral" } />`
- **THEN** lowering SHALL preserve a condition-list property fragment with two condition arms and
  one else branch
- **AND** each branch SHALL preserve its contained property bindings

#### Scenario: Match-style property fragment lowers
- **WHEN** a file contains `<View if state is { LoadState.failed => message=state.message else => message="" } />`
- **THEN** lowering SHALL preserve a match-style property fragment with scrutinee `state`
- **AND** the `LoadState.failed` arm and the `else` branch SHALL preserve their property bindings

### Requirement: Property-list fragments type check possible property paths
Type checking SHALL validate each possible property path produced by conditional property
fragments. A required property MUST be provided on every reachable path. A duplicate property MUST
be rejected when two bindings for the same property can occur on the same path. The same property
MAY appear in mutually exclusive branches.

#### Scenario: Required property supplied by every branch is accepted
- **WHEN** a component requires `label:string`
- **AND** a call site contains `<Button if primary { label="Save" } else { label="Cancel" } />`
- **THEN** type checking SHALL accept the invocation because every branch supplies `label`

#### Scenario: Required property missing on one branch is rejected
- **WHEN** a component requires `label:string`
- **AND** a call site contains `<Button if primary { label="Save" } else { } />`
- **THEN** type checking SHALL reject the invocation because one reachable branch omits `label`

#### Scenario: Same property in mutually exclusive branches is accepted
- **WHEN** a component accepts `tone:string`
- **AND** a call site contains `<Badge if isError { tone="danger" } else { tone="neutral" } />`
- **THEN** type checking SHALL accept the invocation because only one `tone` binding can occur at
  runtime

#### Scenario: Static property colliding with conditional branch is rejected
- **WHEN** a component accepts `tone:string`
- **AND** a call site contains `<Badge tone="neutral" if isError { tone="danger" } />`
- **THEN** type checking SHALL reject the invocation because `tone` can be supplied twice on the
  `isError` path

#### Scenario: Duplicate properties on the same branch are rejected
- **WHEN** a component accepts `tone:string`
- **AND** a call site contains `<Badge if isError { tone="danger" tone="warning" } />`
- **THEN** type checking SHALL reject the invocation because one reachable path supplies `tone`
  twice

### Requirement: Property-list fragments evaluate to active invocation properties
Runtime evaluation SHALL evaluate property-list fragments into the ordered active property bindings
for the selected execution path before invoking the target component, function, record, or union
case constructor. Runtime evaluation MUST NOT apply last-wins duplicate behavior.

#### Scenario: Simple conditional selects then properties
- **WHEN** a component call contains `<Button if enabled { label="Save" } else { label="Disabled" } />`
- **AND** `enabled` evaluates to true
- **THEN** runtime evaluation SHALL invoke `Button` with property `label="Save"`

#### Scenario: Simple conditional selects else properties
- **WHEN** a component call contains `<Button if enabled { label="Save" } else { label="Disabled" } />`
- **AND** `enabled` evaluates to false
- **THEN** runtime evaluation SHALL invoke `Button` with property `label="Disabled"`

#### Scenario: Match fragment selects matching arm properties
- **WHEN** a component call contains `<Notice if state is { LoadState.failed => message=state.message else => message="" } />`
- **AND** `state` evaluates to `LoadState.failed`
- **THEN** runtime evaluation SHALL invoke `Notice` with `message` set from the failed case

### Requirement: Unsupported property-list fragments produce diagnostics
The system SHALL NOT silently drop a syntactically valid property-list fragment. If a fragment kind
is parsed but not fully supported by lowering, type checking, or runtime evaluation, the system
MUST report an explicit unsupported-feature diagnostic for that fragment.

#### Scenario: Incomplete fragment support reports an error
- **WHEN** a property-list fragment kind is present in source
- **AND** the implementation cannot lower or analyze that fragment kind
- **THEN** the system SHALL emit a diagnostic naming the unsupported property-list fragment
- **AND** it SHALL NOT silently omit the fragment's properties from the invocation

### Requirement: Property-list fragments are documented and tooled
The system SHALL update first-party documentation, examples, snippets, and VS Code grammar tests
for supported conditional and match-style property-list fragments.

#### Scenario: Documentation shows conditional property fragments
- **WHEN** a reader opens the NX syntax reference for element properties
- **THEN** the documentation SHALL show an example of conditional property-list fragments
- **AND** it SHALL describe duplicate and required-property branch rules

#### Scenario: Tooling highlights property-list match fragments
- **WHEN** the VS Code grammar tokenizes `<View if state is { LoadState.failed => message=state.message } />`
- **THEN** it SHALL highlight the conditional keyword, match keyword, pattern, property name, and
  property value consistently with surrounding NX syntax
