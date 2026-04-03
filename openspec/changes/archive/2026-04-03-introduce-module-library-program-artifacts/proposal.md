## Why

NX currently overloads `Module` and `PreparedLibrary` in ways that blur important boundaries: a
`Module` means one lowered source file in some parts of the system, while prepared-library APIs
flatten many files into one merged runtime world. That makes the terminology harder to reason
about, weakens cache granularity, and blocks a clean artifact model for per-file, per-library, and
whole-program analysis.

## What Changes

- **BREAKING** Rename the single-file lowered HIR type from `Module` to `LoweredModule` across the
  compiler, runtime, FFI, and .NET surface area.
- Introduce `ModuleArtifact` as the cached artifact container for one source file, holding the
  products of parsing, lowering, analysis, diagnostics, and invalidation metadata for that module.
- Introduce `LibraryArtifact` as the cached artifact container for one library, holding its
  `ModuleArtifact`s, export metadata, dependency metadata, and library-level diagnostics.
- Introduce `ProgramArtifact` as the cached artifact container for a fully resolved executable
  program, covering standalone source files plus all dependent libraries.
- Replace merged prepared-library architecture with file-preserving program/library artifacts so
  source files remain separate after lowering and analysis rather than being copied into one merged
  lowered module.
- Update runtime architecture so interpreter execution is defined in terms of a resolved
  program-level runtime view built from separate lowered modules, rather than relying on a single
  flattened module namespace.
- Update public terminology, documentation, tests, and diagnostics to consistently use the new
  artifact vocabulary.

## Capabilities

### New Capabilities
- `artifact-model`: Defines `ModuleArtifact`, `LibraryArtifact`, and `ProgramArtifact`, including
  what cached products they hold and how they relate to one another.
- `resolved-program-runtime`: Defines the runtime-facing resolved program structure consumed by the
  interpreter when executing across multiple lowered modules without flattening them into one merged
  module.

### Modified Capabilities
- `source-analysis-pipeline`: The shared analysis model needs to describe single-file
  `LoweredModule` output and module-level artifacts instead of treating the lowered result as a
  generically named `Module`.
- `module-imports`: Import resolution needs to define how module and library artifacts preserve
  separate lowered source files while still exposing library exports and dependency metadata.
- `component-runtime-bindings`: Runtime bindings need to describe execution against the new
  resolved-program/program-artifact model rather than a single merged lowered module plus prepared
  libraries.

## Impact

- Affected code spans `nx-hir`, `nx-types`, `nx-api`, `nx-interpreter`, `nx-ffi`, and
  `bindings/dotnet`.
- Public APIs, internal type names, tests, and documentation will see **BREAKING** terminology and
  architecture changes.
- Prepared-library evaluation and component lifecycle flows will need to migrate to the new
  artifact/runtime model.
- Caching, invalidation, and runtime snapshot handling will need to become module-aware rather than
  assuming one flat lowered-module namespace.
