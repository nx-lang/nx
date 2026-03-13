## Why

NX can currently describe emitted component actions only as inline payload shapes inside `emits`, which prevents
multiple components from sharing the same action contract and leaves no first-class way to distinguish
action-shaped records from ordinary records. Adding explicit `action` declarations now gives the language a stable
way to model reusable action contracts before later work starts requiring actions in specific contexts.

## What Changes

- Add a top-level `action <Name> = { ... }` declaration form that uses the existing record-style property syntax.
- Define actions as action records: they are valid anywhere a normal record can be used, while still remaining
  distinguishable from non-action records for future action-only contexts.
- Extend component `emits` syntax so entries with braces define new component-scoped actions and entries without
  braces reference previously declared actions.
- Update parser-facing documentation, examples, and validation coverage for action declarations and mixed `emits`
  groups.
- Defer any new action-only runtime behavior or additional contexts that require actions to follow-up changes.

## Capabilities

### New Capabilities
- `action-records`: Declare reusable action records with `action` syntax and treat them as record-compatible types
  with distinct action identity.

### Modified Capabilities
- `component-syntax`: Allow component `emits` groups to contain either inline action definitions or references to
  existing action declarations.

## Impact

- `crates/nx-syntax` grammar, generated parser output, syntax kinds, validation, highlighting, and parser tests
- `crates/nx-hir` and related consumers that currently assume only `type ... = { ... }` introduces record-shaped
  declarations
- Component emit metadata, examples, and parser/reference documentation that describe shared versus component-scoped
  actions
