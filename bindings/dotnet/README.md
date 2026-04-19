# NxLang.Runtime - NX .NET Bindings

`.NET 10` bindings for the NX language runtime, implemented in C# and backed by the native Rust FFI library.

- **Assembly**: `NxLang.Runtime.dll`
- **Namespace**: `NxLang.Nx`
- **Primary language**: C#
- **Support posture**: C# usage is tested and documented today. Other .NET languages should work because the binding is a normal managed assembly over a native library, but they are not yet validated in this repository.

## Architecture

```
┌─────────────────┐
│   .NET Code     │
│  (NxRuntime)    │
└────────┬────────┘
         │ managed wrapper
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

The managed binding validates the native ABI version at startup and expects the native library to be staged alongside the application output.

## Prerequisites

- **.NET SDK**: `.NET 10.0`
- **Rust**: the workspace toolchain declared in `rust-toolchain.toml`
- **OS**: Linux, macOS, or Windows

## Build

Build the native runtime first:

```bash
cargo build --release -p nx-ffi
```

Then build the managed solution:

```bash
dotnet build bindings/dotnet/NxLang.sln
```

Run the C# test suite:

```bash
dotnet test bindings/dotnet/NxLang.sln
```

The test project imports `bindings/dotnet/build/NxLang.Runtime.targets`, which copies the native library from `target/release` into the test output directory.

To test against a debug native build instead, build `nx_ffi` without `--release` and pass the native
library configuration explicitly:

```bash
cargo build -p nx-ffi
dotnet test bindings/dotnet/NxLang.sln -p:NxRuntimeNativeLibraryConfiguration=Debug
```

## Supported Integration Workflows

NX is not published as a NuGet package yet. The supported consumption model for now is vendoring the NX repository as a git submodule or subtree and building it locally.

### Primary: `ProjectReference`

Use a direct project reference to the managed binding and import the staging targets file:

```xml
<ItemGroup>
  <ProjectReference Include="external/nx/bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj" />
</ItemGroup>

<Import Project="external/nx/bindings/dotnet/build/NxLang.Runtime.targets" />
```

Recommended flow:

1. Vendor NX source into your repository.
2. Run `cargo build --release -p nx-ffi` in the vendored NX checkout.
3. Build your .NET solution.
4. Let `NxLang.Runtime.targets` copy the native library from `target/release` into your application output.

Optional properties:

- `NxRuntimeNativeLibraryConfiguration`: choose `Debug` or `Release` when `NxRuntimeNativeLibraryDir` is not set. Defaults to `Release`.
- `NxRuntimeNativeLibraryDir`: override the directory that contains the built native library.
- `NxRuntimeStageNativeLibrary`: set to `false` if you want to stage the library yourself.
- `NxRuntimeFailIfNativeLibraryMissing`: set to `true` to fail the build when the native library is missing.

### Secondary: Built Assembly Reference

If you cannot use `ProjectReference`, reference the built managed assembly directly and copy the native library alongside your application's output:

- **Linux**: `target/release/libnx_ffi.so`
- **macOS**: `target/release/libnx_ffi.dylib`
- **Windows**: `target/release/nx_ffi.dll`

The managed runtime looks for the native library in the application base directory and the managed assembly directory.

## Usage

### Basic Evaluation

```csharp
using MessagePack;
using NxLang.Nx;

int result = NxRuntime.Evaluate<int>("let root() = { 42 }");
string text = NxRuntime.Evaluate<string>("let root() = { \"Hello, NX!\" }");
bool flag = NxRuntime.Evaluate<bool>("let root() = { true }");
```

### Canonical Raw Bytes

```csharp
using MessagePack;
using NxLang.Nx;

byte[] resultBytes = NxRuntime.EvaluateBytes("let root() = { 42 }");

int value = MessagePackSerializer.Deserialize<int>(resultBytes);
```

The raw-byte APIs now let you choose the returned wire format per call. `MessagePack` remains the
default:

```csharp
using System.Text;
using NxLang.Nx;

