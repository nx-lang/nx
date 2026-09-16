## ADDED Requirements

### Requirement: TypeScript runtime links an entry module against prepared modules
The TypeScript runtime SHALL expose a linking step that takes a prepared entry module and a resolver
the host supplies, asks the resolver for each module in the entry's module table by identity, and
returns a linked program. Linking SHALL check that every resolved module's version equals the version
the entry recorded, SHALL fail naming the module and both versions when they differ unless the host
opts into linking across versions, SHALL fail naming the module and declaration when a referenced
declaration is absent from the resolved module, and SHALL let the host reuse one prepared module
across any number of linked programs.

#### Scenario: A snippet links against a prepared catalog
- **WHEN** a host has prepared the `drawnui` artifact once
- **AND** links a snippet artifact whose module table names `drawnui` with the same version
- **THEN** linking SHALL succeed
- **AND** evaluating the snippet's `root` SHALL construct descriptors of `drawnui`'s components

#### Scenario: A version mismatch is refused by default
- **WHEN** the snippet recorded `drawnui` version `9` and the resolver returns version `10`
- **THEN** linking SHALL fail with a diagnostic naming `drawnui`, `9` and `10`

#### Scenario: A host may link across versions
- **WHEN** the host opts into linking across versions for the same mismatch
- **AND** every declaration the snippet references exists in version `10`
- **THEN** linking SHALL succeed

#### Scenario: A missing declaration is refused
- **WHEN** the resolved `drawnui` does not declare a component the snippet references
- **THEN** linking SHALL fail with a diagnostic naming `drawnui` and the missing declaration

#### Scenario: A module the resolver cannot supply is refused
- **WHEN** the resolver returns nothing for a module in the table
- **THEN** linking SHALL fail with a diagnostic naming that module's identity

#### Scenario: One prepared module serves many programs
- **WHEN** a host links a hundred snippet artifacts against the same prepared `drawnui`
- **THEN** `drawnui` SHALL be prepared once
- **AND** each linked program SHALL evaluate independently

### Requirement: TypeScript runtime passes the conformance corpus
The TypeScript runtime's tests SHALL run every artifact in the conformance corpus and compare each
named entrypoint's canonical value against the corpus's expected result, including the artifacts
emitted without a debug section and those that link to another corpus artifact.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the runtime's tests run
- **THEN** every corpus artifact SHALL be prepared, linked where its module table requires, and
  evaluated
- **AND** a difference from an expected result SHALL fail the run naming the program and entrypoint

## MODIFIED Requirements

### Requirement: TypeScript runtime loads and prepares NX IR programs
The TypeScript runtime SHALL expose APIs that accept an NX IR artifact as JSON text or a parsed
object, validate the format identifier, IR schema version, runtime ABI, required features and
structural references, and return a prepared module. Preparation SHALL index the module's
declarations by name, resolve every reference to the module itself, build entrypoint tables,
precompute schema validators and default evaluators, and create efficient evaluators over the
module's node table. References to other modules SHALL be resolved by the linking step, and a
prepared module whose table names only itself SHALL be usable as a program directly.

#### Scenario: Supported IR prepares successfully
- **WHEN** a caller loads a valid NX IR artifact whose runtime ABI matches the TypeScript runtime
- **THEN** the runtime SHALL return a prepared module
- **AND** the prepared module SHALL expose function and component entrypoint lookup by public name
- **AND** declaration lookup SHALL be by module identity and declaration name

#### Scenario: A self-contained artifact is a program
- **WHEN** a caller prepares an artifact whose module table holds only its own module
- **THEN** the caller SHALL be able to evaluate its entrypoints without a linking step

#### Scenario: Unsupported runtime ABI is rejected
- **WHEN** a caller loads NX IR requiring a runtime ABI that the TypeScript runtime does not support
- **THEN** preparation SHALL fail with an actionable diagnostic
- **AND** the runtime SHALL NOT return a prepared module

#### Scenario: Unknown required feature is rejected
- **WHEN** a caller loads NX IR that declares a required feature unknown to the TypeScript runtime
- **THEN** preparation SHALL fail with an actionable diagnostic naming that feature

#### Scenario: Evaluating an unlinked module is rejected
- **WHEN** a caller evaluates an entrypoint of a prepared module whose table names another module
- **AND** the module has not been linked
- **THEN** evaluation SHALL fail with a diagnostic saying the module must be linked first
