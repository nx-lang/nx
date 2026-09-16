## ADDED Requirements

### Requirement: Workspace builds accept implicit imports
A workspace build or analysis SHALL accept a list of workspace identities that every other module in
the workspace imports as if it began with a wildcard import of that identity. An implicitly imported
module SHALL NOT import itself or another implicitly imported module implicitly. An implicit import
SHALL behave exactly as the written wildcard import would: the same names in scope, the same
ambiguity diagnostics, and the same module-qualified references in the built program. A module's
own text SHALL be unchanged by the option, so its diagnostics and positions are those of the text
the caller supplied. Naming an identity the workspace does not contain SHALL fail the build with a
diagnostic naming it.

#### Scenario: Controls are used without an import line
- **WHEN** a workspace holds `drawnui` declaring `SkiaLabel` and `input.nx` containing
  `<SkiaLabel Text="hi" />`, with `drawnui` implicitly imported
- **THEN** the build SHALL resolve `SkiaLabel` to `drawnui`'s declaration
- **AND** `input.nx` SHALL compile without an import statement

#### Scenario: Positions are the document's own
- **WHEN** `input.nx` has an error on its third line
- **THEN** the diagnostic SHALL name `input.nx` and line 3

#### Scenario: A single trailing element compiles
- **WHEN** `input.nx` declares no `root` and consists of one element expression naming a `drawnui`
  control
- **THEN** the build SHALL accept it as it would the same text with a written wildcard import

#### Scenario: The implicit module does not import itself
- **WHEN** `drawnui` is implicitly imported and is itself analyzed
- **THEN** `drawnui` SHALL be analyzed with no implicit import
- **AND** no cycle diagnostic SHALL be reported

#### Scenario: An unknown implicit import is refused
- **WHEN** the implicit-import list names `missing`
- **AND** the workspace holds no module with that identity
- **THEN** the build SHALL fail with a diagnostic naming `missing`

#### Scenario: The wasm and native SDKs expose implicit imports
- **WHEN** a caller builds a workspace through the wasm SDK, the Node SDK or the C FFI
- **THEN** each SHALL accept the implicit-import list and produce the same program as the Rust API

### Requirement: Workspace modules carry an optional version
A workspace module SHALL accept an optional version string alongside its identity and source. NX
SHALL NOT interpret the string; an empty string SHALL be the same as no version. Every NX IR artifact
emitted from a program SHALL record, for each module in its module table, the version that module
was given when the program was built, and emit options SHALL NOT carry versions. Two builds that
differ only in a module's version SHALL produce programs with different fingerprints.

#### Scenario: Every artifact of a build agrees on a version
- **WHEN** a workspace holds `drawnui` with version `9` and `input.nx`, with `drawnui` implicitly
  imported
- **AND** the caller emits artifacts for both modules
- **THEN** `drawnui`'s artifact SHALL record itself with version `9`
- **AND** `input.nx`'s artifact SHALL record `drawnui` with version `9` and itself with an empty
  version

#### Scenario: A version is not an emit option
- **WHEN** a caller passes emit options naming `versions`
- **THEN** emission SHALL be refused rather than ignoring the key

#### Scenario: A version changes the program's fingerprint
- **WHEN** the same workspace is built with `drawnui` at version `9` and again at version `10`
- **THEN** the two programs SHALL have different fingerprints
- **AND** building it at version `9` twice SHALL give the same fingerprint

#### Scenario: Every SDK takes the version on the module
- **WHEN** a caller builds a versioned workspace through the wasm SDK, the Node SDK, the C FFI or the
  .NET SDK
- **THEN** each SHALL accept the version on the workspace module
- **AND** the wasm and Node SDKs SHALL emit byte-identical artifacts for it
