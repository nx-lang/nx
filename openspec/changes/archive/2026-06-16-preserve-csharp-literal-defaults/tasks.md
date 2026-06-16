## 1. Model Defaults

- [x] 1.1 Add a typegen representation for renderable literal defaults and unsupported default presence.
- [x] 1.2 Populate field default metadata for exported records/actions, union case fields, and external component props.

## 2. C# Emission

- [x] 2.1 Render supported literal defaults as C# property initializers, ahead of `default!`.
- [x] 2.2 Emit C# generation warnings for non-literal defaults that cannot be preserved.

## 3. Regression Coverage

- [x] 3.1 Add C# typegen tests for record field literal defaults.
- [x] 3.2 Add C# typegen tests for union case and external component prop literal defaults.
- [x] 3.3 Add C# typegen warning coverage for unsupported default expressions.

## 4. Verification

- [x] 4.1 Run focused Rust tests for nx-cli type generation.
- [x] 4.2 Confirm OpenSpec status shows the change is apply-ready and tasks are complete.