byte[] jsonBytes = NxRuntime.EvaluateBytes(
    "let root() = { { answer: 42 } }",
    NxOutputFormat.Json);

string json = Encoding.UTF8.GetString(jsonBytes);
```

### Reusable Program Artifacts

```csharp
using NxLang.Nx;

using NxLibraryRegistry registry = new();
registry.LoadFromDirectory("/app/question-flow");
using NxProgramBuildContext buildContext = registry.CreateBuildContext();

string source = """
    import "../question-flow"
    let root() = { answer() }
    """;

using NxProgramArtifact program = NxProgramArtifact.Build(source, buildContext, "/app/main.nx");
int value = NxRuntime.Evaluate<int>(program);
```

Build a `NxProgramArtifact` when you want to reuse the same resolved program across evaluation or
component lifecycle calls. If the source imports local NX libraries, preload them in a
`NxLibraryRegistry` and build through a `NxProgramBuildContext` so program construction uses the
selected loaded snapshots instead of reading libraries from disk on demand. The parameterless
`NxProgramArtifact.Build(source, fileName)` convenience still exists, but it now creates a
transient empty registry/build-context pair internally before calling the native build API.

### Direct JSON Output

```csharp
using System.Text.Json;
using NxLang.Nx;

JsonElement json = NxRuntime.EvaluateJson("let root() = { { answer: 42 } }");
int answer = json.GetProperty("answer").GetInt32();
```

Use the JSON convenience APIs when C# needs a parsed JSON view without introducing MessagePack into
that call path. Use the raw-byte overloads with `NxOutputFormat.Json` when you want UTF-8 JSON
bytes that can be forwarded directly to another client.

### Enum Encoding

NX enum values are encoded as the bare authored member string on the wire, both in raw and typed
layers, for JSON and MessagePack alike:

```json
"dark"
```

Raw APIs (`EvaluateBytes`, `EvaluateJson`, `InitializeComponentJson`,
`DispatchComponentActionsJson`) emit the member string directly. When the host feeds a raw value
back into the runtime for a slot whose declared NX type is an enum, the runtime resolves the string
against that enum's member list. Unknown members surface through the standard argument
type-mismatch error path.

Typed generated DTOs use the same member-string contract. Generated enums emit an explicit
wire-format mapping type and rely on `NxEnumJsonConverter<TEnum, TWire>` and
`NxEnumMessagePackFormatter<TEnum, TWire>` from `NxLang.Runtime` to (de)serialize the authored
member string.

Use the raw APIs when you need a schema-free value tree. Use typed generated models when you want
ergonomic host-side enums.

### Component Lifecycle

```csharp
using NxLang.Nx;
using System.Text.Json;

string source = """
    action SearchSubmitted = { searchString:string }

    component <SearchBox placeholder:string emits { SearchSubmitted } /> = {
      state { query:string = {placeholder} }
      <TextInput value={query} placeholder={placeholder} />
    }
    """;

NxComponentInitResult<TextInputElement> init =
    NxRuntime.InitializeComponent<SearchBoxProps, TextInputElement>(
        source,
        "SearchBox",
        new SearchBoxProps { Placeholder = "Find docs" });

byte[] savedSnapshot = init.StateSnapshot;

NxComponentDispatchResult<SearchSubmittedAction> dispatch =
    NxRuntime.DispatchComponentActions<SearchSubmittedAction[], SearchSubmittedAction>(
        source,
        savedSnapshot,
        new[]
        {
            new SearchSubmittedAction
            {
                Type = "SearchSubmitted",
                SearchString = "docs"
            }
        });
```

If the host wants JSON results instead of typed MessagePack models:

```csharp
NxComponentInitResult<JsonElement> initJson =
    NxRuntime.InitializeComponentJson(
        source,
        "SearchBox",
        new SearchBoxProps { Placeholder = "Find docs" });

