## ADDED Requirements

### Requirement: JavaScript program module output is emitted from ProgramArtifact
The system SHALL provide an `nx-codegen` API that emits a cacheable, host-neutral JavaScript
program module from a successful `ProgramArtifact`. The output SHALL contain one JavaScript ESM
source module plus structured metadata identifying the program fingerprint, expected NX JavaScript
runtime ABI, runtime import specifier, logical module name, exported function entrypoints, and
exported component/schema entrypoints. Code generation MUST fail with diagnostics instead of
emitting a program module when the input artifact contains error diagnostics or when required
semantic data is unavailable.

#### Scenario: Valid artifact emits one program module
- **WHEN** a caller builds a valid `ProgramArtifact` for an NX source file with `root()`
- **AND** the caller requests JavaScript program module output
- **THEN** `nx-codegen` SHALL return exactly one JavaScript ESM source module for the NX program
- **AND** the output metadata SHALL identify the artifact fingerprint
- **AND** the output metadata SHALL list `root` as an exported function entrypoint

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests JavaScript program module output for a `ProgramArtifact` with static
  error diagnostics
- **THEN** `nx-codegen` SHALL return diagnostics
- **AND** `nx-codegen` SHALL NOT emit JavaScript program module source

### Requirement: Program modules depend on a separately supplied NX runtime
Generated JavaScript program modules SHALL import shared NX runtime helpers from a runtime module
specifier instead of embedding the runtime helper source. The default runtime module specifier
SHALL be stable and host-neutral. Callers MAY override the runtime module specifier in codegen
options so local tests or specific isolate hosts can use a concrete module name.

#### Scenario: Runtime helper source is not embedded
- **WHEN** a supported component program requires generated schema helper functions
- **AND** the caller requests JavaScript program module output
- **THEN** the generated program module SHALL import the required runtime helpers from the
  configured runtime module specifier
- **AND** the generated program module SHALL NOT include the full NX runtime helper implementation

#### Scenario: Runtime specifier can be configured
- **WHEN** a caller requests JavaScript program module output with runtime module specifier
  `./nx-runtime.js`
- **THEN** the generated program module SHALL use `./nx-runtime.js` for its runtime import
- **AND** the output metadata SHALL report `./nx-runtime.js` as the runtime import specifier

### Requirement: Program modules are host-neutral
Generated JavaScript program modules SHALL represent only NX program behavior and MUST NOT emit
isolate host wrappers, HTTP handlers, Cloudflare Worker exports, Cloudflare Dynamic Worker
manifests, Rivet actor or registry setup, database access, auth policy, logging policy, resource
limits, CommonJS `require`, or Node built-in imports. Host-specific wrappers and isolate manifests
MUST be supplied outside the cached NX program module.

#### Scenario: Program module omits host entrypoints
- **WHEN** a caller requests JavaScript program module output for a valid NX program
- **THEN** the generated module SHALL export NX entrypoints and component/schema values as ESM
  exports
- **AND** the generated module SHALL NOT contain a default Worker export, `fetch` handler, Rivet
  `actor` definition, Rivet `setup` call, or Cloudflare Dynamic Worker `WorkerCode` object

#### Scenario: Program module omits host platform imports
- **WHEN** a caller requests JavaScript program module output for a valid NX program
- **THEN** the generated module SHALL NOT import from Node built-in modules
- **AND** the generated module SHALL NOT use CommonJS `require`
- **AND** every static import in the generated module SHALL target the configured NX runtime module
  specifier

### Requirement: Program module output preserves resolved cross-module behavior
JavaScript program module output SHALL preserve the behavior of resolved cross-module NX programs
without emitting relative imports between generated NX source modules. References to declarations
from different runtime modules SHALL be emitted as deterministic, collision-free local generated
names inside the single program module.

#### Scenario: Cross-module function import is flattened
- **WHEN** a `ProgramArtifact` contains a root module that imports a supported function or value
  from another resolved module
- **AND** the caller requests JavaScript program module output
- **THEN** the generated program module SHALL contain executable code for the imported declaration
  and the root entrypoint in the same ESM source module
- **AND** executing the generated root entrypoint with the supplied NX runtime SHALL produce the
  same JSON-compatible value as interpreter evaluation
- **AND** the generated program module SHALL NOT import another generated NX module by relative
  path

#### Scenario: Cross-module component schema import is flattened
- **WHEN** a `ProgramArtifact` contains a component that constructs or references a concrete
  component declared in another resolved module
- **AND** the caller requests JavaScript program module output
- **THEN** generated component descriptor functions and schema values SHALL remain coherent inside
  the single generated program module
