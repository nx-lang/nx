## Why

Action declarations are record-compatible today, but they cannot participate in the inheritance
model that normal records already support. That forces repeated payload fields across related
actions and blocks shared abstract action contracts from flowing through handlers, emit metadata,
and generated type surfaces.

Inline emitted actions already surface as public action record names such as
`SearchBox.ValueChanged`. If top-level actions gain inheritance while inline emitted actions do
not, component-scoped actions become an inconsistent special case that still forces duplicated
payload fields in common component event families.

## What Changes

- Allow action declarations to participate in single-base inheritance, including abstract action
  bases and derived action records.
- Extend action syntax so action declarations can use `abstract` and `extends` consistently with
  record declarations while preserving action identity.
- Allow inline component emitted action definitions to use `extends` so component-scoped emitted
  actions can reuse abstract action payload contracts while remaining concrete emitted actions.
- Define inherited-field, subtype, and instantiation rules for action inheritance so derived
  actions behave like inherited records without becoming plain records.
- Update generated TypeScript and C# action surfaces so exported inherited actions preserve base
  fields, concrete discriminators, and resolvable cross-module references.
- Add parser, analysis, type-checking, and code generation coverage for abstract and concrete
  action inheritance chains.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `action-records`: Action declarations can be abstract, can extend abstract action bases, and
  remain action-typed through inherited record behavior, including synthesized inline emitted
  actions.
- `component-syntax`: Inline emitted action definitions can use `extends` while keeping the same
  public `<Component>.<Action>` naming and handler-binding model.
- `record-type-inheritance`: Inheritance rules recognize abstract action bases and concrete derived
  actions without allowing plain record and action hierarchies to mix incorrectly.
- `cli-code-generation`: Exported action inheritance generates coherent TypeScript and C# contracts
  with inherited fields and concrete discriminator values.

## Impact

- `crates/nx-syntax` grammar, AST helpers, validation, parser fixtures, and syntax tests for
  top-level `abstract action`, `action ... extends ...`, and inline emitted `ActionName extends Base { ... }`
- `crates/nx-hir` and prepared binding/model code that classify inherited action declarations and
  synthesized inline emitted action records
- `crates/nx-types` and runtime consumers that validate action compatibility, field inheritance,
  handler inputs, and component-scoped inherited emitted actions
- `crates/nx-cli/src/codegen.rs` plus language-specific generators and tests for inherited action
  output
