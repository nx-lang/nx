## MODIFIED Requirements

### Requirement: Enum member spellings are preserved exactly across runtime and tooling
NX enum members SHALL preserve the exact identifier spelling written in source across lowering,
type analysis, runtime values, formatting, code generation, and host-facing value conversion. The
system SHALL NOT rewrite enum members from `snake_case` to `PascalCase` or any other casing when
they are displayed, serialized, or exposed through first-party tooling. Canonical raw enum payloads
SHALL preserve enum identity explicitly with `"$enum"` and `"$member"`, while schema-aware
generated host models MAY project the same authored member spelling as a plain string when the enum
type is already known.

#### Scenario: Snake_case enum member survives evaluation and canonical host value conversion
- **WHEN** source defines `enum DealStage = draft | pending_review | closed_won`
- **AND** NX evaluates `DealStage.pending_review`
- **THEN** the runtime enum value SHALL preserve the member name `pending_review`
- **AND** any first-party canonical raw host value conversion for that enum value SHALL expose
  `"$enum": "DealStage"` and `"$member": "pending_review"`
- **AND** first-party formatting or display of that value SHALL use `DealStage.pending_review`

