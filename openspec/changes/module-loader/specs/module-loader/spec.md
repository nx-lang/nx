## ADDED Requirements

### Requirement: Shared module-loading primitives live in `nx-module`
The system SHALL define the shared module-loading primitives in a dedicated `nx-module` crate. That crate SHALL own `ModuleId`, `ModuleSource`, `ModuleLoadError`, `ModuleLoader`, `DirectoryModuleLoader`, and `ModuleLoaderList` so parse-time and runtime consumers can use the same abstractions without depending on `nx-api`.

#### Scenario: Lower-level consumer uses `nx-module` directly
- **WHEN** a crate such as `nx-cli`, `nx-interpreter`, or `nx-ffi` needs to construct or consume module loaders
- **THEN** it SHALL use the types from `nx-module` rather than requiring `nx-api` to own those types

### Requirement: ModuleId uniquely identifies a module
The system SHALL represent each resolved module with a `ModuleId` that uniquely identifies it. For directory-based loaders, the `ModuleId` SHALL be the canonicalized absolute file path.

#### Scenario: Two references to the same file produce equal ModuleIds
- **WHEN** two module paths resolve to the same file through different path spellings
- **THEN** the returned `ModuleId` values SHALL be equal

#### Scenario: Different files produce different ModuleIds
- **WHEN** two module paths resolve to different files
- **THEN** the returned `ModuleId` values SHALL NOT be equal

### Requirement: ModuleLoader trait provides resolve and load operations
The `ModuleLoader` trait SHALL define two methods: `resolve`, which takes an optional importing `ModuleId` and a module path string and returns a `Result<Option<ModuleId>, ModuleLoadError>`, and `load`, which takes a `ModuleId` and returns a `Result<ModuleSource, ModuleLoadError>`. The `resolve` method returns `Ok(Some(id))` on success, `Ok(None)` when the module is not found, and `Err(ModuleLoadError)` for hard failures (case mismatch, I/O error, permission denied). `ModuleLoadError` keeps loader failures at the domain level so higher layers can add source context before producing public diagnostics.

#### Scenario: Resolve returns ModuleId for existing module
- **WHEN** `resolve(None, "utils")` is called and a file `utils.nx` exists in the search path
- **THEN** the loader SHALL return `Ok(Some(ModuleId))` containing the canonical path

#### Scenario: Resolve returns None for missing module
- **WHEN** `resolve(None, "nonexistent")` is called and no matching file exists
- **THEN** the loader SHALL return `Ok(None)`

#### Scenario: Load returns source for valid ModuleId
- **WHEN** `load(module_id)` is called with a `ModuleId` from a successful `resolve`
- **THEN** the loader SHALL return a `ModuleSource` containing the file contents and file name

#### Scenario: Load returns error for invalid ModuleId
- **WHEN** `load(module_id)` is called with a `ModuleId` whose file has been deleted since resolve
- **THEN** the loader SHALL return an `Err(ModuleLoadError)` describing the I/O failure

### Requirement: ModuleSource contains source text and file name
`ModuleSource` SHALL contain a `source` field with the module's full source text (String) and a `file_name` field with a display name for diagnostics.

#### Scenario: ModuleSource from directory loader
- **WHEN** a module is loaded from `/project/libs/utils.nx`
- **THEN** `source` SHALL contain the file contents and `file_name` SHALL be `"utils.nx"`

### Requirement: DirectoryModuleLoader resolves from a base directory
`DirectoryModuleLoader` SHALL resolve module paths relative to a configured base directory. It SHALL implement the `ModuleLoader` trait.

#### Scenario: Resolve with .nx extension
- **WHEN** the base directory is `/project/libs` and `resolve(None, "math.nx")` is called
- **THEN** the loader SHALL check for `/project/libs/math.nx` and return a `ModuleId` if it exists

#### Scenario: Resolve without .nx extension appends it
- **WHEN** the base directory is `/project/libs` and `resolve(None, "math")` is called
- **THEN** the loader SHALL check for `/project/libs/math.nx` and return a `ModuleId` if it exists

#### Scenario: Resolve with directory module pattern
- **WHEN** the base directory is `/project/libs`, `resolve(None, "utils")` is called, `/project/libs/utils.nx` does not exist, but `/project/libs/utils/mod.nx` does
- **THEN** the loader SHALL return a `ModuleId` for `/project/libs/utils/mod.nx`

#### Scenario: Relative paths resolve from importing module's directory
- **WHEN** `main_module_id` refers to `/project/src/main.nx`, `resolve(Some(main_module_id), "./helpers")` is called, and `/project/src/helpers.nx` exists
- **THEN** the loader SHALL resolve `./helpers` relative to `/project/src/`, not the loader's base directory

### Requirement: Case-sensitive path resolution
Module path resolution SHALL be case-sensitive on all platforms. After resolving a file on a case-insensitive filesystem, the loader SHALL compare the canonical path against the requested path components. If the case differs, the loader SHALL return an error describing the mismatch and the correct casing.

