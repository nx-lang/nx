## 1. Executable Component Lowering

- [x] 1.1 Extend `crates/nx-hir` component items and lowering so components preserve prop defaults, state fields, emitted-action metadata, and lowered body expressions instead of signature-only metadata
- [x] 1.2 Add lowering/runtime coverage for component initialization inputs, including prop default application, state default evaluation order, empty-state components, and missing required state initializers
- [x] 1.3 Add lowering/runtime coverage that component state defaults are initialization-only and are not reevaluated during later dispatch calls

## 2. Interpreter Component Lifecycle

- [x] 2.1 Add interpreter support for `initialize_component` so it materializes normalized props, computes initial logical state, renders the component body, and produces an opaque serialized state snapshot
- [x] 2.2 Add interpreter support for `dispatch_component_actions` so it decodes the prior state snapshot, processes the host-provided action list in input order, carries state forward unchanged in this phase, and aggregates effect actions from matching handlers
- [x] 2.3 Add interpreter tests for successful init/dispatch flows, invalid or incompatible state snapshots, host-ordered action batches, empty-handler cases, and ordered multi-effect results

## 3. Public Runtime APIs and Bindings

- [x] 3.1 Add bidirectional `NxValue` conversion and component lifecycle result types in `crates/nx-api`, then expose source-based initialization and action-dispatch entry points
- [x] 3.2 Add canonical MessagePack `nx_component_init` and `nx_component_dispatch_actions` entry points in `crates/nx-ffi`, plus MessagePack-to-JSON debug helpers, with round-trip tests for persisted state snapshots and effect payloads
- [x] 3.3 Extend the .NET runtime binding with managed init/dispatch wrappers and tests that prove a host can persist a returned state snapshot and redispatch actions through a fresh runtime instance

## 4. Coverage and Documentation

- [x] 4.1 Add repository-level coverage for this NX core functionality across HIR, interpreter, API, FFI, and .NET layers, excluding state-mutation cases reserved for the later declarative update-action change
- [x] 4.2 Update examples and runtime-facing docs to describe component initialization, host-owned opaque state snapshots, host-provided action ordering, dispatch effect results, and the deferred follow-up for declarative state-update actions
