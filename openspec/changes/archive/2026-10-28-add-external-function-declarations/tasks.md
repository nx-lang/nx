## 1. Syntax and trusted binding

- [x] 1.1 Add explicit bodyless external function syntax, regenerate the parser and update highlighting; verify normal/invalid/external declaration parser tests.
- [x] 1.2 Bind external declarations through closed module metadata during lowering; verify missing/duplicate/unregistered/signature mismatch diagnostics and preservation of docs/source locations.

## 2. Math API and editor integration

- [x] 2.1 Move all 22 math signatures and individual function/constant documentation into math.nx and remove declaration synthesis; verify registry and source agree and existing runtime calls still work.
- [x] 2.2 Verify imported math hover/completion documentation, aliases, selective imports and real declaration spans with language-service/API tests.

## 3. Documentation and conformance

- [x] 3.1 Document external declarations, restrictions and math documentation authoring in NX references; compile executable examples and build/link-check the website.
- [x] 3.2 Refresh math-linked IR fixtures and rerun parser, HIR, API, editor, Rust runtime/codegen and TypeScript conformance; verify math-free fixture bytes remain unchanged and strict OpenSpec/diff checks pass.

## 4. Bare external declaration spelling

- [x] 4.1 Replace external let with external throughout live syntax, diagnostics, math source, editor grammars, references and specs; verify both declaration forms and rejection of the incorrect form.
- [x] 4.2 Refresh math fixtures, run affected compiler/editor/runtime/documentation checks, and obtain an independent Claude CLI review with any high-confidence fixes verified.