NxComponentDispatchResult<JsonElement> dispatchJson =
    NxRuntime.DispatchComponentActionsJson(
        source,
        initJson.StateSnapshot,
        new[]
        {
            new SearchSubmittedAction
            {
                Type = "SearchSubmitted",
                SearchString = "docs"
            }
        });
```

- Initialization returns the rendered element plus an opaque `StateSnapshot` byte array that the host owns.
- Dispatch consumes that saved snapshot and an ordered action list, then returns effect actions plus the next snapshot.
- Reuse a saved `StateSnapshot` only with the exact same `NxProgramArtifact` revision that produced it.
  Mixing snapshots across program revisions is rejected.
- The managed source-based component helpers build transient `NxProgramArtifact`s internally and then
  call the native program-artifact component APIs. The public native C ABI itself is artifact-first.
- State defaults run only during initialization in this change. Declarative state-update actions are still a follow-up, so dispatch currently preserves state values while still producing effect actions from bound handlers.
- Component lifecycle inputs remain MessagePack-only in this phase. Typed prop/action overloads still
  serialize those inputs as MessagePack before calling the runtime.
- JSON component results encode `state_snapshot` as base64 on the wire, and the managed binding
  decodes that back to `StateSnapshot` bytes for later dispatch calls.

### Error Handling

```csharp
using NxLang.Nx;

try
{
    int result = NxRuntime.Evaluate<int>("let x = ");
}
catch (NxEvaluationException ex)
{
    foreach (NxDiagnostic diagnostic in ex.Diagnostics)
    {
        if (diagnostic.Severity == NxSeverity.Error)
        {
            Console.WriteLine(diagnostic.Message);
        }
    }
}
```

All source-driven APIs run the shared NX static-analysis pipeline before any runtime execution.
If parsing, lowering, scope building, or type checking reports errors, the call returns the full
diagnostic set and does not execute `root`, component initialization, or component dispatch.

### Generated Types

NX type generation remains C#-first:

```bash
# Single NX file to stdout or a chosen file
nxlang generate Person.nx --language csharp --csharp-namespace MyApp.Models > Person.g.cs

# Full NX library to a generated output directory
nxlang generate ./models --language csharp --csharp-namespace MyApp.Models --output ./generated
```

Generation now honors NX export visibility, so only declarations marked `export` are emitted.
Library generation writes one `.g.cs` file per contributing module under the requested output
directory. Generated enums use the authored NX member spellings for both JSON and MessagePack, the
same bare-string shape raw runtime payloads carry. Generated C# enums now rely on shared helpers
from `NxLang.Runtime` under `NxLang.Nx.Serialization`, so the project that compiles the generated
files must reference `NxLang.Runtime` in addition to the serializer packages it already uses. The
generated output emits the enum itself plus an explicit wire-format mapping type; the JSON
converter and MessagePack formatter implementation now comes from the shared runtime assembly.

## Troubleshooting

### Native runtime could not be found

Build `crates/nx-ffi` and stage the native library next to the application output. If you are consuming NX as a vendored source dependency, import `bindings/dotnet/build/NxLang.Runtime.targets` to automate that copy step.

### Native runtime ABI mismatch

Rebuild both the managed and native pieces from the same NX source revision. `NxLang.Runtime.dll` and `nx_ffi` must come from the same checkout.

### Entry point not found for new component lifecycle methods

Build or rebuild the native `nx_ffi` library for the same configuration as your managed output. For example:

```bash
cargo build -p nx-ffi
dotnet test bindings/dotnet/NxLang.sln -p:NxRuntimeNativeLibraryConfiguration=Debug
```

## Project Structure

```text
bindings/dotnet/
├── build/
│   └── NxLang.Runtime.targets
├── src/
│   └── NxLang.Runtime/
│       ├── Interop/
│       ├── Serialization/
│       ├── NxRuntime.cs
│       ├── NxDiagnostic.cs
│       ├── NxSeverity.cs
│       └── Properties/
├── tests/
│   └── NxLang.Runtime.Tests/
├── Directory.Packages.props
├── NxLang.sln
└── README.md
```
