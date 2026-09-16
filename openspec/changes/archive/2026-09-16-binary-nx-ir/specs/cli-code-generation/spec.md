## MODIFIED Requirements

### Requirement: CLI explains an NX IR artifact as readable text
Because an NX IR artifact is a binary image, `nxlang ir explain <artifact>` SHALL be the supported
way to read one. The command SHALL read an image and print it as text in which every table index is
replaced by what it names: declarations by name and kind, references by module identity and
declaration name, types spelled as NX would spell them, nodes as an indented expression tree, and
spans as line and column when the debug section is present. The output SHALL be deterministic for a
given artifact. The command SHALL fail with a diagnostic and a failure exit code on an artifact of
an unsupported schema version and on a malformed artifact, and SHALL never panic on any input.

#### Scenario: A snippet artifact reads as its source would
- **WHEN** a user runs `nxlang ir explain input.nxir` on the artifact of a program whose root
  returns `<SkiaLabel Text="hi" />`
- **THEN** the output SHALL show a function `root` whose body is a component descriptor of
  `drawnui`'s `SkiaLabel` with property `Text` equal to the string `"hi"`
- **AND** no table index SHALL appear in the output

#### Scenario: Debug data is shown when present
- **WHEN** the artifact carries a debug section
- **THEN** each declaration in the output SHALL be annotated with its source line and column

#### Scenario: An unsupported artifact is refused
- **WHEN** the artifact's schema version is not the one the CLI supports
- **THEN** the command SHALL print a diagnostic naming both versions and exit with a failure code

#### Scenario: A malformed artifact is reported, not fatal
- **WHEN** the artifact is truncated or one of its indices is out of range
- **THEN** the command SHALL print a diagnostic identifying what is malformed
- **AND** it SHALL exit with a failure code rather than panicking

### Requirement: CLI codegen writes NX IR JSON artifacts
The `nxlang codegen` command SHALL support an explicit `nx-ir` executable target that builds a
`ProgramArtifact` through the existing source/workspace analysis pipeline and writes one
deterministic NX IR image per module of the program, each carrying its debug section, as
`<identity>.nxir`. NX IR output SHALL be host-neutral and SHALL NOT require a JavaScript or
TypeScript source-generation target. Each artifact's file name SHALL derive from its module
identity.

#### Scenario: Source file codegen writes NX IR
- **WHEN** a user runs `nxlang codegen ./app/main.nx --target nx-ir --output ./generated`
- **THEN** the CLI SHALL build a `ProgramArtifact` for `app/main.nx`
- **AND** it SHALL write `main.nxir` under `./generated`
- **AND** it SHALL NOT write JavaScript runtime helper files or generated JavaScript modules for
  that request

#### Scenario: Workspace codegen writes NX IR for selected entry
- **WHEN** a user runs `nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated`
- **THEN** the CLI SHALL build a workspace `ProgramArtifact` using `app/main.nx` as the selected
  entry identity
- **AND** it SHALL write one NX IR image for each module of the workspace program, `app/main.nx`
  included

#### Scenario: Written artifacts carry debug data
- **WHEN** the CLI writes an NX IR image
- **THEN** the image SHALL contain the debug section with spans and source text
- **AND** `nxlang ir explain` on that file SHALL annotate each declaration with line and column

#### Scenario: Static diagnostics prevent IR output
- **WHEN** a user requests `--target nx-ir` for a source file that has static analysis errors
- **THEN** the CLI SHALL print diagnostics through the existing diagnostic rendering path
- **AND** it SHALL NOT write an NX IR artifact

#### Scenario: NX IR target rejects source output formats
- **WHEN** a user requests `--target nx-ir --format program-module`
- **THEN** the CLI SHALL report that NX IR codegen does not use source output formats
- **AND** it SHALL NOT write an NX IR artifact
