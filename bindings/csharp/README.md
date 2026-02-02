# NxLang.Runtime - NX C# Bindings

.NET bindings for the NX language runtime, providing a type-safe way to evaluate NX code from C# applications.

- **Assembly**: `NxLang.Runtime.dll`
- **Namespace**: `NxLang.Nx`

## Overview

The NX C# bindings use P/Invoke to call into the native NX FFI library (`nx_ffi`). Results can be returned as MessagePack bytes, JSON strings, or deserialized directly into .NET types.

### Architecture

```
┌─────────────────┐
│   C# Code       │
│  (NxRuntime)    │
└────────┬────────┘
         │ P/Invoke
         ↓
┌─────────────────┐
│  nx_ffi (Rust)  │
│  C ABI Layer    │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ NX Interpreter  │
│    (Rust)       │
└─────────────────┘
```

## Prerequisites

- **.NET SDK**: .NET 9.0 or later
- **Rust**: 1.70+ (to build the FFI library)
- **OS**: Linux, macOS, or Windows

## Building

### 1. Build the Rust FFI Library

From the repository root:

```bash
cargo build --release -p nx-ffi
```

This produces:
- **Linux**: `target/release/libnx_ffi.so`
- **macOS**: `target/release/libnx_ffi.dylib`
- **Windows**: `target/release/nx_ffi.dll`

### 2. Set Library Discovery Path

The C# runtime needs to find the native library. Set the appropriate environment variable:

**Linux**:
```bash
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH
```

**macOS**:
```bash
export DYLD_LIBRARY_PATH=$PWD/target/release:$DYLD_LIBRARY_PATH
```

**Windows** (PowerShell):
```powershell
$env:PATH = "$PWD\target\release;$env:PATH"
```

Alternatively, copy the library to a directory already in your PATH, or place it alongside your application's executable.

### 3. Build the C# Library

```bash
cd bindings/csharp
dotnet build NxLang.sln
```

### 4. Run Tests

```bash
# Make sure LD_LIBRARY_PATH/DYLD_LIBRARY_PATH is set (see step 2)
dotnet test NxLang.sln
```

## Usage

### Basic Evaluation

Evaluate NX code and get results as native .NET types:

```csharp
using NxLang.Nx;

// Evaluate to an integer
int result = NxRuntime.Evaluate<int>("let root() = { 42 }");
Console.WriteLine(result); // 42

// Evaluate to a string
string text = NxRuntime.Evaluate<string>("let root() = { \"Hello, NX!\" }");
Console.WriteLine(text); // Hello, NX!

// Evaluate to a boolean
bool flag = NxRuntime.Evaluate<bool>("let root() = { true }");
Console.WriteLine(flag); // True
```

### MessagePack and JSON Output

Get raw serialized output without deserialization:

```csharp
using NxLang.Nx;

// Get MessagePack bytes
byte[] msgpackBytes = NxRuntime.EvaluateToMessagePack("let root() = { 42 }");

// Get JSON string
string json = NxRuntime.EvaluateToJson("let root() = { 42 }");
Console.WriteLine(json); // "42"
```

### Error Handling

NX evaluation errors throw `NxEvaluationException` with detailed diagnostics:

```csharp
using NxLang.Nx;

try
{
    int result = NxRuntime.Evaluate<int>("let x = "); // Syntax error
}
catch (NxEvaluationException ex)
{
    Console.WriteLine($"Evaluation failed: {ex.Message}");

    foreach (NxDiagnostic diag in ex.Diagnostics)
    {
        Console.WriteLine($"[{diag.Severity}] {diag.Message}");

        foreach (NxDiagnosticLabel label in diag.Labels)
        {
            Console.WriteLine($"  at {label.File}:{label.Span.StartLine}:{label.Span.StartColumn}");
        }
    }
}
```

### File Names in Diagnostics

Provide a custom file name for better error messages:

```csharp
using NxLang.Nx;

try
{
    int result = NxRuntime.Evaluate<int>("let x = ", "config.nx");
}
catch (NxEvaluationException ex)
{
    // Diagnostics will show "config.nx" as the file name
    Console.WriteLine(ex.Diagnostics[0].Labels[0].File); // "config.nx"
}
```

### Custom MessagePack Options

Use custom serialization options for advanced scenarios:

```csharp
using NxLang.Nx;
using MessagePack;

var options = MessagePackSerializerOptions.Standard
    .WithSecurity(MessagePackSecurity.UntrustedData)
    .WithCompression(MessagePackCompression.Lz4Block);

int result = NxRuntime.Evaluate<int>("let root() = { 42 }", null, options);
```

### Using Generated Types

Generate C# types from NX schemas using the `nxlang` CLI:

