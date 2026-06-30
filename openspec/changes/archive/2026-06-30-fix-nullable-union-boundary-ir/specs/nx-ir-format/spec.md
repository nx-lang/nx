## ADDED Requirements

### Requirement: Emitted IR is boundary-clean for valid nullable and content-boundary programs
For a source program that passes analysis and native evaluation, emitted NX IR SHALL preserve the
schema, default, nullable, nominal, and content-property metadata required for supported runtimes to
evaluate public entrypoints without rejecting the program's own generated values at a boundary
schema check. The IR runtime output SHALL match native canonical JSON-compatible output for
nullable union fields and content-derived required fields.

#### Scenario: Nullable union field does not emit synthetic invalid case
- **WHEN** a valid program constructs a record with an omitted or explicit-null field typed as a nullable discriminated union
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode the field absence as `null` or as an omitted nullable field that normalizes to `null`
- **AND** evaluating the IR SHALL NOT produce an undeclared union discriminator such as `FlowCompletion.undefined`

#### Scenario: Content property children satisfy required field through IR
- **WHEN** a valid program constructs a record or external component whose required content property is supplied by element body content
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL preserve the target content-field name and body expressions
- **AND** evaluating the IR SHALL populate the required content field before boundary validation reports missing fields

#### Scenario: Imported library record and union references remain boundary-clean
- **WHEN** a workspace program imports record, component, and union declarations from loaded libraries and returns a value using those imported declarations
- **AND** native evaluation succeeds and NX IR is emitted for the workspace program
- **THEN** evaluating the IR SHALL use module-qualified nominal references for boundary normalization
- **AND** the IR output SHALL match native canonical JSON-compatible output