#### Scenario: Exact case match succeeds
- **WHEN** `resolve(None, "./utils")` is called and the file on disk is named `utils.nx`
- **THEN** the loader SHALL return `Ok(Some(ModuleId))` successfully

#### Scenario: Case mismatch on case-insensitive filesystem produces error
- **WHEN** `resolve(None, "./Utils")` is called on a case-insensitive filesystem and the file on disk is named `utils.nx`
- **THEN** the loader SHALL return `Err(ModuleLoadError)` describing the case mismatch and the correct casing

#### Scenario: Case mismatch in directory component
- **WHEN** `resolve(None, "./Lib/helpers")` is called and the directory on disk is named `lib/`
- **THEN** the loader SHALL return `Err(ModuleLoadError)` describing the directory case mismatch and the correct casing

### Requirement: ModuleLoaderList composes multiple loaders
`ModuleLoaderList` SHALL hold an ordered list of `ModuleLoader` implementations and SHALL itself implement `ModuleLoader`. Resolution SHALL try each loader in order and return the first successful result. If any loader returns `Err(ModuleLoadError)`, the error SHALL propagate immediately without trying further loaders.

#### Scenario: First matching loader wins
- **WHEN** the list contains loaders for `/path/a` and `/path/b`, and `resolve(None, "foo")` matches in `/path/b` only
- **THEN** the list SHALL return the `ModuleId` from the `/path/b` loader

#### Scenario: No loader matches
- **WHEN** no loader in the list can resolve `"nonexistent"`
- **THEN** the list's `resolve` SHALL return `Ok(None)`

#### Scenario: Empty loader list
- **WHEN** the list contains no loaders and `resolve(None, "anything")` is called
- **THEN** the list SHALL return `Ok(None)`

#### Scenario: Error from a loader short-circuits
- **WHEN** the first loader returns `Err(ModuleLoadError)` for a case mismatch and a second loader could resolve the path
- **THEN** the list SHALL return the `Err(ModuleLoadError)` immediately without trying the second loader

### Requirement: Module load errors are converted into diagnostics at the API boundary
The multi-file evaluation pipeline SHALL convert `ModuleLoadError` values into `nx_diagnostics::Diagnostic`, and `nx-api` SHALL convert those diagnostics into `NxDiagnostic` for public API and FFI responses.

#### Scenario: Case mismatch becomes a public diagnostic
- **WHEN** import resolution fails because `resolve` returns a case-mismatch `ModuleLoadError`
- **THEN** the public API SHALL return an `NxDiagnostic` with code `"module-case-mismatch"` and a helpful message indicating the correct casing

#### Scenario: I/O failure becomes a public diagnostic
- **WHEN** module loading fails because `load` returns an I/O-related `ModuleLoadError`
- **THEN** the public API SHALL return an `NxDiagnostic` with code `"module-io-error"` describing the failure

### Requirement: CLI accepts module search paths
The `nxlang run` command SHALL accept a repeatable `--module-path` flag specifying directories to search for modules.

#### Scenario: Single module path
- **WHEN** `nxlang run main.nx --module-path ./libs` is executed
- **THEN** import resolution SHALL search `./libs` for modules

#### Scenario: Multiple module paths
- **WHEN** `nxlang run main.nx --module-path ./libs --module-path ./vendor` is executed
- **THEN** import resolution SHALL search `./libs` first, then `./vendor`

#### Scenario: Implicit file-relative path
- **WHEN** `nxlang run /project/src/main.nx` is executed with no `--module-path` flags
- **THEN** import resolution SHALL still resolve relative imports (`./foo`) from `/project/src/`

### Requirement: FFI accepts module search paths
The C FFI SHALL provide evaluation functions that accept a list of directory paths for module resolution, in addition to the existing source and file name parameters.

#### Scenario: FFI with module paths
- **WHEN** the FFI is called with source containing `import "./utils"` and a module path list including a directory containing `utils.nx`
- **THEN** the import SHALL resolve and the module SHALL be loaded successfully

#### Scenario: FFI with no module paths
- **WHEN** the FFI is called with a null paths pointer and zero count
- **THEN** import resolution SHALL use an empty loader list (no modules can be resolved)

### Requirement: .NET binding accepts module search paths
The `NxRuntime` .NET evaluation methods SHALL accept an optional list of module search directory paths.

#### Scenario: .NET evaluation with module paths
- **WHEN** `NxRuntime.EvaluateToJson(source, modulePaths: ["/libs"])` is called
- **THEN** the module paths SHALL be forwarded to the native FFI for import resolution

#### Scenario: .NET evaluation without module paths
- **WHEN** `NxRuntime.EvaluateToJson(source)` is called with no module paths
- **THEN** import resolution SHALL use an empty loader list
