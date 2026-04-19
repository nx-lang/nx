## MODIFIED Requirements

### Requirement: Enum member spellings are preserved exactly across runtime and tooling
NX enum members SHALL preserve the exact identifier spelling written in source across lowering,
type analysis, runtime values, formatting, code generation, and host-facing value conversion. The
system SHALL NOT rewrite enum members from `snake_case` to `PascalCase` or any other casing when
they are displayed, serialized, or exposed through first-party tooling. Canonical raw enum payloads
SHALL represent enum members as the bare authored member string, and schema-aware consumers SHALL
recover the declaring enum type from the target context (declared NX type, typed DTO property,
or other type annotation) rather than from an in-payload wrapper.

#### Scenario: Snake_case enum member survives evaluation and canonical host value conversion
- **WHEN** source defines `enum DealStage = draft | pending_review | closed_won`
- **AND** NX evaluates `DealStage.pending_review`
- **THEN** the runtime enum value SHALL preserve the member name `pending_review`
- **AND** any first-party canonical raw host value conversion for that enum value SHALL expose the
  bare authored member string `"pending_review"`
- **AND** first-party formatting or display of that value SHALL use `DealStage.pending_review`

#### Scenario: Schema-aware consumer recovers enum identity from the target type
- **WHEN** a first-party consumer receives the bare string `"pending_review"` as part of a
  canonical raw payload and knows the target field's declared enum type is `DealStage`
- **THEN** the consumer SHALL map that string to `DealStage.pending_review` using the enum's
  authored-member-string contract
- **AND** the consumer SHALL reject unknown member strings with a type mismatch error rather than
  silently accepting them as plain strings
