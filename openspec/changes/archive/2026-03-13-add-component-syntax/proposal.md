## Why

NX currently models reusable UI declarations through `let <.../> = ...`, which leaves no syntax for component-specific concepts like emitted actions and persistent local state. Adding a dedicated `component` declaration lets the grammar represent those concepts directly now, so later lowering and interpreter work can build on a stable parsed form.

## What Changes

- Add a new `component` declaration form for element-shaped definitions.
- Add an optional `emits` group inside a component signature for publicly emitted action shapes.
- Add an optional `state` group inside a component body for component-local persistent state fields.
- Treat `component`, `emits`, and `state` as reserved keywords in the grammar.
- Limit this change to parsing, syntax tree shape, highlighting, and parser-facing documentation.
- Defer HIR lowering, type lowering, interpreter/runtime behavior, and semantic validation to later changes.

## Capabilities

### New Capabilities
- `component-syntax`: Parse `component` declarations with props, optional `emits` definitions, and optional `state` definitions.

### Modified Capabilities

## Impact

- `crates/nx-syntax` grammar, generated parser output, syntax kinds, and parser tests
- Tree-sitter highlighting and any parser-facing queries that classify declaration keywords
- Grammar/reference documentation that describes component declarations
- Example `.nx` inputs and fixtures that should cover the new syntax
