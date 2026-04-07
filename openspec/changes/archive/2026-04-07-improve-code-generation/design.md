## Context

`nxlang generate` currently lowers exactly one source file and emits generated code from the local
`LoweredModule`. That implementation is intentionally small, but it now diverges from the rest of
NX in three important ways:

- NX import and visibility semantics are library-oriented, while code generation is still
  file-oriented.
- Code generation currently ignores declaration visibility and therefore emits private and internal
  declarations that are not part of a library's external API surface.
- The generator only covers enums and record-like declarations, even though NX now has exported
  type aliases and library-wide type surfaces that hosts may want to consume.

This change crosses the CLI, codegen model, and analysis/library-loading layers. It also introduces
different output shapes for single-file and library generation, which benefits from explicit design
decisions before implementation.

## Goals / Non-Goals

**Goals:**
- Keep one public CLI verb, `nxlang generate`, for both file and library generation.
- Infer generation mode from the input path kind: `.nx` file versus directory.
- Generate only exported type declarations.
- Support generated output for exported type aliases, exported enums, and exported record-like
  declarations, including exported action records.
- Reuse NX's library analysis path so library generation sees the same module set and visibility
  semantics as imports and runtime builds.
- Produce multi-file output for library generation using a stable per-module layout.

**Non-Goals:**
- Introducing a separate `codegen`, `generate-library`, or manifest-driven packaging command.
- Generating code for functions, components, or runtime-only declarations.
- Changing library root semantics beyond validating the provided directory as a loadable NX library.
- Designing all future output layouts up front; this change only defines the default layout.
- Adding language-specific build or compile validation of generated files in consumer projects.

## Decisions

### 1. Keep one `generate` command and infer the mode from the input path

`nxlang generate <input>` will inspect the filesystem entry at `<input>`:

- if it is a file with `.nx` extension, run the single-file generation path
- if it is a directory, run the library generation path
- otherwise, report an error

This keeps the CLI intuitive and matches the language model NX already uses, where files are source
units and directories are libraries.

Alternative considered:
- Add a separate `generate-library` subcommand or an explicit `--library` flag.

Why rejected:
- The mode is derivable from the input path, and splitting the surface would make the common case
  harder to discover without adding meaningful capability.

### 2. Single-file generation remains single-file, but it becomes export-only

For `.nx` inputs, the generator will continue to emit a single output unit and preserve current
stdout behavior when `--output` is omitted. The difference is that it will collect only exported
type declarations from that file.

Alternative considered:
- Preserve current behavior for file generation and apply export filtering only to directory input.

Why rejected:
- That would make the same source declarations generate a different public type surface depending on
  whether the user points at a file or its enclosing library.

### 3. Library generation reuses registry-backed analysis rather than bespoke directory walking

Library generation will load the input directory through the same library analysis path used by
NX's program-building APIs. The generator should work from analyzed library/module artifacts rather
than building its own recursive file-discovery rules.

This gives code generation the same answers as the rest of NX for:
- what counts as a library
- which `.nx` files belong to that library
- how declarations are filtered by visibility
- how diagnostics are reported for invalid library contents

Alternative considered:
- Recursively walk the directory in `nx-cli` and lower each file independently.

Why rejected:
- That duplicates library-root semantics, risks drifting from import behavior, and forces CLI code
  to reimplement analysis and diagnostic policy.

### 4. Introduce a language-neutral exported type graph before emission

The current `collect_exported_types` model is too narrow for library generation because it only
walks one `LoweredModule` and only tracks records and enums. The generator will instead build a
language-neutral exported type graph that can represent:

- owning module identity and relative output path
- exported type aliases
- exported enums
- exported record-like declarations, including action records
- cross-module type references between exported declarations

Emitters then consume that richer graph to produce either one file or many files.

Alternative considered:
- Extend the current per-module collector incrementally without introducing a new shared model.

Why rejected:
- Library output needs module ownership and reference metadata that would otherwise leak into both
  emitters in ad hoc ways.

### 5. Library generation uses per-module multi-file output by default

When the input is a directory, generation will produce multiple files under an output directory:

- one generated file per source module that contributes exported types
- stable relative paths derived from the module's relative path within the library

Language-specific conventions will differ slightly:

- TypeScript: one `.ts` file per NX module plus a root `index.ts` that re-exports generated module
  files
- C#: one `.g.cs` file per NX module, with all generated files using the requested root namespace

Alternative considered:
- Generate one monolithic file for the entire library.

Why rejected:
- It scales poorly, produces noisy diffs, and does not match the module-oriented structure of the
  source library.

Alternative considered:
- Generate one file per type.

Why rejected:
- That is noisier than necessary for TypeScript and adds more output-management complexity than the
  current change needs.

### 6. Library generation requires an output directory

Single-file generation can still write to stdout or to a file. Library generation cannot sensibly
stream a multi-file result to stdout, so directory input will require `--output` and that output
path must be treated as a directory root.

Alternative considered:
- Invent a default output directory when `--output` is omitted.

Why rejected:
- Silent filesystem writes are harder to reason about and make CLI behavior less explicit.

### 7. Export visibility defines the generated public surface

Code generation will treat `export` as the only external API surface. Declarations with default
internal visibility or `private` visibility will not be emitted, even during file generation.

Alternative considered:
- Emit internal declarations for file input because they are still visible within the same library.

Why rejected:
- Generated code is intended as an external host-facing contract. Internal declarations are not part
  of that contract.

## Risks / Trade-offs

- [Library generation adds a more complex output mode] -> Mitigation: keep the CLI surface small,
  require an explicit output directory, and model both modes through one shared exported type graph.
- [TypeScript multi-file output needs cross-file imports] -> Mitigation: build owner-module metadata
  into the exported type graph and emit `import type` statements from that graph rather than from
  string heuristics.
- [Export-only filtering changes current single-file output] -> Mitigation: document the behavior
  clearly in CLI help and docs, and add integration tests that make the new contract explicit.
- [Library analysis may surface diagnostics users did not previously see during code generation] ->
  Mitigation: report the same library-analysis diagnostics the rest of NX already uses so failures
  stay consistent across commands.

## Migration Plan

1. Update the CLI contract and docs so `generate` is documented as file-or-directory aware.
2. Replace the current single-module export collector with a library-capable exported type graph
   that honors `export` visibility and includes type aliases.
3. Add single-file export filtering and library-generation output modes in `nx-cli`.
4. Add tests for input-kind inference, export filtering, alias generation, and library multi-file
   layout for both TypeScript and C#.
5. Update examples and the .NET README to remove the stale `codegen` command name and reflect the
   new output expectations.

## Open Questions

No blocking questions. A future change may add optional output layouts such as `single-file` or
`per-type` for library generation, but this proposal intentionally defines only the default
per-module layout.
