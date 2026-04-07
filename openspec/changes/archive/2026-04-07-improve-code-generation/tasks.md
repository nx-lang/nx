## 1. CLI Contract And Input Handling

- [x] 1.1 Update `nxlang generate` to infer file-versus-directory generation from the input path and reject unsupported inputs.
- [x] 1.2 Require `--output` as a directory root for library generation while preserving stdout or file output for single-file generation.
- [x] 1.3 Route directory inputs through the existing NX library analysis path and surface library-validation diagnostics through the CLI.

## 2. Exported Type Graph

- [x] 2.1 Replace the current single-module collector with a language-neutral exported type graph that tracks owner modules and export visibility.
- [x] 2.2 Extend the exported type graph to cover exported type aliases, exported enums, and exported record-like declarations including action records.
- [x] 2.3 Build library-level exported type graphs from analyzed library modules using stable relative module paths for output layout.

## 3. Language Emitters And Output Layout

- [x] 3.1 Update TypeScript emission to generate exported aliases, per-module files, cross-module `import type` statements, and a root `index.ts` barrel for library output.
- [x] 3.2 Update C# emission to generate exported aliases and per-module `.g.cs` files that remain resolvable across the generated library output.
- [x] 3.3 Add output-writing logic for both single-file and multi-file generation modes, including stable per-module path mapping.

## 4. Verification And Documentation

- [x] 4.1 Add unit tests for export filtering, alias generation, action-record generation, and exported type graph behavior.
- [x] 4.2 Add CLI integration tests for file-versus-directory inference, required output directories for library generation, invalid library inputs, and multi-file TypeScript/C# library output.
- [x] 4.3 Update CLI help text and repository documentation, including the stale `.NET` README example, to describe `generate`, export-only behavior, and library generation output.
