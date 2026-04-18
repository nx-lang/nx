## ADDED Requirements

### Requirement: Generated C# enums use authored member strings across JSON and MessagePack
Generated C# enums SHALL preserve the authored NX enum member spellings across both
`System.Text.Json` and MessagePack. Generated C# enum properties and values SHALL serialize as the
plain authored enum member string rather than as the canonical raw `NxValue` enum map, and typed
generated enum deserialization SHALL use that same string form for both serializers.

#### Scenario: Generated C# JSON enum serialization uses the authored member string
- **WHEN** source contains `export enum DealStage = | draft | pending_review | closed_won`
- **THEN** generated C# SHALL include JSON enum serialization support that emits `"pending_review"`
  for `DealStage.PendingReview`
- **AND** SHALL NOT require a `"$enum"` or `"$member"` wrapper for the typed JSON enum value

#### Scenario: Generated C# MessagePack enum serialization uses the authored member string
- **WHEN** source contains `export enum DealStage = | draft | pending_review | closed_won`
- **THEN** generated C# SHALL include MessagePack enum serialization support that emits the string
  `pending_review`
- **AND** typed MessagePack enum handling SHALL use that string-based wire shape rather than the
  canonical raw enum map shape

