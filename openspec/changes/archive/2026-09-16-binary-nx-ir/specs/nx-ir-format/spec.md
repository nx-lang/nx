## ADDED Requirements

### Requirement: An NX IR reader validates an artifact before it evaluates it
A reader SHALL establish, before answering any question about an artifact, that the image's header
is one it implements, that its recorded length equals the bytes it was given, that every section,
offset, string range, table index, module slot and optional operand the artifact contains lies
within the artifact's own bounds, and that every string is valid UTF-8. A malformed or truncated
artifact SHALL be refused with a diagnostic rather than an exception or a crash, SHALL NOT produce
a value, and SHALL NOT cause a read outside the artifact in any runtime.

#### Scenario: A truncated artifact is refused
- **WHEN** a runtime is given an image cut short at any four-byte boundary
- **THEN** preparation SHALL fail with a diagnostic
- **AND** no evaluation SHALL take place

#### Scenario: An out-of-range index is refused rather than followed
- **WHEN** an artifact's node names a string index beyond the string table
- **THEN** the reader SHALL report the artifact as malformed
- **AND** SHALL NOT read memory outside the artifact

#### Scenario: A hostile artifact cannot escape its bounds
- **WHEN** a host plays a share authored by someone else whose offsets have been altered
- **THEN** the runtime SHALL refuse it with a diagnostic rather than reading outside the artifact

#### Scenario: Every cell of an artifact can be damaged without a crash
- **WHEN** any single cell of a valid artifact is overwritten with an arbitrary value
- **THEN** every reader SHALL either refuse the artifact with a diagnostic or read it as a valid
  artifact
- **AND** no reader SHALL throw an exception or panic

## MODIFIED Requirements

### Requirement: NX IR JSON program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR artifact emitted from a successful `ProgramArtifact`, one
per emitted module. The artifact SHALL be a little-endian binary image whose fixed header carries a
format magic, IR schema version `3` and the image's total length, and whose sections carry the
expected runtime ABI `nx-ir-runtime-v2`, the required feature list, the module table, the module's
public entrypoints, and the module's tables and declarations. Equivalent `ProgramArtifact` inputs
with equivalent IR options SHALL produce byte-identical images. A reader SHALL refuse an image whose
magic or schema version it does not implement with a diagnostic naming the version found and the
version supported, and SHALL NOT interpret the bytes that follow the header.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the image SHALL carry schema version `3` and runtime ABI `nx-ir-runtime-v2`
- **AND** the image SHALL list `root` as a function entrypoint
- **AND** the module table's first entry SHALL carry the module's identity and fingerprint

#### Scenario: Equivalent inputs produce deterministic JSON
- **WHEN** two equivalent `ProgramArtifact` inputs are emitted as NX IR with the same options
- **THEN** the emitted images SHALL use stable ordering for the module table, tables,
  declarations, fields, properties, entrypoints and nodes
- **AND** the two emitted images SHALL be byte-identical

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests NX IR emission for a `ProgramArtifact` containing static error
  diagnostics
- **THEN** IR emission SHALL fail with diagnostics
- **AND** the system SHALL NOT emit a partial artifact

#### Scenario: An older schema is refused
- **WHEN** a runtime is given an image whose schema version is `2`
- **THEN** preparation SHALL fail with a diagnostic naming version `2` and the supported version `3`
- **AND** the reader SHALL NOT interpret the bytes that follow the header

#### Scenario: A document that is not an image is refused
- **WHEN** a runtime is given bytes that do not begin with the NX IR magic
- **THEN** preparation SHALL fail with a diagnostic saying the input is not an NX IR image

### Requirement: NX IR modules are encoded as flat tables
A module SHALL be encoded as a string table, a type table, a constant table, a node table and a
declaration list. Node kinds, type kinds and declaration kinds SHALL be small integers; every name,
literal and type SHALL appear once in its table and be referenced by index; children SHALL be node
indices or index ranges. Slots SHALL be integers local to the declaration that owns them, and
element identities SHALL be integers local to the module.

The string table SHALL be stored as an offset array over one UTF-8 blob, so a reader borrows a
name as a slice of the image and MAY decode only the names it resolves. Every other table SHALL be
stored as an offset array over a flat pool of 32-bit cells, so entry `k` is addressable by index
without reading the entries before it. Sections of the image SHALL be listed in a directory in its
header, and a reader SHALL skip a directory entry whose kind it does not know, so a section can be
added without a schema change.

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

#### Scenario: An entry is addressable without reading its predecessors
- **WHEN** a runtime resolves a declaration referenced by a linked module
- **THEN** it SHALL read that declaration's entry without decoding the entries before it

#### Scenario: A name is decoded only when it is named
- **WHEN** a module of five hundred strings is prepared and three of its declarations are resolved
- **THEN** the reader SHALL decode only the strings those resolutions name

#### Scenario: An unknown section is skipped
- **WHEN** an image's directory lists a section of a kind the reader does not know, beside every
  section it requires
- **THEN** the reader SHALL prepare the module from the sections it knows
- **AND** SHALL NOT refuse the artifact for the unknown section

### Requirement: A conformance corpus defines NX IR behavior
The repository SHALL hold a corpus of NX programs, each with its expected artifact for every emitted
module, the explained text of each expected artifact, and its expected evaluation results for every
entrypoint the corpus names. The emitter's tests SHALL check that each program emits its expected
artifact byte for byte, SHALL report a difference as explained text, and SHALL check that each
committed explained text is the explanation of its committed artifact. Every supported runtime SHALL
check that it evaluates each expected artifact to the expected results. One regeneration command
SHALL write the artifacts and their explained text together. The corpus SHALL cover every node
kind, type kind and declaration kind, cross-artifact references, and a program emitted without its
debug section. For every corpus program, the artifact emitted without a debug section SHALL be at
most six times the UTF-8 length of the program's own module source.

#### Scenario: Emitter output is pinned by the corpus
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **THEN** the emitter's corpus test SHALL fail naming the program and the difference as explained
  text

#### Scenario: A change to the emitter is readable in review
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **AND** the corpus is regenerated
- **THEN** both the image and its explained text SHALL be updated
- **AND** the explained text's diff SHALL name what changed about the program

#### Scenario: Stale explained text is caught
- **WHEN** a corpus image is committed without regenerating its explained text
- **THEN** the emitter's corpus test SHALL fail naming the program

#### Scenario: A runtime is checked against the corpus
- **WHEN** a runtime evaluates a corpus artifact's named entrypoint
- **THEN** its canonical value SHALL equal the expected result recorded for it

#### Scenario: The size budget is enforced
- **WHEN** a corpus program of 8,000 bytes of source is emitted without a debug section
- **THEN** the artifact SHALL be at most 48,000 bytes
