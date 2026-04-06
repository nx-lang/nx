# Review: Library Artifact FFI + .NET Bindings

## Findings

### 🟡 Fixed RF1 — `NxLibraryArtifactHandle` missing `#[repr(C)]`

**File:** [lib.rs:34](crates/nx-ffi/src/lib.rs#L34)
**Severity:** Bug

`NxProgramArtifactHandle` has `#[repr(C)]` (via the block at line 23 that covers `NxBuffer`... actually it doesn't — see below), but `NxLibraryArtifactHandle` has no `#[repr(C)]` attribute. Since both structs are passed as opaque pointers (`*mut`) across the FFI boundary and never dereferenced by the foreign caller, this is not a correctness issue *today*. However, it's inconsistent — `NxProgramArtifactHandle` also lacks `#[repr(C)]`, so both are relying on the opaque-pointer pattern. This is fine but worth noting: if either struct ever gains a second field that the C side reads, the missing `repr(C)` will silently produce UB.

**Verdict:** Not a bug right now (opaque pointers), but a latent risk. Consider adding `#[repr(C)]` to both handle structs for defensive correctness.

- **Fix:** Added `#[repr(C)]` to both `NxProgramArtifactHandle` and `NxLibraryArtifactHandle` so the opaque-handle ABI stays defensive and consistent.

---

### 🔴 Open RF2 — `_library_artifact` field is unused — artifact is built then immediately discarded

**File:** [lib.rs:34-36](crates/nx-ffi/src/lib.rs#L34-L36)
**Severity:** Bug / Missing functionality

The `NxLibraryArtifactHandle` stores `_library_artifact: LibraryArtifact` with a leading underscore, suppressing the unused-field warning. The FFI exposes `nx_build_library_artifact_from_directory` (build) and `nx_free_library_artifact` (drop), but there is no FFI function that *uses* the library artifact handle — e.g., to evaluate a program against a pre-built library, or to query the library's exports. The .NET side similarly has `DangerousGetHandle()` but nothing calls it.

This means the caller can build and dispose a library artifact, but cannot actually *do* anything with it between those two calls. The `_` prefix on the field confirms this awareness, but the feature is incomplete as shipped: users pay the cost of building the artifact with no way to use it.

**Recommendation:** Either add an FFI entry point that accepts the library handle (e.g., `nx_eval_with_library_artifact` or `nx_build_program_with_library`), or document that the build/free pair is intentionally a forward-looking skeleton.

- **Status:** Left open. The staged change still exposes only build/free for library artifacts; adding a consumer API needs a broader design decision than this fix pass.

---

### 🟡 Fixed RF3 — `DangerousGetHandle` is dead code on `NxLibraryArtifact`

**File:** [NxLibraryArtifact.cs:91-96](bindings/dotnet/src/NxLang.Runtime/NxLibraryArtifact.cs#L91-L96)
**Severity:** Low (code quality)

`DangerousGetHandle()` is defined as `internal` on `NxLibraryArtifact` but has no callers. It was presumably copied from `NxProgramArtifact` where it's used by the eval path. Until there is a .NET method that passes the library handle back into native code, this is dead code.

- **Fix:** Removed the unused `DangerousGetHandle()` helper from `NxLibraryArtifact`.

---

### ✅ Resolved RF4 — `library_artifact_error_diagnostics` passes empty string for source context

**File:** [eval.rs:72-77](crates/nx-api/src/eval.rs#L72-L77)
**Severity:** Low (diagnostic quality)

```rust
pub(crate) fn library_artifact_error_diagnostics(
    library: &LibraryArtifact,
) -> Option<Vec<NxDiagnostic>> {
    has_error_diagnostics(&library.diagnostics)
        .then(|| diagnostics_to_api(&library.diagnostics, ""))
}
```

The `""` fallback source means any diagnostic labels with source spans will have no source text to render against. The `LibraryArtifact` contains multiple modules each with their own source, but `diagnostics_to_api` receives none of it. Compare with `program_artifact_error_diagnostics` which at least receives `fallback_source`. The result is that library build errors will have correct error codes and messages but empty/missing source snippets in diagnostic labels.

- **Status:** Resolved as not a bug in the current implementation. Library diagnostics carry on-disk module paths, and `diagnostics_to_api` reloads `label.file` when it exists. Added an FFI regression test that asserts invalid library diagnostics preserve the module path and line number.

---

### 🟡 Fixed RF5 — No test for `nx_build_library_artifact_from_directory` on the Rust FFI side

**File:** [lib.rs:327-366](crates/nx-ffi/src/lib.rs#L327-L366)
**Severity:** Medium (test coverage)

The .NET end-to-end tests cover the happy path and the invalid-source path, but there are no Rust-level tests for the FFI function itself. Key untested FFI-specific paths:
- Null `out_handle` or `out_buffer` pointers (the `InvalidArgument` guards)
- Empty `root_path` string
- Panic recovery inside `catch_unwind`

The existing FFI crate appears to have no tests at all, so this is consistent with the current approach, but the new function adds more complex logic (directory I/O) that would benefit from at least a basic integration test.

- **Fix:** Added Rust FFI smoke tests for library-artifact build success, invalid-library diagnostics, null output-pointer validation, and empty-root-path rejection.

---

### ✅ Resolved RF6 — Success path leaks handle on `rmp_serde` serialization failure in error branch

**File:** [lib.rs:356-362](crates/nx-ffi/src/lib.rs#L356-L362)
**Severity:** Not a bug (false alarm on closer inspection)

On the success path, the handle is written to `*out_handle` before returning. On the error path (`Err(diagnostics)`), no handle is allocated, so no leak. If the `rmp_serde::to_vec_named` call fails in the error branch, the `Err(...)` propagates up through `finish_msgpack_entry` which writes an ffi-error diagnostic — no handle was allocated in that branch, so it's clean. The success path sets the handle and returns `Ok((Ok, Vec::new()))` — the empty vec can't fail serialization. This looks correct.

**Verdict:** No issue. Removed from actionable findings.

---

## Summary

| ID  | Status       | Severity | Summary |
|-----|--------------|----------|---------|
| RF1 | Fixed        | Low      | Handle structs now use `#[repr(C)]` for defensive ABI consistency |
| RF2 | Open         | Medium   | Library artifact handle is built but never usable (no consuming FFI endpoint) |
| RF3 | Fixed        | Low      | Removed dead `DangerousGetHandle` helper from `NxLibraryArtifact` |
| RF4 | Resolved     | Low      | Library diagnostics preserve file context via on-disk `label.file` reload |
| RF5 | Fixed        | Medium   | Added Rust-level FFI tests for the new library artifact functions |
| RF6 | Resolved     | N/A      | No success-path leak after closer inspection |
