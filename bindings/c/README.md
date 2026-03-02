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
