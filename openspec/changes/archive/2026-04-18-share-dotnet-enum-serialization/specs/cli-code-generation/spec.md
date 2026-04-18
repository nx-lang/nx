## ADDED Requirements

### Requirement: Generated C# enums reuse shared runtime serialization helpers
Generated C# enums SHALL reference shared enum serialization helpers from `NxLang.Runtime` for both
`System.Text.Json` and MessagePack instead of emitting a dedicated converter and formatter type per
enum. Generated output SHALL continue to emit an enum-specific wire-format mapping type that
preserves the authored NX member strings explicitly.

#### Scenario: Generated C# enum references shared runtime helpers
- **WHEN** source contains `export enum DealStage = | draft | pending_review | closed_won`
- **THEN** generated C# SHALL include `using NxLang.Nx.Serialization;`
- **AND** SHALL annotate `DealStage` with `NxEnumJsonConverter<DealStage, DealStageWireFormat>`
- **AND** SHALL annotate `DealStage` with
  `NxEnumMessagePackFormatter<DealStage, DealStageWireFormat>`
- **AND** SHALL emit `DealStageWireFormat` with explicit mappings between
  `DealStage.PendingReview` and `"pending_review"`
- **AND** SHALL NOT emit dedicated `DealStageJsonConverter` or `DealStageMessagePackFormatter`
  types

#### Scenario: Generated C# enum mapping remains explicit when CLR names are normalized
- **WHEN** source contains `export enum BuildTarget = | web_api | ios_app`
- **THEN** generated C# SHALL emit CLR members `WebApi` and `IosApp`
- **AND** SHALL preserve the authored wire strings `"web_api"` and `"ios_app"` through the
  generated wire-format mapping type rather than inferring them from CLR member names at runtime
