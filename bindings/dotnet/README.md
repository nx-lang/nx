# NxLang.Sdk - NX .NET SDK

`.NET 10` SDK for NX host, compiler, diagnostics, program artifact, IR generation, and DTO serialization
workflows, implemented in C# and backed by the native Rust FFI library.

- **Assembly**: `NxLang.Sdk.dll`
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
│ NX Runtime      │
│    (Rust)       │
└─────────────────┘
```

Rust and TypeScript are the NX runtime implementations. This SDK exposes compiler tooling through
the native library and provides no execution binding. C# type generation is separate from
NX execution and does not imply an executable C# backend.

The managed binding validates the native ABI version at startup. Published package consumers get
the native SDK library through normal .NET runtime asset restore, build, test, and publish behavior.

## Prerequisites

- **.NET SDK**: `.NET 10.0`
- **Rust**: the workspace toolchain declared in `rust-toolchain.toml` when building NX from source
- **OS**: Linux, macOS, or Windows

## Build

Build the native SDK library first:

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

The test project imports `bindings/dotnet/build/NxLang.Sdk.targets`, which copies the native library from `target/release` into the test output directory.

To test against a debug native build instead, build `nx_ffi` without `--release` and pass the native
library configuration explicitly:

```bash
cargo build -p nx-ffi
dotnet test bindings/dotnet/NxLang.sln -p:NxSdkNativeLibraryConfiguration=Debug
```

## Supported Integration Workflows

### Primary: `PackageReference`

Applications should reference the published SDK package:

```xml
<ItemGroup>
  <PackageReference Include="NxLang.Sdk" Version="0.1.0" />
</ItemGroup>
```

The package contains `NxLang.Sdk.dll` and the native `nx_ffi` runtime assets for supported
runtime identifiers under `runtimes/<rid>/native/`. Package consumers do not need to vendor the NX
repository, import `bindings/dotnet/build/NxLang.Sdk.targets`, or install Rust just to build,
test, run, or publish an application that uses `NxLang.Sdk`.

Publish for a supported runtime identifier when creating a deployable application:

```bash
dotnet publish MyApp.csproj -c Release -r linux-x64 --self-contained false
```

Initial supported RIDs are:

- `linux-x64`
- `osx-arm64`
- `win-x64`

Application-owned `.nx` source files and domain libraries are not packaged by NX. Embed them, copy
them as content, or otherwise stage them from the consuming application.

## Package Publishing

Cross-package CI setup is documented in
[docs/deployment-setup.md](../../docs/deployment-setup.md), and the recurring release runbook is in
[docs/deployment.md](../../docs/deployment.md). The `NxLang.Sdk` package is published from the
complete `deployables-Complete` artifact after package metadata verification and RID smoke tests
pass. Production publishing prefers NuGet.org trusted publishing with `NUGET_API_KEY` as a gated
fallback.

### Advanced: Source `ProjectReference`

Use a direct project reference to the managed binding and import the staging targets file:

```xml
<ItemGroup>
  <ProjectReference Include="external/nx/bindings/dotnet/src/NxLang.Sdk/NxLang.Sdk.csproj" />
</ItemGroup>

