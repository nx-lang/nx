## ADDED Requirements

### Requirement: An artifact carries one module and names the modules it links against
An NX IR artifact SHALL carry exactly one module. It SHALL carry a module table whose first entry
describes the artifact's own module and whose remaining entries describe every module the artifact
references, each entry giving the module's logical identity, the version string the build was given
for it (empty when none was given) and a fingerprint of its source. A program that spans several
modules SHALL be emitted as one artifact per module, and a build SHALL let the caller choose which
modules to emit.

#### Scenario: A snippet compiled against a catalog names the catalog
- **WHEN** a workspace holds `drawnui` with version `9` and `input.nx`, and `input.nx` uses a control
  `drawnui` declares
- **AND** the caller emits an artifact for `input.nx` only
- **THEN** the build SHALL return one artifact
- **AND** its module table SHALL list `input.nx` first and `drawnui` with version `9` second
- **AND** the artifact SHALL contain no declaration from `drawnui`

#### Scenario: A module nothing references is not in the table
- **WHEN** a workspace holds a module the entry never imports, directly or transitively
- **THEN** the entry's artifact SHALL NOT list that module

#### Scenario: A catalog emits as its own artifact
- **WHEN** the caller emits an artifact for `drawnui` alone
- **THEN** the artifact's module table SHALL hold only `drawnui`
- **AND** it SHALL carry every external component, union and record `drawnui` declares

### Requirement: NX IR modules are encoded as flat tables
A module SHALL be encoded as a string table, a type table, a constant table, a node table and a
declaration list. Node kinds, type kinds and declaration kinds SHALL be small integers; every name,
literal and type SHALL appear once in its table and be referenced by index; children SHALL be node
indices or index ranges. Slots SHALL be integers local to the declaration that owns them, and
element identities SHALL be integers local to the module. The same structure SHALL be expressible
in JSON, the encoding this version defines, and in a binary encoding without changing the tables.

#### Scenario: A name is written once
- **WHEN** a module uses the property name `Text` on forty elements
- **THEN** the emitted artifact SHALL contain the string `Text` once in its string table
- **AND** every use SHALL refer to it by index

#### Scenario: A type is written once
- **WHEN** twenty component props share the type nullable `string`
- **THEN** the type table SHALL contain nullable `string` once
- **AND** each prop SHALL refer to it by index

#### Scenario: Slots are declaration-local integers
- **WHEN** a function declares two parameters and a `let` inside its body
- **THEN** their slots SHALL be the integers `0`, `1` and `2` within that function
- **AND** another declaration SHALL be free to use the same integers

### Requirement: A conformance corpus defines NX IR behavior
The repository SHALL hold a corpus of NX programs, each with its expected artifact for every emitted
module and its expected evaluation results for every entrypoint the corpus names. The emitter's
tests SHALL check that each program emits its expected artifact, and every supported runtime SHALL
check that it evaluates each expected artifact to the expected results. The corpus SHALL cover every
node kind, type kind and declaration kind, cross-artifact references, and a program emitted without
its debug section. For every corpus program, the artifact emitted without a debug section SHALL be
at most six times the UTF-8 length of the program's own module source.

#### Scenario: Emitter output is pinned by the corpus
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **THEN** the emitter's corpus test SHALL fail naming the program and the difference

#### Scenario: A runtime is checked against the corpus
- **WHEN** a runtime evaluates a corpus artifact's named entrypoint
- **THEN** its canonical value SHALL equal the expected result recorded for it

#### Scenario: The size budget is enforced
- **WHEN** a corpus program of 8,000 bytes of source is emitted without a debug section
- **THEN** the artifact SHALL be at most 48,000 bytes

## MODIFIED Requirements

### Requirement: NX IR JSON program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR artifact emitted from a successful `ProgramArtifact`, one
per emitted module. The artifact SHALL include a format identifier, IR schema version `3`, expected
runtime ABI `nx-ir-runtime-v2`, required feature list, the module table, the module's public
entrypoints, and the module's tables and declarations. Equivalent `ProgramArtifact` inputs with
equivalent IR options SHALL produce byte-for-byte stable output apart from explicitly documented
formatting choices. A runtime SHALL refuse an artifact of an earlier schema version with a
diagnostic naming both versions rather than attempting to read it.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the artifact SHALL include schema version `3` and runtime ABI `nx-ir-runtime-v2`
- **AND** the artifact SHALL list `root` as a function entrypoint
- **AND** the module table's first entry SHALL carry the module's identity and fingerprint

#### Scenario: Equivalent inputs produce deterministic JSON
- **WHEN** two equivalent `ProgramArtifact` inputs are emitted as NX IR with the same options
- **THEN** the emitted artifacts SHALL use stable ordering for the module table, tables,
  declarations, fields, properties, entrypoints and nodes
