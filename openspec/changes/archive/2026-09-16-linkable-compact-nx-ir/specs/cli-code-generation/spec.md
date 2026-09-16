## ADDED Requirements

### Requirement: CLI explains an NX IR artifact as readable text
The `nxlang ir explain <artifact>` command SHALL read an NX IR artifact and print it as text in
which every table index is replaced by what it names: declarations by name and kind, references by
module identity and declaration name, types spelled as NX would spell them, nodes as an indented
expression tree, and spans as line and column when the debug section is present. The output SHALL
be deterministic for a given artifact, and the command SHALL fail with a diagnostic on an artifact
of an unsupported schema version.

#### Scenario: A snippet artifact reads as its source would
- **WHEN** a user runs `nxlang ir explain input.nxir.json` on the artifact of a program whose root
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

## MODIFIED Requirements

### Requirement: CLI codegen writes NX IR JSON artifacts
The `nxlang codegen` command SHALL support an explicit `nx-ir` executable target that builds a
`ProgramArtifact` through the existing source/workspace analysis pipeline and writes one
deterministic, pretty-printed NX IR artifact per module of the program, each carrying its debug
section. NX IR output SHALL be host-neutral and SHALL NOT require a JavaScript or TypeScript
source-generation target. Each artifact's file name SHALL derive from its module identity.

#### Scenario: Source file codegen writes NX IR
- **WHEN** a user runs `nxlang codegen ./app/main.nx --target nx-ir --output ./generated`
- **THEN** the CLI SHALL build a `ProgramArtifact` for `app/main.nx`
- **AND** it SHALL write one NX IR artifact under `./generated` for `main.nx`
- **AND** it SHALL NOT write JavaScript runtime helper files or generated JavaScript modules for
  that request

#### Scenario: Workspace codegen writes NX IR for selected entry
- **WHEN** a user runs `nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated`
- **THEN** the CLI SHALL build a workspace `ProgramArtifact` using `app/main.nx` as the selected
  entry identity
- **AND** it SHALL write one NX IR artifact for each module of the workspace program, `app/main.nx`
  included

#### Scenario: Written artifacts carry debug data
- **WHEN** the CLI writes an NX IR artifact
- **THEN** the artifact SHALL contain the debug section with spans and source text

#### Scenario: Static diagnostics prevent IR output
- **WHEN** a user requests `--target nx-ir` for a source file that has static analysis errors
- **THEN** the CLI SHALL print diagnostics through the existing diagnostic rendering path
- **AND** it SHALL NOT write an NX IR artifact

#### Scenario: NX IR target rejects source output formats
- **WHEN** a user requests `--target nx-ir --format program-module`
- **THEN** the CLI SHALL report that NX IR codegen does not use source output formats
- **AND** it SHALL NOT write an NX IR artifact