<Import Project="external/nx/bindings/dotnet/build/NxLang.Sdk.targets" />
```

Use this flow when contributing to NX or intentionally testing unreleased SDK changes:

1. Vendor NX source into your repository.
2. Run `cargo build --release -p nx-ffi` in the vendored NX checkout.
3. Build your .NET solution.
4. Let `NxLang.Sdk.targets` copy the native library from `target/release` into your application output.

Optional properties:

- `NxSdkNativeLibraryConfiguration`: choose `Debug` or `Release` when `NxSdkNativeLibraryDir` is not set. Defaults to `Release`.
- `NxSdkNativeLibraryDir`: override the directory that contains the built native library.
- `NxSdkStageNativeLibrary`: set to `false` if you want to stage the library yourself.
- `NxSdkFailIfNativeLibraryMissing`: set to `true` to fail the build when the native library is missing.

### Advanced: Built Assembly Reference

If you cannot use `ProjectReference`, reference the built managed assembly directly and copy the native library alongside your application's output:

- **Linux**: `target/release/libnx_ffi.so`
- **macOS**: `target/release/libnx_ffi.dylib`
- **Windows**: `target/release/nx_ffi.dll`

The managed SDK looks for the native library in the application base directory and the managed assembly directory.

### Migration From Source Consumption

To migrate an application from a vendored NX checkout to the package:

1. Remove the `ProjectReference` to `bindings/dotnet/src/NxLang.Sdk/NxLang.Sdk.csproj`.
2. Remove the manual import of `bindings/dotnet/build/NxLang.Sdk.targets`.
3. Add `PackageReference Include="NxLang.Sdk"`.
4. Remove consumer-side `cargo build -p nx-ffi` steps that only existed to stage the runtime.
5. Keep application-specific `.nx` files in the application and stage them with application-owned
   content rules.

## Usage

Build and emit IR with `NxProgramArtifact.Build(source)` and `artifact.GenerateNxIr()`.
Source/artifact evaluation and lifecycle methods have been removed, with no replacement C/.NET
execution binding. DTO serialization and update helpers remain. See the
[migration guide](../../docs/hir-interpreter-retirement.md).

```csharp
using NxProgramArtifact artifact = NxProgramArtifact.Build("let root() = {42}", "main.nx");
NxGeneratedNxIr image = artifact.GenerateNxIr();
```

### Reusable Program Artifacts



Build a `NxProgramArtifact` when you want to reuse the same resolved program across evaluation or
component lifecycle calls. If the source imports local NX libraries, preload them in a
`NxLibraryRegistry` and build through a `NxProgramBuildContext` so program construction uses the
selected loaded snapshots instead of reading libraries from disk on demand. The parameterless
`NxProgramArtifact.Build(source, fileName)` convenience still exists, but it now creates a
transient empty registry/build-context pair internally before calling the native build API.

### Program Module Codegen

Use `NxProgramArtifact.GenerateJSProgramModule` when a managed host needs cacheable JavaScript source
for a resolved NX program rather than immediate interpreter evaluation:

```csharp
using NxLang.Nx;

using NxProgramArtifact program = NxProgramArtifact.Build(source, "/app/main.nx");
NxGeneratedJSProgramModule generated = program.GenerateJSProgramModule(
    new NxJSProgramModuleOptions
    {
        LogicalModuleName = "app/main",
        RuntimeImportSpecifier = "nx:runtime",
    });

string sourceText = generated.SourceText;
string runtimeAbi = generated.RuntimeAbi;
```

The generated source is one host-neutral JavaScript ESM module. It imports NX runtime helpers from
the configured runtime specifier and does not include host wrappers, `nx-runtime.js`, or
Cloudflare/Rivet packaging. The returned metadata includes the program fingerprint, runtime ABI,
function entrypoint exports, and component/schema exports so managed hosts can cache and validate
the module without parsing generated source.

### In-Memory Workspaces

Use `NxWorkspace` when source modules come from editor buffers, database rows, or other logical
records and should not be written to temporary files:

```csharp
using NxLang.Nx;

NxWorkspace workspace = new([
    NxWorkspaceModule.FromSourceText(
        "app/main.nx",
        """
        import { answer } from "../shared/value.nx"
        let root(): int = { answer() }
        """),
    NxWorkspaceModule.FromSourceText(
        "shared/value.nx",
        "export let answer(): int = { 42 }"),
]);

using NxLibraryRegistry registry = new();
using NxProgramBuildContext buildContext = registry.CreateBuildContext();

IReadOnlyList<NxDiagnostic> diagnostics = NxRuntime.ValidateWorkspace(workspace, buildContext);
using NxProgramArtifact program =
    NxProgramArtifact.BuildWorkspace(workspace, "app/main.nx", buildContext);
