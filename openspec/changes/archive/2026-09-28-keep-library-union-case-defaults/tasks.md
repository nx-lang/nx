## 1. Fix

- [x] 1.1 Add `UnionCaseField.has_default` and `is_required()`; set it from the interface's flags in `interface_union` and from the expression when lowering
- [x] 1.2 Use `is_required()` in the union-case element check and in the published union-case interface field
- [x] 1.3 Test that a consumer omits a defaulted field of a library union case and gets the default, and that a required field stays required (`library_source_tests::a_union_case_field_default_declared_in_a_library_applies_to_a_tenant_construction`); confirm it fails without the fix
- [x] 1.4 Run the full Rust suite (2554 tests pass) and rebuild the wasm SDK
