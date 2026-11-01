## ADDED Requirements

### Requirement: nx-program owns program building and the resolved-program model
The `nx-program` crate SHALL own workspaces, source graphs, library and standard-library loading,
`ProgramArtifact` construction, the resolved-program types, binding-friendly diagnostics, schema
queries and program visualizer inspection, and SHALL NOT depend on `nx-interpreter`, `nx-codegen`,
`nx-language-service`, `nx-ir-runtime` or `nx-api`.

#### Scenario: Resolved types have one definition
- **WHEN** `nx-interpreter`, `nx-codegen` or `nx-api` refers to `ResolvedProgram`,
  `ResolvedItemKind` or `RuntimeModuleId`
- **THEN** it SHALL use the type defined in `nx-program`
- **AND** `nx-interpreter` SHALL NOT define or re-export those types

#### Scenario: Codegen and the language service do not need the interpreter
- **WHEN** `nx-codegen` or `nx-language-service` is built
- **THEN** neither SHALL have a normal dependency on `nx-interpreter` or `nx-api`

### Requirement: nx-api is the external Rust entry point
The `nx-api` crate SHALL be the Rust entry point for external adapters, depending on `nx-program`,
`nx-codegen`, `nx-language-service`, `nx-interpreter` and `nx-ir-runtime`, re-exporting the
foundation types flat and the services as the `codegen`, `language_service` and `ir_runtime`
namespaces.

#### Scenario: An external caller builds, emits and runs a program through nx-api
- **WHEN** a crate depending only on `nx-api` builds a `ProgramArtifact`, emits NX IR through
  `nx_api::codegen` and evaluates it through `nx_api::ir_runtime`
- **THEN** the program SHALL build, emit and run without a direct dependency on another NX crate

#### Scenario: Adapters use nx-api for service calls
- **WHEN** the CLI, the C/.NET FFI crate, the Node binding, the WASM binding or the native graphics
  preview calls code generation, the language service or the IR runtime
- **THEN** it SHALL reach that service through `nx-api`
- **AND** `nx-lsp` MAY depend on `nx-language-service` directly as that service's protocol adapter

### Requirement: Lower crates never depend on nx-api
No crate that `nx-api` depends on, directly or transitively, SHALL depend on `nx-api` through a
normal, build or dev dependency, and the repository's normal checks SHALL fail when such an edge or
an edge between `nx-program` and `nx-ir-runtime` is added.

#### Scenario: A test-only upward edge is rejected
- **WHEN** a developer adds `nx-api` as a dev-dependency of `nx-codegen` to run a test
- **THEN** the repository's dependency check SHALL fail and name the forbidden edge

#### Scenario: The IR runtime stays independent of the compiler
- **WHEN** the dependency check runs
- **THEN** it SHALL fail if `nx-ir-runtime` depends on `nx-program` or `nx-program` depends on
  `nx-ir-runtime`
