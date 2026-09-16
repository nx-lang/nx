## MODIFIED Requirements

### Requirement: TypeScript runtime loads and prepares NX IR programs
The TypeScript runtime SHALL expose APIs that accept an NX IR artifact as bytes, validate the image
header, IR schema version, runtime ABI, required features and every structural reference before
reading anything else, and return a prepared module. Preparation SHALL read the image in place
through typed-array views rather than parsing a document, SHALL decode a string only when it is
named, SHALL index the module's declarations by name, resolve every reference to the module itself,
build entrypoint tables, precompute schema validators and default evaluators, and create efficient
evaluators over the module's node table. References to other modules SHALL be resolved by the
linking step, and a prepared module whose table names only itself SHALL be usable as a program
directly. A malformed image SHALL be reported through the runtime's diagnostic result, never as a
thrown exception from the non-throwing API.

#### Scenario: Supported IR prepares successfully
- **WHEN** a caller loads a valid NX IR image whose runtime ABI matches the TypeScript runtime
- **THEN** the runtime SHALL return a prepared module
- **AND** the prepared module SHALL expose function and component entrypoint lookup by public name
- **AND** declaration lookup SHALL be by module identity and declaration name

#### Scenario: Bytes are read without copying when aligned
- **WHEN** a caller passes an `ArrayBuffer`, or a byte view whose offset is a multiple of four
- **THEN** preparation SHALL read the image through views over those bytes
- **AND** SHALL NOT copy the image

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

#### Scenario: A malformed image is a diagnostic, not an exception
- **WHEN** a caller passes bytes that are truncated, or whose cells have been altered, to the
  non-throwing preparation API
- **THEN** the result SHALL carry a diagnostic identifying what is malformed
- **AND** the API SHALL NOT throw

#### Scenario: Evaluating an unlinked module is rejected
- **WHEN** a caller evaluates an entrypoint of a prepared module whose table names another module
- **AND** the module has not been linked
- **THEN** evaluation SHALL fail with a diagnostic saying the module must be linked first
