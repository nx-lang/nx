## Why

The `editor-language-service` spec already describes hover and completions that resolve against the
cursor's semantic surroundings. The implementation does not: it resolves positions with line-prefix
string scanning and a top-level symbol list, so it fails the spec's own scenarios on ordinary NX.

Hover is specified to answer "at positions where NX can identify a declaration, **reference**, type
annotation, component tag, property, or **expression** with known metadata"
(`openspec/specs/editor-language-service/spec.md`). `WorkspaceSnapshot::hover`
(`crates/nx-language-service/src/lib.rs:316`) instead matches the offset against the *selection
ranges of top-level document symbols* and returns `format!("{} `{}`", kind, name)`. Nothing else in
a document has a hover, and where one exists it carries no type.

Completions are specified for "inside an element opening tag" and "immediately after `=` in a
property value position". Both context probes — `component_property_context`
(`lib.rs:1563`) and `property_value_context` (`lib.rs:1500`) — begin with
`line_prefix.rfind('<')?` and give up when `<` is not on the cursor's own line. A multi-line opening
tag is the style `AGENTS.md` prescribes and `examples/nx/` and the DrawnUI catalog use throughout,
so the specified behavior is absent exactly where NX is normally written. `is_type_position`
(`lib.rs:1647`) is single-line for the same reason.

Measured against the shipped `nx-lsp` binary, with `type Mode = light | dark` and
`let <Panel mode:Mode="light" title:string caption:string />`:

| Request | Position | Result today |
| --- | --- | --- |
| Property-name completion | `<Panel ⟨cursor⟩ />` on one line | `mode`, `title`, `caption` |
| Property-name completion | same tag written across lines | `import`, `from`, `as`, `export`, … |
| Property-value completion | `<Panel mode=⟨cursor⟩ />` on one line | `light`, `dark` |
| Property-value completion | same tag written across lines | `import`, `from`, `as`, `export`, … |
| Hover | declaration name `Panel` | ``component `Panel` `` — kind only, no signature |
| Hover | tag reference `<Panel` | `null` |
| Hover | parameter `x` in a function body | `null` |

The analysis needed to fix this is already computed and discarded on every request. `ModuleArtifact`
(`crates/nx-types/src/check.rs:21`) carries both `lowered_module` — an expression arena whose every
`Expr` has a `TextSpan` (`crates/nx-hir/src/ast/expr.rs:328`) — and `type_env`, a complete
`ExprId → Type` map (`crates/nx-types/src/env.rs:130`). `build_workspace_declarations`
(`lib.rs:508`) walks those artifacts for names and kinds and keeps neither the arena nor the types.

## What Changes

- **Resolve editor positions against the lowered module, not the line prefix.** Introduce a single
  position-resolution step that maps a byte offset to the innermost enclosing semantic entity — an
  `ExprId`, a declaration, a component tag, a property name, a property value, or a type
  annotation — from the lowered module and its CST, and make hover and completions both consume it.
  Line-prefix scanning stops being how context is determined.
- **Hover reports the inferred type.** Where the resolved entity has an entry in the module's
  `TypeEnvironment`, hover reports that type. This makes hover answer on references, parameters,
  properties, and expressions, which today return nothing.
- **Hover on a declaration reports its signature.** The existing scenario asks for "the symbol kind
  and available signature or type information"; today only the kind is returned.
- **Completion context is derived from the resolved position.** Property-name, property-value, and
  type-annotation contexts hold for multi-line opening tags and multi-line signatures. Supplied
  properties are collected from the whole opening tag rather than from the text following the tag
  name on one line (`supplied_properties`, `lib.rs:1592`), so the "SHALL NOT include properties
  already supplied" clause holds across lines too.
- **Hover stays conservative.** The existing "no useful metadata means no hover" scenario is
  preserved: an unresolved position returns no hover rather than a fabricated one.

Explicitly out of scope, to keep this change inside `nx-language-service`:

- Loading workspace documents from disk. The snapshot remains whatever documents the client submits.
- Exposing the language service through `nx-ffi` or the Node SDK, and any drawnui-fiddle wiring.
- New LSP methods. `nx-lsp`'s advertised capabilities are unchanged; its existing hover and
  completion handlers return better answers.
- Go-to-definition, references, rename, semantic tokens, and inlay hints. The position resolver this
  change introduces is what they would later build on, but none are added here.

## Capabilities

### New Capabilities
<!-- None. Position resolution is how the existing hover and completion requirements are met, not a
     separately observable capability. -->

### Modified Capabilities
- `editor-language-service`: The hover requirement gains the obligation to report inferred types and
  declaration signatures at resolved positions. The hover and completion requirements gain scenarios
  fixing their behavior at multi-line opening tags and signatures, and on references, parameters,
  and expressions. The conservative-hover and import-visibility guarantees are unchanged.

## Impact

- `crates/nx-language-service/src/lib.rs` — `WorkspaceSnapshot::hover` and
  `WorkspaceSnapshot::completions`; the `property_value_context` / `component_property_context` /
  `is_type_position` / `supplied_properties` line-scanning helpers are replaced by the resolver;
  `WorkspaceDeclarations` gains the per-module data (lowered module and type environment) the
  resolver reads, which `build_workspace_declarations` already has in hand and drops.
- `crates/nx-types` and `crates/nx-hir` — read-only consumers. `ModuleArtifact::type_env` and the
  expression arena are already public; no new analysis is introduced. Any gap found here is expected
  to be an accessor, not a change to inference.
- `crates/nx-lsp/src/lib.rs` — no protocol change. Its delegation tests assert richer hover content.
- Editor-visible behavior changes in both hosts that consume the service: the VS Code extension
  immediately, and drawnui-fiddle once a later change gives it a transport.
- Performance: the resolver reads artifacts the snapshot already builds under its existing
  `OnceLock`. Snapshot construction cost per request is unchanged and out of scope here.
- Sequencing: this change is planned to start after `empty-list-spelling` completes. The two do not
  overlap in code, but `empty-list-spelling` alters the primitive set and completion keyword list,
  which the completion scenarios here read.
