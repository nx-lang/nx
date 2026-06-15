## Why

NX can already parse, lower, type-check, and interpret resolved programs, and it can generate
TypeScript/C# DTO surfaces for exported data contracts. It does not yet have a reusable backend
model for emitting executable program code, which blocks running NX-authored UI/config logic as
ordinary JavaScript on both clients and servers.

## What Changes

- Add a new `nx-codegen` Rust crate that builds a backend-facing `CodegenProgram` from an existing
  `ProgramArtifact`.
- Define `CodegenProgram` as a resolved, typed, source-mapped semantic program for code generation,
  not as a JavaScript-specific AST or executable bytecode.
- Add initial TypeScript and JavaScript source emission for non-reactive NX programs that produce
  serializable `NxValue` output.
- Add a small JavaScript/TypeScript runtime helper surface used by generated output for
  element-like records, runtime errors, and function calls, while emitting ordinary arrays,
  records, and enum
  values directly as plain JavaScript data.
- Emit strongly typed TypeScript surfaces for NX records and enums using structural record types
  with per-property `readonly` modifiers, `as const` enum value objects with
  `typeof Enum[keyof typeof Enum]` derived types, and direct object/string construction instead of
  record/enum helper calls.
- Add CLI/API entry points for `nxlang codegen` to generate executable TypeScript or JavaScript
  from analyzed NX source/workspace inputs.
- Replace the types-only CLI surface `nxlang generate` with `nxlang typegen`, preserving its role for
  interacting with NX-authored contracts without executing NX-authored behavior.
- Keep functional reactivity out of the initial implementation; reactive dependency tracking and
  component lifecycle/action-handler runtime helpers will be added in a later change.

## Capabilities

### New Capabilities
- `executable-code-generation`: Building a backend-facing `CodegenProgram` from `ProgramArtifact`
  inputs and emitting executable TypeScript/JavaScript source for non-reactive NX programs.

### Modified Capabilities
- `cli-code-generation`: Rename the types-only generation command from `nxlang generate` to
  `nxlang typegen`.

## Impact

- Adds a new workspace crate: `crates/nx-codegen`.
- Extends CLI/API surfaces to expose executable TypeScript/JavaScript generation through
  `nxlang codegen` and the types-only DTO/type generation surface through `nxlang typegen`.
- Introduces generated source/runtime helper tests that compare emitted program behavior with the
  existing interpreter for supported non-reactive scenarios, comparing object values without
  treating object property order as semantic.
- Defers component lifecycle/action-handler generation, reactive semantics, framework-specific
  adapters, optimized bytecode, and bundler-specific packaging to later changes.