```

Workspace identities are logical names, not filesystem paths. NX uses `/` separators, normalizes
`.` and `..`, rejects identities or imports that escape the workspace root, and resolves imports by
exact normalized identity before falling back to libraries already visible in the supplied
`NxProgramBuildContext`. Diagnostics preserve normalized workspace identities and calculate spans
from the submitted source bytes. Workspace validation returns NX source diagnostics as data;
malformed workspace inputs such as duplicate normalized identities are rejected as interop argument
errors. Workspace artifact builds throw `NxEvaluationException` when static diagnostics or a missing
entry identity prevent artifact creation.

### Constant Case Encoding

NX has one declaration form for a union. A case is **constant** when it declares no fields and its
union declares no base, so the case carries nothing beyond its own name. A **constant union** is one
whose cases are all constant — the closed set of named constants that `enum` used to declare.

A constant case is encoded as the bare authored case string on the wire, both in raw and typed
layers, for JSON and MessagePack alike:

```json
"dark"
```

This holds wherever the case appears. In a constant union such as `type ThemeMode = light | dark`
every value takes this form; in a union that also has payload cases, the constant ones take it while
the payload ones take the `$type` map described below.

IR runtimes encode constant cases as their authored strings. DTOs deserialize those strings
using the declared union type.

Typed generated DTOs use the same string contract. A constant union generates a CLR `enum` plus an
explicit wire-format mapping type, and relies on `NxEnumJsonConverter<TEnum, TWire>` and
`NxEnumMessagePackFormatter<TEnum, TWire>` from `NxLang.Sdk` to (de)serialize the authored case
string. The CLR `enum` is the generated *host* shape for a constant union; it is not a separate NX
concept.

Use the raw APIs when you need a schema-free value tree. Use typed generated models when you want
ergonomic host-side enums.

### Discriminated Union Encoding

NX discriminated union cases are encoded as canonical maps with a `$type` discriminator whose value
is the fully scoped case name. Payload fields keep their authored NX wire names:

```json
{
  "$type": "LoadState.failed",
  "message": "Offline"
}
```

Raw APIs expose the same shape for JSON and MessagePack. A constant case of the same union, such as
`LoadState.idle`, is a bare string rather than a map, because it carries nothing a map would hold:

```json
"idle"
```

A fieldless case of a union that `extends` an abstract base is **not** constant — it carries the
base's fields — and so keeps the `$type` map form.

Generated C# union roots use `JsonPolymorphic`/`JsonDerivedType` for JSON and
`NxPolymorphicMessagePackFormatter<T>` for MessagePack, so serializing or deserializing through the
generated union root type preserves the same `$type` map contract:

```csharp
LoadState state = new LoadStateFailed
{
    Message = "Offline"
};

string json = JsonSerializer.Serialize(state);
byte[] bytes = MessagePackSerializer.Serialize(state);

