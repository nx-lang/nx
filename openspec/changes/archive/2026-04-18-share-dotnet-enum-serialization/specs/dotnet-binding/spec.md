## ADDED Requirements

### Requirement: Managed binding exposes reusable enum serialization helpers for typed DTOs
`NxLang.Runtime` SHALL expose public generic enum serialization helpers in
`NxLang.Nx.Serialization` so generated C# enums and hand-written managed enums can share the same
typed DTO serialization path for `System.Text.Json` and MessagePack. The helper contract SHALL use
an explicit wire-format mapping type rather than inferring authored NX member strings from CLR enum
member names.

#### Scenario: Runtime exposes shared helper types for generated enums
- **WHEN** a C# caller or generated file references shared NX enum serialization infrastructure
- **THEN** `NxLang.Runtime` SHALL expose public `INxEnumWireFormat<TEnum>`
- **AND** SHALL expose public `NxEnumJsonConverter<TEnum, TWire>`
- **AND** SHALL expose public `NxEnumMessagePackFormatter<TEnum, TWire>`
- **AND** the helper contract SHALL allow the caller-provided wire-format type to map
  `DealStage.PendingReview` to `"pending_review"` explicitly

#### Scenario: Managed binding enum uses the shared helper path
- **WHEN** a caller serializes or deserializes `NxSeverity.Warning` through the managed typed DTO
  workflow
- **THEN** the managed binding SHALL use the shared enum helper infrastructure to emit and parse the
  plain member string `"warning"`
- **AND** SHALL NOT require dedicated `NxSeverityJsonConverter` or
  `NxSeverityMessagePackFormatter` support types