- **AND** executing the generated component entry API with the supplied NX runtime SHALL preserve
  descriptor and evaluation behavior equivalent to existing JavaScript file output

### Requirement: Program module output is deterministic and cache-addressable
JavaScript program module output SHALL be deterministic for equivalent `ProgramArtifact` inputs
and equivalent program-module codegen options. The generated metadata SHALL include enough
information for callers to compute a database cache key and validate runtime compatibility without
parsing generated source.

#### Scenario: Equivalent inputs produce equivalent program modules
- **WHEN** two equivalent `ProgramArtifact` inputs are generated with the same JavaScript program
  module options
- **THEN** the emitted source, logical module name, runtime import specifier, runtime ABI,
  exported entrypoint metadata, and exported component/schema metadata SHALL be stable across
  repeated runs

#### Scenario: Runtime ABI is exposed for cache validation
- **WHEN** a caller receives JavaScript program module output
- **THEN** the output metadata SHALL include the NX JavaScript runtime ABI expected by the
  generated source
- **AND** an isolate host SHALL be able to compare that ABI with the separately supplied runtime
  before loading the generated program module

### Requirement: Existing executable file output remains supported
JavaScript program module output SHALL be an additional output style and MUST NOT replace the
existing executable TypeScript/JavaScript file output behavior. Existing file output SHALL continue
to emit the local runtime helper file, generated module files, and index file unless the caller
explicitly requests the program-module output style.

#### Scenario: Default JavaScript file output is unchanged
- **WHEN** a caller requests existing JavaScript executable file output for a valid NX program
- **THEN** `nx-codegen` SHALL emit the existing generated file set including `nx-runtime.js`,
  generated module files, and `index.js`
- **AND** the generated file set SHALL remain executable in a standard ESM host without requiring
  the program-module host composition workflow

### Requirement: CLI executable codegen exposes an explicit output format
The `nxlang codegen` command SHALL provide an explicit executable output format selector that
defaults to the existing file layout and can opt into JavaScript program-module output. CLI
program-module output SHALL be JavaScript-only in this change and SHALL NOT write a local runtime
helper file or generated index barrel.

#### Scenario: CLI default format remains file output
- **WHEN** a caller runs `nxlang codegen` for JavaScript without specifying an output format
- **THEN** the CLI SHALL write the existing file-output layout
- **AND** the output directory SHALL include `nx-runtime.js`, generated module files, and
  `index.js`

#### Scenario: CLI writes one program module when requested
- **WHEN** a caller runs `nxlang codegen --target javascript --format program-module`
- **THEN** the CLI SHALL write one JavaScript program module source file to the output directory
- **AND** that source SHALL import runtime helpers from the default host-neutral runtime specifier
- **AND** the output directory SHALL NOT include `nx-runtime.js`, generated module files, or
  `index.js`

#### Scenario: CLI rejects TypeScript program-module output
- **WHEN** a caller runs `nxlang codegen --target typescript --format program-module`
- **THEN** the CLI SHALL fail with a clear error
- **AND** the CLI SHALL NOT emit TypeScript program-module output

### Requirement: .NET hosts can invoke program-module codegen from ProgramArtifact
The .NET runtime package SHALL expose an artifact-first API that invokes host-neutral JavaScript
program-module generation for an `NxProgramArtifact`. The API SHALL return source text plus
structured metadata for logical module name, runtime import specifier, runtime ABI, program
fingerprint, function entrypoint exports, and component/schema exports. Invalid or unsupported
artifacts SHALL surface diagnostics through the managed diagnostic exception path.

#### Scenario: Managed artifact emits program module metadata
- **WHEN** a .NET caller builds a valid `NxProgramArtifact`
- **AND** the caller requests program-module output with default options
- **THEN** the managed API SHALL return JavaScript source text for one program module
- **AND** the returned metadata SHALL include the default logical module name, default runtime
  import specifier, runtime ABI, program fingerprint, and exported `root` entrypoint

#### Scenario: Managed artifact honors configured runtime specifier
- **WHEN** a .NET caller requests program-module output with runtime module specifier
  `./nx-runtime.js`
- **THEN** the generated source SHALL import runtime helpers from `./nx-runtime.js`
- **AND** the returned metadata SHALL report `./nx-runtime.js` as the runtime import specifier

#### Scenario: Managed artifact surfaces codegen diagnostics
- **WHEN** a .NET caller requests program-module output for an artifact that cannot be emitted
- **THEN** the managed API SHALL throw an `NxEvaluationException`
- **AND** the exception SHALL contain the native codegen diagnostics
