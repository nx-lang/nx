# enum-values Specification

## Purpose
Defines how NX preserves enum member spelling across runtime behavior, tooling, examples, and
documentation.

## Requirements
### Requirement: Enum member spellings are preserved exactly across runtime and tooling
NX enum members SHALL preserve the exact identifier spelling written in source across lowering,
type analysis, runtime values, formatting, code generation, and host-facing value conversion. The
system SHALL NOT rewrite enum members from `snake_case` to `PascalCase` or any other casing when
they are displayed, serialized, or exposed through first-party tooling. Canonical raw enum
payloads SHALL preserve enum identity explicitly with `"$enum"` and `"$member"`, while
schema-aware generated host models MAY project the same authored member spelling as a plain string
when the enum type is already known.

#### Scenario: Snake_case enum member survives evaluation and canonical host value conversion
- **WHEN** source defines `enum DealStage = draft | pending_review | closed_won`
- **AND** NX evaluates `DealStage.pending_review`
- **THEN** the runtime enum value SHALL preserve the member name `pending_review`
- **AND** any first-party canonical raw host value conversion for that enum value SHALL expose
  `"$enum": "DealStage"` and `"$member": "pending_review"`
- **AND** first-party formatting or display of that value SHALL use `DealStage.pending_review`

### Requirement: First-party enum examples and fixtures use snake_case by convention
First-party NX examples, docs, test fixtures, and grammar tests that introduce enum members SHALL
use `snake_case` member names by convention instead of `PascalCase`.

#### Scenario: Repository examples follow the documented enum convention
- **WHEN** the repository adds or updates an enum example such as `Status`, `Direction`, or
  `DealStage`
- **THEN** those first-party examples SHALL use member names such as `active`, `north`, or
  `pending_review`
- **AND** the documented convention for enum members SHALL be `snake_case`
