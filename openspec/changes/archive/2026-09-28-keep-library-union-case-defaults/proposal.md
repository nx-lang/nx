## Why

A default on a union case's field was lost when the union came from a library: a consumer
constructing `<ScaleConfig.numeric min=1 max=5 />` against a library declaring
`numeric { min:int = 0 max:int = 10 step:int = 1 }` was told `step` is required. The library interface
publishes whether a field is required, but the consumer's view of an imported union rebuilt that from
a default expression the interface does not carry, so every defaulted field read as required. Records
and component props, which keep the published flag, were unaffected, as were unions declared in the
same workspace. ReachMe's built-in `question-flow` library hits this.

## What Changes

- An imported union case's field keeps whether it has a default, read from the published
  `is_required` flag, so a defaulted field may be omitted at a consumer's construction and takes its
  default.
- The union-case field check and the published interface both derive "required" from one rule: no
  default and no `?` mark.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `discriminated-unions`: a union-case field default applies at every construction, including one in
  a module that imports the union from a library.

## Impact

- `crates/nx-hir` (`UnionCaseField.has_default`, `is_required()`, the interface view of a union),
  `crates/nx-types` (union-case element bindings), `crates/nx-api` (the published union-case field).
- No change to IR or bindings: the declaring module still evaluates its defaults.
