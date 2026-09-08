# source-analysis-pipeline Specification

## Purpose
Defines the shared static-analysis contract for NX source text, including aggregated diagnostics,
file-name fidelity, and the analyze-then-execute boundary used by source-driven runtime APIs.

## Requirements
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

### Requirement: Type analysis reaches every expression an element encloses
Type analysis SHALL infer a type for every lowered expression reachable from a module's items,
including expressions written as the body content or the property values of an element, and SHALL do
so regardless of whether the element's tag resolves to a declaration in scope.

An element whose tag names no function, component, record, or union case has no binding contract to
check its content and properties against, and the analysis SHALL still infer each of those
expressions on its own terms and report the diagnostics that inference produces. The absent tag
SHALL be the only thing left unchecked about such an element.

The recorded type of every such expression SHALL be available to callers through the analysis
result's type environment, on the same terms as an expression written outside any element.

#### Scenario: A type error inside an unresolved element's content is reported
- **WHEN** a module contains `let root() = <div>{1 + "a"}</div>` and `div` names no declaration in
  scope
- **THEN** the analysis result SHALL include the diagnostic rejecting the addition of `int` and
  `string`
- **AND** it SHALL be the same diagnostic reported for `let root() = { 1 + "a" }`

#### Scenario: A type error in an unresolved element's property value is reported
- **WHEN** a module contains `let root() = <div class={1 + "a"} />` and `div` names no
  declaration in scope
- **THEN** the analysis result SHALL include the diagnostic rejecting the addition of `int` and
  `string`

#### Scenario: An element nested inside an unresolved element is still checked against its declaration
- **WHEN** a module declares `let <Wrap content:string /> = <div />`
- **AND** contains `let root() = <div><Wrap content={42} /></div>`
- **THEN** the analysis result SHALL include the diagnostic reporting that `content` on `Wrap`
  expects `string` and found `int`
- **AND** it SHALL be the same diagnostic reported when the `<Wrap />` element is written outside
  the `<div>`

#### Scenario: A property supplied twice on an unresolved element is reported
- **WHEN** a module contains `let root() = <div class=1 class=2 />` and `div` names no declaration
  in scope
- **THEN** the analysis result SHALL report that `class` is supplied more than once
- **AND** it SHALL report it on the same terms as a duplicate supplied to a resolved element,
  because detecting one reads the element's own property list rather than any contract

#### Scenario: An unresolved tag reports nothing beyond the tag itself
- **WHEN** a module contains `let root() = <div>{"ok"}</div>` and `div` names no declaration in
  scope
- **THEN** the analysis result SHALL report no diagnostic for the content expression

#### Scenario: The type of an expression inside an unresolved element is recorded
- **WHEN** a module contains `let <Row count:int /> = <div>{count}</div>`
- **THEN** the analysis result's type environment SHALL record `int` for the `count` expression
  written inside the `<div>`


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

### Requirement: Workspace source analysis aggregates module diagnostics
The shared source-analysis pipeline SHALL support analyzing multiple in-memory workspace modules as
one effective workspace. Workspace analysis SHALL parse, lower, prepare, validate, resolve scopes,
and type-check submitted modules with the same semantic ordering used for program construction.

#### Scenario: Multiple workspace module errors are returned together
- **WHEN** a workspace contains `main.nx` with a lowering error
- **AND** the same workspace contains `shared/value.nx` with a type error
- **THEN** workspace analysis SHALL return diagnostics for both modules in one result

#### Scenario: Workspace analysis does not require per-module caller loops
- **WHEN** a caller validates an `NxWorkspace` containing three modules
- **THEN** NX SHALL analyze the effective workspace through one workspace validation call
- **AND** callers SHALL NOT be required to invoke single-source analysis once per module

### Requirement: Shared source analysis accepts module source providers
The shared source-analysis pipeline SHALL operate on a logical set of source modules provided by a
source provider rather than being tied directly to either in-memory workspace descriptors or
filesystem paths. In-memory and filesystem-backed callers SHALL use the same analysis ordering once
their source modules have been loaded.

#### Scenario: File-backed analysis uses the shared module set
- **WHEN** a caller analyzes filesystem-backed NX source
- **THEN** the filesystem source provider SHALL load the source modules
- **AND** shared source analysis SHALL parse, lower, prepare, validate, resolve scopes, and
  type-check those modules through the same module-set path used by workspace validation

#### Scenario: Provider differences do not change static analysis ordering
- **WHEN** equivalent NX modules are supplied once through `NxWorkspace` and once through a
  filesystem source provider
- **THEN** shared source analysis SHALL run the same static phases in the same order for both
  source providers

### Requirement: Workspace diagnostics use submitted source maps
Diagnostic conversion for workspace analysis SHALL calculate label spans from the submitted
workspace source text associated with each normalized identity before falling back to any
file-backed diagnostic behavior.

#### Scenario: Path-like workspace identity is not re-read from disk
- **WHEN** a workspace module identity is `shared/config.nx`
- **AND** a different file with that name exists on disk
- **AND** workspace validation reports a diagnostic in `shared/config.nx`
- **THEN** the diagnostic span SHALL be calculated from the submitted workspace source text
- **AND** NX SHALL NOT re-read the disk file to compute the line and column

#### Scenario: Workspace diagnostic labels preserve normalized identity
- **WHEN** a caller submits `shared/./config.nx`
- **AND** workspace analysis reports a diagnostic in that module
- **THEN** the diagnostic label file SHALL be `shared/config.nx`