- **AND** the two emitted artifacts SHALL be byte-identical

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests NX IR emission for a `ProgramArtifact` containing static error
  diagnostics
- **THEN** IR emission SHALL fail with diagnostics
- **AND** the system SHALL NOT emit a partial artifact

#### Scenario: An older schema is refused
- **WHEN** a runtime is given an artifact whose schema version is `2`
- **THEN** preparation SHALL fail with a diagnostic naming version `2` and the supported version `3`

### Requirement: NX IR program fingerprints are lossless for JavaScript consumers
Every fingerprint in an NX IR artifact, whether of the artifact's own module or of a module in its
table, SHALL be encoded in a form that represents the native fingerprint without JavaScript `number`
precision loss.

#### Scenario: Fingerprint exceeds JavaScript safe integer range
- **WHEN** an emitted module fingerprint is greater than JavaScript's maximum safe integer
- **THEN** the artifact SHALL encode it as a decimal string
- **AND** structured metadata returned with the generated IR SHALL expose the same fingerprint as a
  string or another explicitly lossless representation
- **AND** JavaScript consumers SHALL NOT need to parse the value as `number` to compare cache
  identity

### Requirement: NX IR preserves resolved module-qualified references
NX IR SHALL represent every reference to a declaration as a module-table slot and the declaration's
name, with slot `0` meaning the artifact's own module. Function calls, value references, component
descriptors, record construction, union cases, default expressions, selected entrypoints and nominal
type references SHALL all use this form, and no reference SHALL depend on the position of a
declaration in its module or of a module in a program. Top-level declaration names SHALL be unique
within a module, and the compiler SHALL reject a module that declares one name twice. Primitive type
references SHALL be encoded separately from nominal type references so runtimes do not resolve
records, unions or type aliases through global bare-name lookup.

#### Scenario: Imported function reference is module-qualified
- **WHEN** a root module imports function `answer()` from a resolved library module
- **AND** NX IR is emitted for the program
- **THEN** the call to `answer()` SHALL reference the library module's table slot and the name
  `answer`
- **AND** the TypeScript runtime SHALL resolve it through the linked module's declarations by name

#### Scenario: Imported component descriptor reference is module-qualified
- **WHEN** a root module constructs a component exported by another resolved module
- **AND** NX IR is emitted for the program
- **THEN** the component descriptor expression SHALL reference the component by that module's table
  slot and the component's name

#### Scenario: Imported nominal type reference is module-qualified
- **WHEN** a root module declares a function parameter or component prop using record, union, or
  type-alias `User` imported from another module
- **AND** another module in the same program also declares an item named `User`
- **THEN** the emitted type reference SHALL be nominal and SHALL name the imported module's slot and
  `User`
- **AND** supported runtimes SHALL normalize boundary values through that declaration rather than a
  bare `User` lookup

#### Scenario: A regenerated module keeps old references valid
- **WHEN** a snippet artifact references `SkiaLabel` in `drawnui`
- **AND** `drawnui` is regenerated with a new control declared before `SkiaLabel`
- **THEN** the snippet artifact, unchanged, SHALL still reference `SkiaLabel`
- **AND** a runtime linking it against the regenerated `drawnui` SHALL resolve `SkiaLabel`

#### Scenario: Duplicate top-level names are rejected
- **WHEN** a module declares `let Card = 1` and `type Card = { }`
- **THEN** analysis SHALL report an error naming `Card` and both declarations

### Requirement: NX IR preserves source provenance for diagnostics and source maps
An NX IR artifact SHALL be able to carry a debug section holding source spans for declarations and
nodes and the module's source text, and the section SHALL be optional: an artifact without it SHALL
prepare and evaluate exactly as one with it. The module's source fingerprint SHALL be present
whether or not the section is. A runtime diagnostic SHALL cite the span when the section is present
and the declaration name when it is not, and SHALL never need to read a source file from disk.

#### Scenario: Runtime diagnostic can identify source expression
- **WHEN** the TypeScript IR runtime reports a runtime diagnostic for an expression of an artifact
  that carries a debug section
- **THEN** the diagnostic SHALL identify the originating source identity and span

#### Scenario: A stripped artifact still diagnoses
- **WHEN** the same diagnostic arises in the same artifact emitted without a debug section
- **THEN** the diagnostic SHALL name the declaration the expression belongs to
- **AND** evaluation up to that point SHALL be identical to the artifact with the section

#### Scenario: Stripping is the emitter's choice
- **WHEN** a caller emits an artifact and asks for no debug section
- **THEN** the artifact SHALL contain no spans and no source text
- **AND** the artifact with and without the section SHALL differ only in that section