```bash
# Create a schema file
cat > Person.nx << 'EOF'
type Person = {
  name: string
  age: int
  email: string?
}
EOF

# Generate C# types
nxlang codegen --language csharp --namespace MyApp.Models Person.nx > Person.cs
```

Use the generated types:

```csharp
using MyApp.Models;
using NxLang.Nx;

string source = @"
type Person = { name: string, age: int, email: string? }
let root() = { { name: ""Alice"", age: 30, email: null } }
";

Person person = NxRuntime.Evaluate<Person>(source);
Console.WriteLine($"{person.Name} is {person.Age} years old");
```

## API Reference

All public types are in the `NxLang.Nx` namespace.

### `NxRuntime` Class

Main entry point for evaluating NX code.

#### Methods

- **`byte[] EvaluateToMessagePack(string source, string? fileName = null)`**
  Evaluates NX source and returns MessagePack-serialized bytes.

- **`string EvaluateToJson(string source, string? fileName = null)`**
  Evaluates NX source and returns a JSON string.

- **`T Evaluate<T>(string source, string? fileName = null, MessagePackSerializerOptions? options = null)`**
  Evaluates NX source and deserializes to type `T`.

#### Parameters

- **`source`**: The NX source code. Must contain a `root()` function.
- **`fileName`** (optional): File name for diagnostic messages.
- **`options`** (optional): MessagePack serialization options.

#### Exceptions

- **`ArgumentNullException`**: Thrown when `source` is null.
- **`NxEvaluationException`**: Thrown when evaluation fails (syntax errors, missing root function, runtime errors).
- **`InvalidOperationException`**: Thrown when the native library cannot be found.

### `NxEvaluationException` Class

Exception thrown when NX evaluation fails.

#### Properties

- **`NxDiagnostic[] Diagnostics`**: Array of diagnostic messages with error details.

### `NxDiagnostic` Class

Represents a diagnostic message from the NX runtime.

#### Properties

- **`string Severity`**: Severity level ("error", "warning", "info").
- **`string? Code`**: Optional diagnostic code.
- **`string Message`**: Main diagnostic message.
- **`NxDiagnosticLabel[] Labels`**: Source code locations related to this diagnostic.
- **`string? Help`**: Optional help message.
- **`string? Note`**: Optional note.

### `NxDiagnosticLabel` Class

Points to a specific location in source code.

#### Properties

- **`string File`**: File name.
- **`NxTextSpan Span`**: Text span with line/column information.
- **`string? Message`**: Optional label-specific message.
- **`bool Primary`**: Whether this is the primary label.

### `NxTextSpan` Class

Represents a span of text in a source file.

#### Properties

- **`uint StartByte`**: Starting byte offset.
- **`uint EndByte`**: Ending byte offset.
- **`uint StartLine`**: Starting line (0-based).
- **`uint StartColumn`**: Starting column (0-based).
- **`uint EndLine`**: Ending line (0-based).
- **`uint EndColumn`**: Ending column (0-based).

## Troubleshooting

### "NX native library (nx_ffi) was not found"

**Cause**: The C# runtime cannot find the native library.

**Solution**:
1. Verify the library was built: `ls target/release/libnx_ffi.so` (Linux)
2. Set the library path environment variable (see Building section)
3. Or copy the library to a directory in your PATH
4. Or place it in the same directory as your executable

### "Package 'MessagePack' has a known vulnerability"

**Cause**: MessagePack 2.5.140 has a known vulnerability (GHSA-4qm4-8hg2-g2xm).

**Solution**: This warning can be safely ignored for trusted input. For production use, consider:
- Updating to a newer MessagePack version when available
- Using JSON output instead: `NxRuntime.EvaluateToJson()`
- Validating input before evaluation

### Tests Fail with DllNotFoundException

**Cause**: Library path not set when running tests.

**Solution**: Set the library path before running tests:
```bash
export LD_LIBRARY_PATH=$PWD/target/release:$LD_LIBRARY_PATH  # Linux
dotnet test bindings/csharp/NxLang.sln
```

## Project Structure

```
bindings/csharp/
├── src/
│   └── NxLang.Runtime/         # Main library (NxLang.Nx namespace)
│       ├── NxRuntime.cs
│       ├── NxDiagnostic.cs
│       ├── NxEvaluationException.cs
│       ├── NxBuffer.cs
│       ├── NxEvalStatus.cs
│       └── NxNativeMethods.cs
├── tests/
│   └── NxLang.Runtime.Tests/   # Integration tests
│       ├── NxRuntimeBasicTests.cs
│       ├── NxRuntimeErrorTests.cs
│       └── NxEndToEndTests.cs
├── NxLang.sln                  # Solution file
├── Directory.Packages.props
└── README.md                   # This file
```

## License

This project is licensed under the MIT License. See the [LICENSE](../../LICENSE) file for details.
