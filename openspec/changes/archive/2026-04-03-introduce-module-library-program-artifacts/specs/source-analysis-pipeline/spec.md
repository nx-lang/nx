## MODIFIED Requirements

### Requirement: Shared source analysis aggregates static diagnostics
The system SHALL expose a shared source-analysis entry point for NX source text that performs
parsing, lowering, scope building, and type checking in a single call. The analysis result SHALL
return every diagnostic produced by those static phases, and it SHALL return a `ModuleArtifact` for
the analyzed source file. That `ModuleArtifact` SHALL preserve the `LoweredModule` whenever parsing
produced a syntax tree.

#### Scenario: Lowering and type diagnostics are returned together
- **WHEN** a caller analyzes source containing `record Base = { id:int }`, `record Child : Base = { name:string }`, and `let root(): int = "oops"`
- **THEN** the analysis result SHALL include a lowering diagnostic rejecting `Child : Base`
- **AND** the analysis result SHALL include a type diagnostic rejecting `root(): int = "oops"`
- **AND** the returned `ModuleArtifact` SHALL preserve the `LoweredModule` for the parsed source

#### Scenario: Fatal parse failure returns diagnostics without a lowered module
- **WHEN** a caller analyzes malformed source such as `let root( =`
- **THEN** the analysis result SHALL return parse diagnostics
- **AND** the returned `ModuleArtifact` SHALL NOT include a `LoweredModule`

### Requirement: Shared source analysis preserves caller file identity in diagnostics
The shared source-analysis entry point SHALL preserve the caller-provided `file_name` and source
spans on static diagnostics returned from lowering, scope building, and type checking, including
the diagnostics preserved on the resulting `ModuleArtifact`.

#### Scenario: Lowering and type diagnostics retain the provided file name
- **WHEN** a caller analyzes source named `widgets/search-box.nx` that contains both a lowering
  error and a type error
- **THEN** the primary labels on the returned lowering diagnostics SHALL use `widgets/search-box.nx`
- **AND** the primary labels on the returned type diagnostics SHALL use `widgets/search-box.nx`
- **AND** each label span SHALL point at the offending source text within that file

### Requirement: Source-driven runtime execution stops on static analysis errors
Source-driven runtime entry points SHALL treat shared source analysis as a required first phase. If
the `ModuleArtifact` returned from shared analysis contains any error diagnostics, the entry point
SHALL return those diagnostics and SHALL NOT build executable program state or execute interpreter
behavior for that source.

#### Scenario: Root evaluation is gated by static analysis
- **WHEN** `eval_source` is called with source that contains a lowering error and also defines a
  `root` function
- **THEN** `eval_source` SHALL return the static analysis diagnostics from the shared analysis phase
- **AND** `eval_source` SHALL NOT build a `ProgramArtifact` or `ResolvedProgram`
- **AND** `eval_source` SHALL not execute `root`
