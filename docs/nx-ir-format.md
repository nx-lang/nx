# NX IR JSON v1

NX IR is a deterministic JSON artifact emitted from a successful `ProgramArtifact`. It is intended
for caching, inspection, and loading by the TypeScript IR runtime without re-reading NX source.

## Artifact Shape

An `.nxir.json` document contains:

- `format`: `nx-ir-json`
- `schemaVersion`: `1`
- `runtimeAbi`: `nx-ir-runtime-v1`
- `programFingerprint`: decimal string form of the native `u64` fingerprint
- `requiredFeatures`
- public function and component entrypoints
- resolved modules and module-qualified declaration references
- declarations, expressions, component contracts, prop/state defaults, and source spans
- source entries used for diagnostics and artifact inspection

Type references distinguish built-in primitives from nominal declarations. Primitive references use
`{ "kind": "primitive", "name": "string" }` or another built-in type name. Nominal references use
`{ "kind": "nominal", "reference": { ... }, "display": "User" }`, where `reference` is the
module-qualified declaration identity used by runtimes for record, union, enum, and type-alias
boundary normalization. Display names are retained for canonical `$type` values, diagnostics, and
human-readable tooling output.

Expression records use tagged `op` payloads for literals, slots, top-level references, calls,
unary/binary operations, `if`, match-style `if is`, `let`, blocks, arrays, `for`, index/member
access, records, union cases, enum members, intrinsic elements, and component descriptors.
Index access requires an array base and integer index. Negative or out-of-bounds indexes produce a
runtime diagnostic rather than a synthetic `null` value.

Large integer literals that cannot safely round-trip through JavaScript numbers are encoded with a
string `value` and no numeric `number` field. The TypeScript runtime preserves those values and
rejects arithmetic that would require lossy JavaScript number conversion.

Cache identity is byte-level on the deterministic IR JSON. `programFingerprint` is emitted as a
decimal string so JavaScript consumers can compare it without `Number` precision loss; structured
native and managed metadata may expose the same fingerprint as an integer type.

## CLI Usage

```bash
nxlang codegen ./app/main.nx --target nx-ir --output ./generated
```

Workspace inputs use the same selected entry identity as executable source generation:

```bash
nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated
```

IR generation writes one `<entry>.nxir.json` artifact and does not emit JavaScript/TypeScript
runtime helper files.

## TypeScript Runtime

The TypeScript runtime lives in `runtime/typescript` and exposes:

- `prepareNxIrProgram` / `tryPrepareNxIrProgram`
- `evaluateFunction`
- `constructComponentDescriptor`
- `initializeComponent`
- `evaluateComponent`
- `normalizeComponentState`
- `applyComponentStatePatch`

Runtime loading validates the format identifier, schema version, runtime ABI, required features,
structural references, and expression operation tags before returning a prepared program.
Public host APIs such as `evaluateFunction` and `initializeComponent` resolve names through the
exported function/component entrypoint tables. Runtime internals use module-qualified declaration
references rather than global bare declaration-name lookup.

## Non-Goals In v1

NX IR v1 is eager and non-reactive. It does not implement dependency tracking, subscriptions,
invalidation, hidden mutable component instances, NX-owned reducers, or serialized action-handler
execution. Host-owned component state is validated and patched through pure runtime APIs.
