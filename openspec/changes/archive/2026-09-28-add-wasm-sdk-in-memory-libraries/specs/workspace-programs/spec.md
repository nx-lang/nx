## MODIFIED Requirements

### Requirement: Workspace builds accept implicit imports
A workspace build or analysis SHALL accept a list of identities that every other module in the
workspace imports as if it began with a wildcard import of that identity. Each identity SHALL name
either a workspace module or the root of a library loaded into the build context's registry. An
implicitly imported module SHALL NOT import itself or another implicitly imported module
implicitly, and a module of an implicitly imported library SHALL NOT import that library or any
other implicit import implicitly. An implicit import SHALL behave exactly as the written wildcard
import would: the same names in scope, the same ambiguity diagnostics, and the same
module-qualified references in the built program. A module's own text SHALL be unchanged by the
option, so its diagnostics and positions are those of the text the caller supplied. Naming an
identity that is neither a workspace module nor a loaded library root SHALL fail the build with a
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

#### Scenario: A loaded library is imported implicitly
- **WHEN** a registry holds in-memory libraries `libraries/chat-link` and `libraries/question-flow`,
  where `chat-link` imports `../question-flow`
- **AND** a workspace holds `chat-link.nx` using `ChatLinkConfig` and `QuestionFlow` with no import
  line, built with implicit imports `libraries/chat-link` and `libraries/question-flow`
- **THEN** the build SHALL resolve both names to the libraries' declarations
- **AND** the built program SHALL equal the program built from the same text preceded by written
  wildcard imports of both libraries

#### Scenario: A workspace module and a library root collide
- **WHEN** an implicit-import identity names both a workspace module and exactly the root of a
  visible loaded library
- **THEN** the build SHALL fail with a diagnostic naming the ambiguous identity

#### Scenario: A directory library whose path ends in the identity does not collide
- **WHEN** an implicit-import identity `drawnui` names a workspace module
- **AND** a visible directory library's path ends in `/drawnui`
- **THEN** the implicit import SHALL resolve to the workspace module and the build SHALL succeed

## ADDED Requirements

### Requirement: Workspace validation is available as data through every SDK
Workspace validation SHALL be exposed through the wasm SDK and the Node SDK as a call that analyzes
a workspace against a build context, with the same implicit imports a build accepts, and returns
every diagnostic for the submitted modules — errors, warnings, info and hints — as data rather than
throwing. A linked in-memory library's own warnings, info and hints SHALL NOT be among them; its
errors SHALL. A directory-loaded library's diagnostics SHALL all be among them, as before.
Validation SHALL NOT create a program artifact. A workspace build that fails SHALL throw the same
diagnostics validation of the same input returns. The two SDKs SHALL return equal diagnostics for
the same inputs.

#### Scenario: Warnings are reported for a valid workspace
- **WHEN** a caller validates a workspace whose only diagnostics are warnings
- **THEN** validation SHALL return those warnings with severity, code, message and labels
- **AND** it SHALL NOT throw

#### Scenario: Errors are returned, not thrown
- **WHEN** a caller validates a workspace in which `chat-link.nx` has a type error
- **THEN** validation SHALL return the error diagnostic labelled against `chat-link.nx`
- **AND** it SHALL NOT throw

#### Scenario: SDKs agree
- **WHEN** the same workspace, libraries and implicit imports are validated through the wasm SDK and
  the Node SDK
- **THEN** both SHALL return equal diagnostics

#### Scenario: A library's warnings are not the workspace's
- **WHEN** a caller validates a workspace against a library loaded from memory whose own analysis
  reports a warning
- **THEN** validation SHALL NOT return that warning

#### Scenario: A directory library's warnings are still reported
- **WHEN** a caller validates a workspace that imports a library loaded from a directory whose own
  analysis reports a warning
- **THEN** validation SHALL return that warning, labelled against the library module's source text

#### Scenario: A failed build agrees with validation
- **WHEN** a caller validates a workspace with an error and then builds it
- **THEN** the build SHALL throw diagnostics equal to those validation returned
