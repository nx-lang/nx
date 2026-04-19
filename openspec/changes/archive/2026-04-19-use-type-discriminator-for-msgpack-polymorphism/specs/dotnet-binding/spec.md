## ADDED Requirements

### Requirement: Managed raw-value and typed-model polymorphic record workflows share a single `$type` wire shape
The managed NX binding SHALL represent polymorphic NX records with the same `$type`-discriminated
map contract across both raw `NxValue` runtime-result workflows and schema-aware typed-model
MessagePack workflows. Generated typed DTO serialization and deserialization for polymorphic NX
record families SHALL align with the canonical raw runtime shape rather than a separate
MessagePack-specific union envelope.

#### Scenario: Typed MessagePack polymorphic record serialization matches raw runtime shape
- **WHEN** a C# caller serializes a generated typed DTO value for `SearchRequested` through
  MessagePack
- **THEN** the payload SHALL encode the record as a map containing `$type: "SearchRequested"` and
  the declared record fields
- **AND** the payload SHALL NOT use a MessagePack `Union` discriminator envelope

#### Scenario: Typed MessagePack polymorphic record deserialization accepts canonical `$type` map values
- **WHEN** a C# caller deserializes MessagePack bytes produced from canonical raw runtime output for
  a polymorphic record family
- **THEN** the managed typed workflow SHALL resolve the concrete CLR type from the `$type` field
- **AND** SHALL populate declared fields using their authored NX wire names

#### Scenario: Raw-to-typed round-trip preserves polymorphic record identity
- **WHEN** a C# caller receives a polymorphic record from raw runtime output and then maps it
  through a typed DTO MessagePack workflow
- **THEN** the resulting value SHALL preserve the same concrete record identity indicated by `$type`
- **AND** the typed and raw workflows SHALL remain wire-compatible for that value
