## 1. Type checker (`nx-types`)

- [x] 1.1 Add named-type satisfaction that combines record subtyping with external component contract ancestry (`extends` chain) for `Type::Named` vs `Type::Named`.
- [x] 1.2 Extend least upper bound for two named types to fall back to a shared external component supertype when record common supertype does not apply.
- [x] 1.3 Add regression tests for a single derived external value under an abstract base type and for a `Base[]`-annotated braced list mixing sibling derived externals.

## 2. Interpreter (`nx-interpreter`)

- [x] 2.1 Resolve the effective external component contract by visible name (including import targets) for runtime type compatibility checks.
- [x] 2.2 When matching an external component record value against an expected named type, accept the value when the concrete runtime type is the expected name or inherits from it via contract ancestors.
- [x] 2.3 Add interpreter regression tests mirroring the type checker scenarios (single value and mixed list return).
