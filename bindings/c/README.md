# NX C FFI Bindings

This directory contains the C FFI layer for the NX language runtime.

## Overview

The C FFI is implemented in Rust (`crates/nx-ffi`) and provides a stable C ABI for language bindings.

## Language-Specific Bindings

For language-specific integrations, see:
- [.NET Bindings](../dotnet/README.md)

## Header File

The C header file is located at `bindings/c/nx.h` and is generated from the Rust FFI implementation.

Refresh it with one of the following commands:

```bash
bash tools/generate-nx-ffi-header.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File tools/generate-nx-ffi-header.ps1
```

## Workspace APIs

`NxWorkspaceModule` describes one borrowed logical module for the duration of a native call:

- `identity_ptr` / `identity_len`: UTF-8 logical identity bytes such as `app/main.nx`
- `source_utf8_ptr` / `source_utf8_len`: UTF-8 NX source bytes

Use `nx_validate_workspace` to validate a set of descriptors against an existing
`NxProgramBuildContextHandle`. It returns `NxEvalStatus_Ok` when the validation request succeeds
and serializes a MessagePack diagnostics array into `out_buffer`; an empty array means the
workspace is valid.

Use `nx_build_workspace_program_artifact` to build a reusable `NxProgramArtifactHandle` from the
same descriptors plus an explicit entry identity. The function copies any data needed by the
artifact into NX-owned memory before returning. Static NX errors or a missing entry serialize
diagnostics and return `NxEvalStatus_Error`; malformed pointers, invalid UTF-8, malformed logical
identities, or duplicate normalized identities return `NxEvalStatus_InvalidArgument`.
