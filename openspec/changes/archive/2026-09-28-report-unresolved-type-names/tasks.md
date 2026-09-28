## 1. Checker

- [x] 1.1 Report `unresolved-type` (with a closest-name suggestion) where the checker resolves a type reference written by the module under analysis, reusing the visibility test of `unresolved-type-argument`; keep references resolved on behalf of other modules on the quiet path
- [x] 1.2 Cover every declaration and annotation position (fields, union-case and action fields, props, state, emits, parameters, return types, `let` annotations, aliases, function types; `extends` keeps its own diagnostic) and every kind of visible name (primitives, imports, built-ins and prelude, type parameters, `Property`/`Update` companions); add a test per spec scenario
- [x] 1.3 Verify a library naming an unresolved type fails to load, and a consumer of a loaded library reports nothing for the library's references

## 2. Sweep and release

- [x] 2.1 Run the full Rust, .NET, TypeScript, playground and example suites; fix every source that names an undeclared type
- [x] 2.2 Validate ReachMe's built-in libraries, templates and seeds through the wasm SDK with no `unresolved-type` diagnostics
- [x] 2.3 Update the language reference (types page) to state that an unresolved type name is an error, and add a release note marking the change as breaking