LoadState fromJson = JsonSerializer.Deserialize<LoadState>(json)!;
LoadState fromMessagePack = MessagePackSerializer.Deserialize<LoadState>(bytes);
```

A union that mixes the two kinds of case therefore has two wire shapes, and generated readers accept
both. `NxPolymorphicMessagePackFormatter<T>` and the generated JSON converter read a bare string as
the union's constant case of that name and a `$type` map as a payload case; generated C# exposes a
constant case as a `[NxConstantCase]` singleton, for example `LoadStateIdle.Instance`, which
serializes back to the bare string.

### Function Values in Rendered Output

A function value in NX names a declaration and captures nothing, so rendered output represents one as a `Function`
record with the declaring module's identity and the function's name — a template bound to a list, most often. Type the
property as `NxFunctionRef` to read which function it was handed:



`typegen` types a function-typed member this way too. A .NET host can read a function value and pass the record
along, but not call it: a function value is invoked by the NX program that received it, and a `Function` record
supplied back as a prop, as state, or inside an action is refused.

### Update Records and `NxOptional<T>`

Generated `<Name>_update` DTOs type every property as `NxOptional<T>`, which tells an unset property ("leave this
field unchanged") apart from one that is cleared. `null` is the .NET spelling of a cleared field — the NX empty
value `{}` — and only a field the target declares optional (`email?:string`) can be cleared, so the accessor's
value type is nullable only for such a field: `User_update.Email` is `NxOptional<string?>` while `Name` is
`NxOptional<string>`. Unset properties are omitted from both JSON and MessagePack, a cleared one is written as
`null`, and a missing key reads back as unset:

```csharp
User_update patch = new() { Email = null };   // clears email; Name stays unset
string json = JsonSerializer.Serialize(patch); // {"$type":"User.Update","email":null}
```

A component is not a record, so it gets no `<Name>_update` or `<Name>_property` of its own. When it declares
state, it gets `<Name>_state` for the state snapshot with `<Name>_state_update` and `<Name>_state_property` beside it. `Counter_state_update`
derives from `NxUpdate<Counter_state>`, so it applies to the snapshot a host holds, and it serializes with
`$type` `Counter.State.Update`.

The runtime checks every record a host passes in — as a prop, in explicit state, or inside an action (even one with no
bound handler), at any nesting depth — against its NX declaration. A property the NX type does not declare, `null` for a
field that is not optional, or an empty array for a `name:T+` field, fails the call with an `NxEvaluationException`
naming the field, so a DTO that has drifted from the NX source cannot turn "unchanged" into "cleared". On the way out,
an optional field the NX value leaves empty is an omitted key, so it reads as `null`, and a host may send `null`, an
empty array, or no key at all for one; the runtime reads all three as the empty value.

If the host wants JSON results instead of typed MessagePack models:



### MessagePack Polymorphism Migration

Generated C# polymorphic DTOs now use the canonical NX MessagePack map shape with a `$type`
string key instead of MessagePack `Union` envelopes.

- Remove any custom host assumptions that polymorphic records/actions are encoded as `Union` arrays.
- Keep generated action/record DTO classes free of explicit `Type`/`$type` data members.
- Ensure abstract polymorphic roots keep the generated polymorphism attributes so runtime
  serialization can resolve concrete descendants from `$type`.

- Initialization returns the rendered element plus an opaque `StateSnapshot` byte array that the host owns.
- Dispatch consumes that saved snapshot and an ordered action list, then returns effect actions plus the next snapshot.
- Evaluation accepts explicit props and explicit current state, then returns only the rendered value.
- Reuse a saved `StateSnapshot` only with the exact same `NxProgramArtifact` revision that produced it.
  Mixing snapshots across program revisions is rejected.
- Use evaluation for host-owned transparent state. Use initialization and dispatch for NX-owned
  component lifecycles that need opaque snapshot round-tripping and handler effects.
- The managed source-based component helpers build transient `NxProgramArtifact`s internally and then
  call the native program-artifact component APIs. The public native C ABI itself is artifact-first.
- State defaults run only during initialization in this change. Declarative state-update actions are still a follow-up, so dispatch currently preserves state values while still producing effect actions from bound handlers.
- Component runtime inputs remain MessagePack-only in this phase. Typed prop/state/action overloads still
  serialize those inputs as MessagePack before calling the runtime.
- JSON component results encode `state_snapshot` as base64 on the wire, and the managed binding
  decodes that back to `StateSnapshot` bytes for later dispatch calls.

### Generated Types

NX type generation remains C#-first:

```bash
# Single NX file to stdout or a chosen file
nxlang typegen Person.nx --language csharp --csharp-namespace MyApp.Models > Person.g.cs

