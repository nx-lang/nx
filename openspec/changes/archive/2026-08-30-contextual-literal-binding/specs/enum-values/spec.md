## MODIFIED Requirements

### Requirement: Enum member spellings are preserved exactly across runtime and tooling
NX enum members SHALL preserve the exact identifier spelling written in source across lowering,
type analysis, runtime values, formatting, code generation, and host-facing value conversion. The
system SHALL NOT rewrite enum members from `snake_case` to `PascalCase` or any other casing when
they are displayed, serialized, or exposed through first-party tooling. Canonical raw enum payloads
SHALL represent enum members as the bare authored member string, and schema-aware consumers SHALL
recover the declaring enum type from the target context (declared NX type, typed DTO property,
or other type annotation) rather than from an in-payload wrapper. First-party rendering of an enum
value SHALL split by context: where the output is NX source at a typed binding site, it SHALL use
the bare member form; everywhere else — hover, diagnostics, and value display — it SHALL use the
qualified `Type.member` form, because there the declaring type is the information the reader needs.

#### Scenario: Snake_case enum member survives evaluation and canonical host value conversion
- **WHEN** source defines `enum DealStage = draft | pending_review | closed_won`
- **AND** NX evaluates `DealStage.pending_review`
- **THEN** the runtime enum value SHALL preserve the member name `pending_review`
- **AND** any first-party canonical raw host value conversion for that enum value SHALL expose the
  bare authored member string `"pending_review"`
- **AND** first-party display of that value SHALL use `DealStage.pending_review`

#### Scenario: Schema-aware consumer recovers enum identity from the target type
- **WHEN** a first-party consumer receives the bare string `"pending_review"` as part of a
  canonical raw payload and knows the target field's declared enum type is `DealStage`
- **THEN** the consumer SHALL map that string to `DealStage.pending_review` using the enum's
  authored-member-string contract
- **AND** the consumer SHALL reject unknown member strings with a type mismatch error rather than
  silently accepting them as plain strings

#### Scenario: Source-position formatting emits the bare member form
- **WHEN** first-party formatting renders an enum value as an NX property value in NX source
- **THEN** it SHALL emit `stage=pending_review`
- **AND** it SHALL NOT emit `stage="DealStage.pending_review"`, which is a string literal and does
  not type check when read back at an enum-typed property

## ADDED Requirements

### Requirement: Enum members are referenceable without naming the enum type
At a binding site whose declared type is an enum, the system SHALL accept a bare member name and
SHALL resolve it against that enum, without requiring the enum type to be in lexical scope at the
use site. A declaration's property types SHALL be resolved in the namespace of the module that
declares them, so the enum a bare name resolves against is the one the declaring module named, not
whatever the use site happens to spell the same way.

Resolving a member is distinct from carrying it through lowering. Until nominal types carry their
declaring origin, a resolved member whose enum is not nameable in the using module SHALL be
reported as needing an import, rather than accepted and lowered to an unresolvable reference.

Telling a foreign enum apart from a same-named local one is bounded by the same missing origin. Until
nominal types carry it, two same-named enums SHALL be distinguished by their declared members: a
local enum declaring different members SHALL NOT stand in for the foreign one. Two same-named enums
declaring the same members are not distinguished, and the local one is accepted. Because an enum
member carries only its name, that acceptance produces the same value either way; what it does not
yet provide is identity.

#### Scenario: Enum-valued property resolves against the declaring module
- **WHEN** a module imports a component whose property is typed by an enum declared in another
  module, and does not import that enum type
- **THEN** the bare member name SHALL resolve against the declaring module's enum
- **AND** an unknown member SHALL be reported against that enum's members, not against any
  same-named type visible at the use site

#### Scenario: A same-named local enum with different members does not stand in for the declaring module's enum
- **WHEN** a module declares its own enum sharing a name with the enum that types an imported
  component's property, and the two declare different members
- **THEN** neither a bare member of the local enum nor its qualified form SHALL be accepted at that
  property
- **AND** the diagnostic SHALL list the declaring module's members

#### Scenario: A member whose enum is not nameable here reports the needed import
- **WHEN** a bare name resolves against an enum that the using module cannot name
- **THEN** the system SHALL report that the enum must be imported, naming the qualified form to
  write instead
- **AND** it SHALL NOT emit a reference that fails to resolve during code generation
