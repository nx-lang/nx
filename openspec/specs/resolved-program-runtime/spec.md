# resolved-program-runtime Specification

## Purpose
Defines the Rust IR runtime-facing resolved program model and the module-qualified runtime reference
rules required for multi-module execution and serialized runtime state.

## Requirements
### Requirement: Runtime references are module-qualified
Runtime-owned executable references SHALL identify the owning IR module and exact local declaration
or node. Resolved source declaration identities SHALL be preserved during compilation rather than
rediscovered through ambiguous global name lookup.

#### Scenario: Function references identify the owning lowered module and exact local definition
- **WHEN** functions or handlers originate in an imported module
- **THEN** the emitted/runtime references SHALL preserve that owning module and local declaration or node

#### Scenario: Imported component references do not require a name rescan at execution time
- **WHEN** a host initializes a component exported from an imported module through an IR runtime
- **THEN** execution SHALL use the exact resolved component in that module's linked image
- **AND** SHALL NOT rescan by a globally visible component name

#### Scenario: Captured handler references identify the owning lowered module
- **WHEN** a handler captures an expression from `search-box.nx`
- **THEN** its emitted/runtime reference SHALL retain that module identity and the exact local IR node

### Requirement: Serialized runtime state is program-specific and module-aware
Stored IR runtime component instances and captured handlers SHALL encode module-qualified IR
references and identify the exact linked images that produced them. Restoration SHALL reject a
mismatched program revision or invalid references. The removed interpreter snapshot binding SHALL
NOT be replaced with a new opaque binding transport.

#### Scenario: Snapshot from the same program revision can be reused
- **WHEN** a host stores and restores an instance using the existing IR runtime storage API against the same images
- **THEN** later dispatch SHALL preserve its state and module-qualified handlers

#### Scenario: Snapshot from a different program revision is rejected
- **WHEN** a host restores an instance against different linked images
- **THEN** restoration SHALL fail without silently reinterpreting local IDs

### Requirement: Workspace program evaluation uses entry-module scoped root
Executing IR compiled from a workspace SHALL run `root()` from the workspace's selected entry
module and SHALL NOT select another module's root from a global symbol table.

#### Scenario: Two workspace modules can define root
- **WHEN** a workspace contains `a.nx` returning `"a"` and `b.nx` returning `"b"` from root
- **AND** the caller compiles with entry identity `b.nx` and executes the linked IR
- **THEN** the result SHALL be `"b"`

#### Scenario: Entry module without root reports that entry module
- **WHEN** the selected entry lacks root but another module defines it
- **THEN** CLI execution SHALL report the missing entrypoint for the selected module
- **AND** SHALL NOT execute the other module's root

### Requirement: Workspace entrypoints are module-qualified in resolved programs
Resolved workspace programs SHALL preserve selected entry-module identity and resolved declaration
references for IR compilation. Execution SHALL resolve the selected entry through linked IR rather
than execute a lowered module directly.

#### Scenario: Workspace root lookup uses selected runtime module reference
- **WHEN** a workspace selects `app/main.nx` and it lowers successfully
- **THEN** emission and linking SHALL preserve that module as the entry
- **AND** execution SHALL use its IR declaration rather than an HIR execution path

### Requirement: Native execution is defined by the program's emitted IR
Native NX execution SHALL evaluate linked NX IR, not execute `ResolvedProgram` directly.
`ResolvedProgram` SHALL remain the compiler's resolved view across lowered modules.

#### Scenario: Root calls an imported function
- **WHEN** source root calls a function imported from another module
- **THEN** native execution SHALL call that module's linked image without merging lowered modules