# Full NX library to a generated output directory
nxlang typegen ./models --language csharp --csharp-namespace MyApp.Models --output ./generated
```

Generation now honors NX export visibility, so only declarations marked `export` are emitted.
Library generation writes one `.g.cs` file per contributing module under the requested output
directory. The generated host shape for a union follows from whether it is constant. A constant
union generates a CLR `enum` using the authored NX case spellings for both JSON and MessagePack, the
same bare-string shape raw runtime payloads carry. A union with any payload case generates an
abstract root plus sealed case DTOs whose JSON and MessagePack attributes use the canonical `$type`
map shape, with each constant case emitted as a `[NxConstantCase]` singleton that serializes as its
bare string. Generated C# enums and unions rely on shared helpers from `NxLang.Sdk` under
`NxLang.Nx.Serialization`, so the project that compiles the generated files must reference
`NxLang.Sdk` in addition to the serializer packages it already uses. The generated constant-union
output emits the `enum` itself plus an explicit wire-format mapping type; the JSON converter and
MessagePack formatter implementation comes from the shared SDK assembly.

A record that declares NX type parameters — `type Range = { T:type start:T end:T }` — generates a
real C# generic, `Range<T>`, and an applied type generates the instantiation, so
`week:<Range T=int/>` is a `Range<long>`. Unlike a component contract, nothing is erased: the host
names the concrete instantiation at its own deserialization site.

The record's `<Name>_update` companion follows it, so a patch is usable at the instantiation the
host holds:

```csharp
Range<long> before = new() { Start = 1, End = 5 };
Range<long> after = new() { Start = 1, End = 9 };

Range_update<long> patch = Range_update<long>.Diff(before, after);
long end = patch.End.Value;              // typed as the record's field, not object
Range<long> applied = patch.Apply(before);
```

`Range_update<T>` derives from `NxUpdate<Range<T>>`, its key table is `RangeProperties<T>`, and its
`Start` and `End` are `NxOptional<T>`. The wire is unaffected by the parameter: a patch serializes
as `{"$type":"Range.Update","end":9}` in either format, with no type argument. A *component's*
state companions still erase the component's type parameters: `<Name>_state_update` patches its
`<Name>_state`, which is already concrete.

Because a generic type cannot name its own converter in an attribute — an attribute argument cannot
use type parameters (CS0416) — a generic companion names `NxUpdateRecordJsonConverterFactory` from
the SDK for JSON, and for MessagePack a generated `<Name>_updateFormatter<T>` shim beside it. Both
are wired up by generation; nothing is needed at the call site, and a non-generic companion is
unchanged.

**AOT note.** Closed instantiations round-trip with the reflection-based resolvers of both
serializers, which is what `JsonSerializer` and `MessagePackSerializer` use by default. A
source-generated or otherwise AOT-safe resolver has no open generic to generate from, so each
instantiation the host actually serializes must be named to it — a `[JsonSerializable(typeof(...))]`
entry per instantiation for `System.Text.Json`, and a generated formatter per instantiation for
MessagePack. This covers the update companions too: `Range_update<long>` is its own instantiation
and needs its own entry, separately from `Range<long>`. `NxUpdateRecordJsonConverterFactory` also
closes its converter through `MakeGenericType` and `Activator.CreateInstance`, which a trimmed or
AOT publish cannot see, so a host publishing that way should keep the companion instantiations it
uses rooted.

## Troubleshooting

### Native SDK library could not be found

Package consumers should restore, build, and publish for one of the supported runtime identifiers
so the `NxLang.Sdk` package can stage the matching native SDK asset. Source consumers should
build `crates/nx-ffi` and import `bindings/dotnet/build/NxLang.Sdk.targets` to automate that
copy step.

### Native SDK ABI mismatch

Rebuild both the managed and native pieces from the same NX source revision. `NxLang.Sdk.dll` and `nx_ffi` must come from the same checkout.

## Project Structure

```text
bindings/dotnet/
├── build/
│   └── NxLang.Sdk.targets
├── src/
│   └── NxLang.Sdk/
│       ├── Interop/
│       ├── Serialization/
│       ├── NxRuntime.cs
│       ├── NxDiagnostic.cs
│       ├── NxSeverity.cs
│       └── Properties/
├── tests/
│   └── NxLang.Sdk.Tests/
├── Directory.Packages.props
├── NxLang.sln
└── README.md
```
