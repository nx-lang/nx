## ADDED Requirements

### Requirement: Prepared modules separate raw definitions from visible bindings
The system SHALL represent prepared analysis state separately from the raw file-local
`LoweredModule`. A prepared module SHALL preserve the analyzing file's raw definitions and SHALL
also expose a prepared visible namespace for the names visible from that file. Constructing that
visible namespace SHALL NOT require foreign peer or imported definitions to be copied into the raw
stored module artifact.

#### Scenario: Same-library peer visibility does not require cloning peer HIR into the raw module
- **WHEN** `helpers.nx` in one library declares `let answer() = 42`
- **AND** `main.nx` in that same library references `answer()`
- **THEN** prepared analysis for `main.nx` SHALL make `answer` visible from `helpers.nx`
- **AND** the stored raw `LoweredModule` for `main.nx` SHALL remain a file-local artifact for
  `main.nx` only

#### Scenario: Imported visibility uses interface metadata rather than foreign raw HIR
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and `let root() = Button`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` library that exports `Button`
- **THEN** prepared analysis for `app/main.nx` SHALL make `Button` visible through imported library
  interface metadata
- **AND** the stored raw `LoweredModule` for `app/main.nx` SHALL NOT require copied raw HIR items
  from `../ui`

### Requirement: Visible names resolve to stable binding targets
Each prepared visible top-level name SHALL resolve to a stable binding target that identifies the
kind of symbol and the owning definition or interface origin. The prepared binding model SHALL
distinguish between local definitions, same-library peer definitions, and imported library
interface definitions.

#### Scenario: Local declaration resolves to a local binding target
- **WHEN** a file declares `type Theme = string`
- **THEN** prepared analysis for that file SHALL resolve `Theme` to a local type binding target
- **AND** that binding target SHALL identify the owning definition within the raw file-local module

#### Scenario: Imported declaration resolves to an imported binding target
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and references `Button`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` that exports `Button`
- **THEN** prepared analysis for `app/main.nx` SHALL resolve `Button` to an imported binding target
- **AND** that binding target SHALL identify the owning imported library interface definition

### Requirement: Lexical scopes layer over prepared top-level bindings
The system SHALL resolve lexical bindings such as parameters, `let` bindings, and loop bindings
before falling back to prepared top-level visible bindings. A local lexical binding SHALL shadow a
prepared top-level binding with the same name.

#### Scenario: Function parameter shadows an imported top-level name
- **WHEN** a file imports `../ui` that exports `Button`
- **AND** the file declares `let render(Button:string) = Button`
- **THEN** the function body SHALL resolve `Button` to the parameter binding
- **AND** the imported top-level `Button` SHALL remain available only when not shadowed lexically

#### Scenario: Undefined lexical name falls back to prepared top-level visibility
- **WHEN** `helpers.nx` in one library declares `let answer() = 42`
- **AND** `main.nx` in that same library declares `let root() = answer()`
- **THEN** `root()` SHALL resolve `answer` through the prepared top-level binding visible from
  `helpers.nx`

### Requirement: Program and runtime lookup preserve module-qualified definition identity
When the system builds a `ProgramArtifact` and `ResolvedProgram`, runtime-visible item lookup SHALL
preserve the owning module identity and the stable local definition identity of the target
declaration. Runtime lookup SHALL NOT need to rediscover the target declaration by rescanning the
owning module for the visible string name.

#### Scenario: Imported function lookup preserves the exact owning definition
- **WHEN** a root source file imports `../math` and calls exported function `answer()`
- **THEN** the resulting `ResolvedProgram` SHALL record a module-qualified reference to the exact
  `answer` definition in the owning imported module
- **AND** runtime execution SHALL use that module-qualified definition reference to execute the call

#### Scenario: Runtime entry lookup preserves the exact local root definition
- **WHEN** a root source file defines `let root() = 42`
- **THEN** the resulting `ResolvedProgram` SHALL record a module-qualified reference to the exact
  local `root` definition in that file
- **AND** runtime execution SHALL use that module-qualified definition reference rather than
  rediscovering `root` by name in the module at execution time
