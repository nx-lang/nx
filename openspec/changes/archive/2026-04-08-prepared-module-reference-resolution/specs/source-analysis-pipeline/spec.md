## MODIFIED Requirements

### Requirement: Shared source analysis aggregates static diagnostics
The system SHALL expose a shared source-analysis entry point for NX source text that performs
parsing, raw lowering, prepared-module construction, top-level binding construction,
prepared-module semantic validation, lexical scope resolution, and type checking in a single call.
The analysis result SHALL return every diagnostic produced by those static phases, and it SHALL
return a `ModuleArtifact` for the analyzed source file. That `ModuleArtifact` SHALL preserve the
raw file-local `LoweredModule` whenever parsing produced a syntax tree.

#### Scenario: Binding-driven prepared validation and type diagnostics are returned together
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** analyzes source file `app/main.nx` containing `import "../ui"`,
  `type TextField extends Field = { label:string }`, and `let root(): int = "oops"`
- **AND** the loaded `../ui` visible namespace contains `export abstract type Field = { label:string }`
- **THEN** the analysis result SHALL include a prepared-module validation diagnostic rejecting the
  duplicate inherited field `label`
- **AND** the analysis result SHALL include a type diagnostic rejecting `root(): int = "oops"`
- **AND** the returned `ModuleArtifact` SHALL preserve the raw file-local `LoweredModule` for the
  parsed source

#### Scenario: Fatal parse failure returns diagnostics without a lowered module
- **WHEN** a caller analyzes malformed source such as `let root( =`
- **THEN** the analysis result SHALL return parse diagnostics
- **AND** the returned `ModuleArtifact` SHALL NOT include a `LoweredModule`

### Requirement: Core type analysis operates on caller-prepared lowered modules
The shared analysis core SHALL treat module preparation as a caller-owned step. Lower-level
analysis entry points SHALL accept a prepared module that separates preserved raw HIR from the
prepared visible namespace and binding tables, SHALL run prepared-module semantic validation before
scope building and type checking, SHALL use prepared bindings rather than top-level string rescans
for name-resolution-dependent work, and SHALL NOT perform implicit filesystem library loading
themselves.

#### Scenario: Prepared-module analysis validates a caller-prepared namespace without loading libraries
- **WHEN** a caller lowers `app/main.nx` that imports `../question-flow`
- **AND** prepares bindings that inject visible declarations from a supplied build context
- **AND** passes that prepared module to the shared analysis core
- **THEN** the analysis result SHALL preserve the raw module's import metadata
- **AND** SHALL use the prepared bindings for name-resolution-dependent validation, scope building,
  and type checking
- **AND** SHALL NOT silently load `../question-flow` from disk during that analysis call

## ADDED Requirements

### Requirement: Raw lowering remains file-local and does not require prepared visibility
Raw lowering SHALL preserve unresolved top-level references that depend on peer files or imported
libraries without requiring those names to be available during lowering. Diagnostics for those
references SHALL be emitted only after prepared bindings have been constructed.

#### Scenario: Same-library base name is preserved during raw lowering
- **WHEN** one library contains `base.nx` with `abstract type Field = { label:string }`
- **AND** `derived.nx` in that same library contains `type TextField extends Field = { placeholder:string? }`
- **THEN** raw lowering of `derived.nx` SHALL preserve the `extends Field` declaration without
  requiring `Field` to be present in the raw file-local module

#### Scenario: Imported base name is preserved during raw lowering
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and
  `type TextField extends Field = { placeholder:string? }`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` that exports abstract `Field`
- **THEN** raw lowering of `app/main.nx` SHALL preserve the `extends Field` declaration without
  requiring `Field` to be present in the raw file-local module

### Requirement: Name-resolution-dependent semantic validation runs only on prepared modules
Any semantic validation that depends on visible-name resolution SHALL run only after the analyzing
file has been prepared with same-library peer declarations and imported library interfaces and after
the prepared binding tables for those names exist. Raw lowering MUST NOT emit those diagnostics.

#### Scenario: Same-library peer declaration resolves during prepared-module validation
- **WHEN** one library contains `base.nx` with `abstract type Field = { label:string }`
- **AND** `derived.nx` in that same library contains `type TextField extends Field = { placeholder:string? }`
- **THEN** raw lowering of `derived.nx` SHALL preserve the `extends Field` declaration without
  reporting an unresolved-base diagnostic
- **AND** prepared-module validation of `derived.nx` SHALL resolve `Field` successfully through a
  same-library prepared binding

#### Scenario: Imported library declaration resolves during prepared-module validation
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and
  `type TextField extends Field = { placeholder:string? }`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` that exports abstract `Field`
- **THEN** raw lowering of `app/main.nx` SHALL preserve the `extends Field` declaration without
  reporting an unresolved-base diagnostic
- **AND** prepared-module validation SHALL resolve `Field` successfully through an imported
  prepared binding
