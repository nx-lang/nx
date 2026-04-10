## Why

Action declarations are record-compatible today, but they cannot participate in the inheritance
model that normal records already support. That forces repeated payload fields across related
actions and blocks shared abstract action contracts from flowing through handlers, emit metadata,
and generated type surfaces.

## What Changes

- Allow action declarations to participate in single-base inheritance, including abstract action
  bases and derived action records.
- Extend action syntax so action declarations can use `abstract` and `extends` consistently with
  record declarations while preserving action identity.
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
  remain action-typed through inherited record behavior.
- `record-type-inheritance`: Inheritance rules recognize abstract action bases and concrete derived
  actions without allowing plain record and action hierarchies to mix incorrectly.
- `cli-code-generation`: Exported action inheritance generates coherent TypeScript and C# contracts
  with inherited fields and concrete discriminator values.

## Impact

- `crates/nx-syntax` grammar, AST helpers, validation, parser fixtures, and syntax tests for
  `abstract action` and `action ... extends ...`
- `crates/nx-hir`, `crates/nx-binding`, and prepared binding/model code that classify inherited
  action declarations
- `crates/nx-types` and runtime consumers that validate action compatibility, field inheritance,
  and handler inputs
- `crates/nx-cli/src/codegen.rs` plus language-specific generators and tests for inherited action
  output
