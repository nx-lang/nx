## ADDED Requirements

### Requirement: Executable code supports conditional property fragments
Executable code generation SHALL support simple conditional, condition-list and match-style
property fragments. Only the selected branch SHALL contribute properties or evaluate its property
expressions. Match-arm narrowing, defaults, authored evaluation order and duplicate-property
validation SHALL be preserved in Rust IR, TypeScript IR and generated JavaScript execution.

#### Scenario: A conditional supplies its active label
- **WHEN** `enabled` is true in `<button if enabled { label="Save" } else { label="Disabled" } />`
- **THEN** execution SHALL produce a button whose label is `Save`
- **AND** false SHALL select `Disabled`

#### Scenario: A condition list chooses its first matching branch
- **WHEN** two condition-list arms are true
- **THEN** only the first arm SHALL contribute properties
- **AND** an inactive branch that would fail SHALL not be evaluated

#### Scenario: A match fragment reads the narrowed payload
- **WHEN** a fragment matches a payload case and binds a property from its payload
- **THEN** execution SHALL produce the property's payload value

#### Scenario: An omitted property retains its default
- **WHEN** the selected path contributes no binding for a defaulted property
- **THEN** construction SHALL apply the declared default
- **AND** an explicitly empty binding SHALL keep its distinct source semantics

#### Scenario: Mutually exclusive bindings are accepted
- **WHEN** both branches of a conditional bind `label` exactly once
- **THEN** compilation SHALL accept those bindings and execution SHALL supply the selected one

#### Scenario: Duplicate active properties are rejected
- **WHEN** two properties can bind the same name on an active path
- **THEN** compilation SHALL reject the duplicate rather than apply last-wins behavior

#### Scenario: Properties preserve authored evaluation order
- **WHEN** direct properties and multiple conditional fragments are mixed in one invocation
- **THEN** guards and selected value expressions SHALL be evaluated once in their source-defined order
- **AND** inactive expressions that would fail SHALL not be evaluated

#### Scenario: Multiple independent fragments compile compactly
- **WHEN** an invocation contains an increasing number of independent conditional fragments
- **THEN** emitted IR and generated code SHALL grow linearly with the fragments and bindings
- **AND** lowering SHALL NOT enumerate every combination of branch outcomes

#### Scenario: Content properties retain their binding rules
- **WHEN** an invocation combines conditional properties with body content
- **THEN** content binding and duplicate validation SHALL follow the existing content-property rules
- **AND** conditional compile-time type arguments SHALL remain rejected under existing rules

#### Scenario: Conditional properties work for each existing invocation target
- **WHEN** a supported fragment supplies properties to an intrinsic element, record, payload union case, component or named-argument function invocation
- **THEN** each target SHALL receive only the selected bindings through its existing construction or invocation semantics
- **AND** target-specific validation and defaults SHALL remain in force

#### Scenario: Required property missing on a possible path is rejected
- **WHEN** a required property has no default and a possible branch omits it
- **THEN** shared source analysis SHALL reject the invocation under the existing property-path rules
- **AND** lowering SHALL NOT insert an empty value to make the branch appear complete

#### Scenario: Inactive branches remain statically checked
- **WHEN** a fragment branch contains an invalid name or type under existing analysis rules
- **THEN** shared source analysis SHALL still report that error
- **AND** runtime branch laziness SHALL NOT bypass static validation
