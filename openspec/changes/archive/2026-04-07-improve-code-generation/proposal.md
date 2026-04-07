## Why

NX already has a CLI path for generating TypeScript and C# types, but it only works on a single
source file and it emits all local records and enums regardless of visibility while omitting type
aliases. That falls short of the library model NX now uses, where directories are the unit of
import/export and `export` marks the intended external API surface.

## What Changes

- Extend `nxlang generate` so the input path is inferred by kind:
  - a `.nx` file triggers single-file generation
  - a directory triggers full-library generation, and the directory must resolve as a valid NX
    library root
- Generate code only for type declarations marked `export`.
- Generate code for a library by traversing all NX source files that belong to that library and
  emitting the library's full exported type surface.
- Generate library output as multiple files by default, using per-module output rather than one
  monolithic file for the entire library.
- Expand generated output to cover exported type aliases in addition to exported enums and exported
  record-style types.
- Keep one `generate` command for both workflows instead of introducing a separate `codegen` or
  `generate-library` command.
- Update diagnostics, CLI help text, and docs so users understand path inference, library
  validation, and export-only behavior.
- **BREAKING**: single-file generation stops emitting non-export declarations. Files that relied on
  internal or private type generation will produce less output unless those declarations are marked
  `export`.

## Capabilities

### New Capabilities

- `cli-code-generation`: Generate TypeScript or C# type definitions from either a single NX source
  file or a full NX library directory while honoring NX export visibility.

### Modified Capabilities

- `declaration-visibility`: Generated code treats `export` as the only externally visible type
  surface for both file and library code generation.

## Impact

- Affected code: `crates/nx-cli/src/main.rs`, `crates/nx-cli/src/codegen/*`, and any shared NX API
  helpers needed to enumerate analyzed library modules for code generation.
- Affected behavior: `nxlang generate` path handling, generated output contents, diagnostics, and
  help/documentation examples.
- Affected tests: new CLI integration coverage for file-vs-directory inference, export filtering,
  and multi-file library generation for both TypeScript and C#.
