# source-analysis-pipeline Specification

## Purpose
Defines the shared static-analysis contract for NX source text, including aggregated diagnostics,
file-name fidelity, and the analyze-then-execute boundary used by source-driven runtime APIs.

## Requirements
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

### Requirement: Shared source analysis resolves imports through a supplied registry-backed build context
Shared source analysis SHALL resolve local library imports through a supplied `ProgramBuildContext`
backed by a `LibraryRegistry`. Shared analysis SHALL NOT silently load missing local libraries from
disk during that request. Program construction SHALL reuse that same direct-import resolution
result when selecting library snapshots for the resulting `ProgramArtifact`.

#### Scenario: Shared analysis succeeds with a preloaded imported library
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** analyzes source file `app/main.nx` containing `import "../question-flow"`
- **THEN** shared analysis SHALL resolve the import through the supplied build context

#### Scenario: Shared analysis reports a missing library from the build context
- **WHEN** a host analyzes source file `app/main.nx` containing `import "../question-flow"`
- **AND** the supplied `ProgramBuildContext` does not expose a loaded `../question-flow` snapshot
- **THEN** shared analysis SHALL return a library-load diagnostic for the missing normalized root
- **AND** the analysis call SHALL NOT silently load `../question-flow` from disk

#### Scenario: Program construction does not re-select a direct library hidden from analysis
- **WHEN** a `LibraryRegistry` contains loaded `../ui` and `../internal-admin` snapshots
- **AND** a `ProgramBuildContext` exposes only `../ui`
- **AND** source file `app/main.nx` contains `import "../internal-admin"`
- **THEN** shared analysis SHALL report the missing loaded `../internal-admin` snapshot from that
  build context
- **AND** the resulting `ProgramArtifact` SHALL NOT silently select the hidden
  `../internal-admin` snapshot during later program assembly

### Requirement: Core type analysis operates on caller-prepared lowered modules
The shared type-analysis core SHALL treat import resolution as a caller-owned preparation step.
Lower-level analysis entry points SHALL accept an already-lowered or already-prepared module and
SHALL NOT perform implicit filesystem library loading themselves.

#### Scenario: Prepared-module analysis preserves imports without loading libraries
- **WHEN** a caller lowers `app/main.nx` that imports `../question-flow`
- **AND** passes that lowered module to the shared type-analysis core without an import-preparation
  step
- **THEN** the analysis result SHALL preserve the module's import metadata
- **AND** SHALL NOT silently load `../question-flow` from disk during that analysis call
