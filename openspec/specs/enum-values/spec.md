# enum-values Specification

## Purpose
Defines how NX preserves the authored spelling of a constant union case — a case that declares no
fields in a union that declares no base — across runtime behavior, tooling, examples, and
documentation.

The capability keeps its `enum-values` path. The `enum` keyword was removed by
`replace-enums-with-unions`, but the concept it named did not go anywhere: a closed set of named
constants with a bare-string wire contract recovered through the target type is exactly what a
constant union is, and it is what the C# and TypeScript generators still emit. Moving the
requirements to another capability would have lost their history for no behavioral gain. The
requirements below are stated in constant-case terms.

## Requirements
### Requirement: Constant case names are preserved exactly across runtime and tooling
Constant union case names SHALL preserve the exact identifier spelling written in source across
lowering, type analysis, runtime values, formatting, code generation, and host-facing value
conversion. The system SHALL NOT rewrite a case name from `snake_case` to `PascalCase` or any other
casing when it is displayed, serialized, or exposed through first-party tooling. Canonical raw
payloads SHALL represent a constant case as the bare authored case string, and schema-aware
consumers SHALL recover the declaring union from the target context (declared NX type, typed DTO
property, or other type annotation) rather than from an in-payload wrapper.

First-party rendering of a constant case SHALL split by context: where the output is NX source at a
typed binding site, it SHALL use the bare case form; everywhere else — hover, diagnostics, and value
display — it SHALL use the qualified `Union.case` form, because there the declaring type is the
information the reader needs.

#### Scenario: Snake_case case name survives evaluation and canonical host value conversion
- **WHEN** source defines `type DealStage = draft | pending_review | closed_won`
- **AND** NX evaluates `DealStage.pending_review`
- **THEN** the runtime value SHALL preserve the case name `pending_review`
- **AND** any first-party canonical raw host value conversion for that value SHALL expose the bare
  authored case string `"pending_review"`
- **AND** first-party display of that value SHALL use `DealStage.pending_review`

#### Scenario: Schema-aware consumer recovers union identity from the target type
- **WHEN** a first-party consumer receives the bare string `"pending_review"` as part of a
  canonical raw payload and knows the target field's declared type is `DealStage`
- **THEN** the consumer SHALL map that string to `DealStage.pending_review` using the constant-case
  authored-string contract
- **AND** the consumer SHALL reject unknown case strings with a type mismatch error rather than
  silently accepting them as plain strings

#### Scenario: Source-position formatting emits the bare case form
- **WHEN** first-party formatting renders a constant case value as an NX property value in NX source
- **THEN** it SHALL emit `stage=pending_review`
- **AND** it SHALL NOT emit `stage="DealStage.pending_review"`, which is a string literal and does
  not type check when read back at a union-typed property

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

### Requirement: First-party constant-union examples and fixtures use snake_case by convention
First-party NX examples, docs, test fixtures, and grammar tests that introduce constant union cases
SHALL use `snake_case` case names by convention instead of `PascalCase`.

#### Scenario: Repository examples follow the documented case-name convention
- **WHEN** the repository adds or updates a constant-union example such as `Status`, `Direction`, or
  `DealStage`
- **THEN** those first-party examples SHALL use case names such as `active`, `north`, or
  `pending_review`
- **AND** the documented convention for union case names SHALL be `snake_case`
