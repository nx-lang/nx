## Why

The current executable TypeScript component shape mixes strongly typed generated APIs with JSON
boundary validation, and exposes component entry classes that feel unlike normal TypeScript or
React-adjacent component code. ReachMe needs generated TypeScript that is pleasant for typed
callers while still preserving explicit JSON validation/conversion at host boundaries.

## What Changes

- **BREAKING**: Replace generated component entry classes as the primary TypeScript surface with
  strongly typed component functions and explicit `Props` types.
- **BREAKING**: Treat normal NX components more like render functions in generated TypeScript:
  calling/constructing a normal component evaluates its body, while external components produce
  serializable external element values for client UI handoff.
- Add generated `Schema`-suffixed runtime metadata/adapters for JSON validation, normalization,
  diagnostics, and serialization support.
- Keep `NxValue` and JSON-compatible input types out of the primary typed TypeScript component API
  except where a value is intentionally untyped or externally serialized.
- Generate `Element`-suffixed TypeScript types for external component values, e.g.
  `TextInputElement`, and use unsuffixed external component functions as typed factories for those
  element values.
- Share schema-driven JSON normalization logic through runtime helpers where practical instead of
  emitting custom handwritten validation code for every generated prop and state field.
- Preserve explicit host/boundary APIs for initialization/evaluation, but move them behind schema
  or runtime adapter surfaces rather than making them the default component API.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `executable-code-generation`: Refine generated TypeScript component APIs to separate typed
  component usage from JSON boundary adapters, introduce `Props`/`Schema`/`Element` generated
  naming, and change component expression generation so only external components produce
  serializable external element values.
- `external-components`: Clarify that generated external component values are external elements
  intended for host/client rendering, with `Element`-suffixed TypeScript types and schema-backed
  JSON boundary support.

## Impact

- Updates `crates/nx-codegen` TypeScript emission for component declarations, component
  expressions, state handling, runtime helper imports, and generated tests.
- Updates generated runtime helper support for schema-driven JSON validation/conversion and result
  diagnostics.
- Requires migration of existing component-codegen tests that assert class-based `SearchBox`
  output and descriptor-first behavior for non-external child components.
- Leaves JavaScript executable output behavior compatible with the same runtime semantics, while
  TypeScript receives the stronger typed surface.
- Does not change `nxlang typegen` DTO-only output unless a later change intentionally aligns that
  surface with executable component API naming.
