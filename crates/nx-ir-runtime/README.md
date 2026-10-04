# nx-ir-runtime

The NX IR runtime for Rust. It prepares NX IR images, links them by name and evaluates them, with
the same results, handler tokens and diagnostic codes as the TypeScript runtime
`@nx-lang/ir-runtime`.

It depends on `nx-ir`, the image format, and `nx-value`, the host value type, and on no part of
the NX compiler. A host that holds compiled images runs them with this crate alone.

The image format is documented in [`docs/nx-ir-format.md`](../../docs/nx-ir-format.md). Images
come from `nxlang codegen --target nx-ir`, or from `nx_codegen::emit_nx_ir` in process.

## Prepare, link, evaluate

`PreparedModule::prepare` validates an image completely and indexes its declarations. The bytes
are kept and read in place; nodes are decoded the first time evaluation reaches them. `Program::link`
resolves the modules the entry's table names through a resolver the host supplies.

```rust,no_run
use nx_ir_runtime::{LinkOptions, PreparedModule, Program, RuntimeOptions};

fn main() -> nx_ir_runtime::Result<()> {
    // Prepare the catalog once. A prepared module is shared, never copied, by every program
    // linked against it, and it is `Send + Sync`.
    let catalog = PreparedModule::prepare(std::fs::read("catalog.nxir").unwrap())?;

    let snippet = PreparedModule::prepare(std::fs::read("snippet.nxir").unwrap())?;
    let program = Program::link(
        &snippet,
        |identity| (identity == catalog.identity()).then(|| catalog.clone()),
        &LinkOptions::default(),
    )?;

    let rendered = program.evaluate_function("root", &[], &RuntimeOptions::default())?;
    println!("{}", rendered.to_json_string().unwrap());
    Ok(())
}
```

An image whose module table names only itself, or only itself and the prelude, needs no resolver:

```rust,no_run
use nx_ir_runtime::{Program, RuntimeOptions};
use nx_value::NxValue;

fn main() -> nx_ir_runtime::Result<()> {
    let program = Program::prepare(std::fs::read("main.nxir").unwrap())?;
    let total = program.evaluate_function("total", &[NxValue::Int(3)], &RuntimeOptions::default())?;
    assert_eq!(total, NxValue::Int(6));
    Ok(())
}
```

The prelude is the one module no host has to supply. The crate carries the compiled prelude of
its release and links it for the identity `@nx/prelude.nx` whenever the resolver returns nothing
for it. A resolver that does return a module for that identity is used instead.

Linking fails with `nx-ir-link-missing-module` when nothing supplies a module, with
`nx-ir-link-version` when a resolved module's version is not the one the entry recorded (unless
`LinkOptions::allow_version_mismatch` is set), and with `nx-ir-link-missing-declaration` when a
declaration the entry references is absent.

## Values

Values cross the API as `nx_value::NxValue`.

| NX | Accepted from the host | Returned to the host |
| --- | --- | --- |
| `int`, `int32`, `int64` | `Int32`, `Int`, an integral `Float` | `Int` |
| `float64`, `float32` | `Float32`, `Float`, `Int32`, `Int` | `Float` |
| `string`, `boolean` | `String`, `Bool` | `String`, `Bool` |
| a constant union case | `String` holding the case name | `String` |
| a record, a union case with fields | `Record`, `$type` optional where the site names the type | `Record` with its `$type` |
| a sequence (`T+`, `T*`) | `Array`, or a single value | `Array` |
| an optional (`T?`) that is empty | `Null`, the empty `Array`, or an omitted field | an omitted field |
| a function value | a `Function` record: `{ $type: "Function", module, name }` | the same record |
| an action handler | an `ActionHandler` record, through a parent instance | an `ActionHandler` record |

`Null` is written in two places only: an entry call's result whose type is a standalone `T?` and
holds nothing, and a cleared field of an update record.

Integer arithmetic is 64-bit. An `int32` site checks its range where a value meets it; arithmetic
on `int32` values is not wrapped at 32 bits, as in the JavaScript backends.

## Components

```rust,no_run
use nx_ir_runtime::{ComponentInit, Program, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;

fn main() -> nx_ir_runtime::Result<()> {
    let program = Program::prepare(std::fs::read("counter.nxir").unwrap())?;
    let options = RuntimeOptions::default();

    // Render the component against its initial state.
    let initialized =
        program.initialize_component("Counter", &BTreeMap::new(), &ComponentInit::default(), &options)?;
    // `initialized.rendered` carries each handler as
    // `{ "$type": "ActionHandler", "action": "Button.Tapped", "token": "h1-1" }`.

    // The host reports an interaction by the token it read from the rendered output.
    let tap = NxValue::from_json_str(
        r#"{ "$type": "ActionHandlerInvocation", "token": "h1-1", "action": { "$type": "Button.Tapped" } }"#,
    )
    .unwrap();
    let dispatched = program.dispatch_component_actions(&initialized.instance, &[tap], &options)?;

    // `dispatched.rendered` is the body against the next state, with fresh tokens (`h2-1`);
    // `dispatched.effects` is what the handlers returned for the host; `dispatched.instance` is
    // what the next dispatch runs against. The instance given is never modified.
    let _ = (dispatched.rendered, dispatched.effects, dispatched.state, dispatched.instance);
    Ok(())
}
```

| Operation | What it does |
| --- | --- |
| `construct_component_descriptor` | Builds a component's descriptor from props and content, as an element in a body would. |
| `initialize_component` | Renders the body against the initial state, or a supplied one; returns the rendered output, the state and an instance. `ComponentInit::parent` resolves `ActionHandler` records in the props through the parent's instance. |
| `evaluate_component` | Renders the body from props and explicit state. The output carries no tokens. |
| `dispatch_component_actions` | Runs a batch of emitted actions and handler invocations in order, then renders once. |
| `normalize_component_state` | Validates a complete state. |
| `apply_component_state_patch` | Applies a partial state or a `<Component>.Update` record to a state. |
| `call_function` | Calls the function a `Function` record names, with arguments by parameter name. |
| `apply`, `merge`, `diff`, `Program::changed` | The update intrinsics, over values the host holds. |

### Storing an instance

An instance is plain data and implements `serde::Serialize`. `Program::restore_component_instance`
is the only way back from the serialized form. It checks the instance against the program once:
the program must be linked from the same images, byte for byte; every handler and function the
instance holds must be one of that program; and its props, state and handler properties must be
ones the component accepts. Dispatch then compares only a hash of each image, so a host that keeps
instances in memory pays for no validation.

The serialized form is a flat table that holds each value once, however many handlers captured it,
so its size follows the data and not the number of handlers, and it nests a fixed few levels
whatever the values do: a deserializer with a recursion limit, such as `serde_json`'s, reads any
instance the runtime wrote. The layout is the runtime's own and may change between releases; a
stored instance is for the release and the images that wrote it.

The check is against mistakes, not against an adversary. Someone who can rewrite a stored instance
can give it any state the component accepts, or point a token at another handler of the same
component. A host that stores instances where others can write should authenticate them.

```rust,no_run
use nx_ir_runtime::{ComponentInit, Program, RuntimeOptions};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::prepare(std::fs::read("counter.nxir")?)?;
    let initialized = program.initialize_component(
        "Counter",
        &BTreeMap::new(),
        &ComponentInit::default(),
        &RuntimeOptions::default(),
    )?;

    let stored = serde_json::to_string(&initialized.instance)?;

    // Later, in a process that prepared the same images:
    let restored = program.restore_component_instance(&mut serde_json::Deserializer::from_str(&stored))?;
    assert_eq!(restored.component(), "Counter");
    Ok(())
}
```

## Limits

| Limit | Default | Set by | Name in a diagnostic |
| --- | --- | --- | --- |
| Operations one call may cost | Unlimited | `RuntimeOptions::max_operations` | `maxOperations` |
| Call depth | 100 | `RuntimeOptions::max_call_depth` | `maxCallDepth` |
| Integers one range may hold when a loop iterates it | 1,000,000 | `RuntimeOptions::max_range_length` | `maxRangeLength` |
| Nested expressions, across every call of one evaluation | 1,000 | fixed | `maxExpressionNesting` |
| Native stack one evaluation may use | 1 MiB | fixed | `maxStackBytes` |
| Nesting of a value crossing the host boundary, and of component state after each patch | 256 | fixed | `maxValueNesting` |
| Nesting of the values of a serialized instance | 1,024 | fixed | |
| Values one value of a serialized instance holds, written out | 16,777,216 | fixed | |

Exceeding any of them fails with `nx-ir-resource-limit`, or with `nx-ir-component` for a
serialized instance. The fixed limits are what keep an image, a host value or a raised call depth
from exhausting the native stack, so the thread that calls the runtime needs 1 MiB of stack free.
Checking a value against a declared type is under the stack budget as well, so an unoptimized
build, whose frames are larger, refuses a deeply nested typed value sooner than 256 levels, and
can meet the stack budget before a thousand expressions nest.

**Set `max_operations` for any code you did not write.** It is the only limit that bounds work and
allocation: three nested loops over ranges of a thousand run a billion bodies inside the others,
and a function that doubles a list on each of its 100 permitted calls asks for 2^100 items, an
allocation failure that aborts the process. An operation is a unit of work on a value: a node
evaluated, an item placed in a sequence a node builds or bound to a content parameter, 64 UTF-16
code units of a string a concatenation produces, a value checked against a declared type, a pair
of values an equality compares, or a value written for the host, with text read or written
charged by its length, as `docs/nx-ir-format.md` defines them under
*Evaluation cost*; the TypeScript runtime counts the same number and stops at the same place. One
budget covers one call of a method, for `dispatch_component_actions` the whole batch and the render
after it, and each call starts with the whole budget. `restore_component_instance` evaluates no
node and takes no options.

```rust
use nx_ir_runtime::{Program, Result, RuntimeOptions};
use nx_value::NxValue;

fn run_tool(program: &Program, input: NxValue) -> Result<NxValue> {
    let options = RuntimeOptions {
        max_operations: Some(100_000),
        ..RuntimeOptions::default()
    };
    program.evaluate_function("tool", &[input], &options)
}
```

The rules are written so that every step that touches a value in proportion to its size is charged
in proportion, and the time and the memory of a call are proportional to its count. That holds for
everything this crate's tests and the conformance corpus cover, but it is a claim about every step
of the runtime, and review has several times found a step that broke it, each time through a large
or unusual value a host passed at `object`. Treat the budget as the first limit on code you did
not write, and keep one of your own on time and memory as well. The same holds for a
value held in many places too: a record that names one value twice, forty levels deep, costs a few
hundred operations to build and is 2^40 values to anything that walks it, so the type check, the
equality and the conversion for the host that walk it are what pay. The one walk the runtime makes
for itself, the check that state nests no deeper than 256, looks only at the fields a patch
supplies and remembers, for the call, what it found under each shared value. Typed data is
checked each time it meets a type: a list of `n` items returned through `k` calls that declare
their result costs about `k × n`. Two lists are compared item by item up to the first pair that
differs, so two long lists that differ early are cheap to compare; two records are compared field
by field to the end under a budget, so that the cost does not depend on the order fields are held
in. The result is the same either way. A budget failure made for a type check, or for the result
written, names its declaration and has no span, since it belongs to no node.

## Diagnostics

Every API returns `Result<_, NxIrRuntimeError>`. The error holds one or more diagnostics, each
with a `code`, a `message`, the declaration the failing expression belongs to as `identity::name`
when evaluation had reached one, and the expression's span when the image carries its debug
section. An `nx-ir-resource-limit` diagnostic also carries `limit`, a `Limit` holding the name the
table under *Limits* gives and the limit's value, so a host tells an exhausted budget from runaway
recursion without reading the message; no other diagnostic carries one. The runtime never reads a
source file and never panics on an image, a host value or an instance.

| Code | Reported when |
| --- | --- |
| `nx-ir-format`, `nx-ir-schema-version`, `nx-ir-malformed` | The bytes are not an image, are of another schema version, or break the format. |
| `nx-ir-runtime-abi`, `nx-ir-required-feature` | The image needs a runtime ABI or a feature this runtime does not implement. |
| `nx-ir-duplicate-declaration`, `nx-ir-entrypoint`, `nx-ir-reference` | The module's declarations, entrypoints or references are inconsistent. |
| `nx-ir-link-missing-module`, `nx-ir-link-version`, `nx-ir-link-identity`, `nx-ir-link-missing-declaration`, `nx-ir-prelude-image` | Linking failed. |
| `nx-ir-unlinked` | A module that names other modules was used as a program without linking. |
| `nx-ir-missing-entrypoint`, `nx-ir-component` | The name is not a function or component entrypoint, or the instance belongs to another program. |
| `nx-ir-arguments`, `nx-ir-call`, `nx-ir-function-value` | A call has the wrong arguments, or its callee is not a function. |
| `nx-ir-boundary-type`, `nx-ir-boundary-field`, `nx-ir-schema` | A value does not fit the type of the site it reached. |
| `nx-ir-state-field`, `nx-ir-state-patch` | A state patch names an unknown field or is another component's update record. |
| `nx-ir-handler-token`, `nx-ir-handler-result`, `nx-ir-component-action` | A dispatch entry names no handler, a handler returned something that is not a record, or the component does not emit the action. |
| `nx-ir-division-by-zero`, `nx-ir-number`, `nx-ir-operator`, `nx-ir-type` | An operator met an operand it cannot take. |
| `nx-ir-member`, `nx-ir-slot`, `nx-ir-for`, `nx-ir-record`, `nx-ir-union`, `nx-ir-intrinsic` | An expression could not be evaluated as written. |
| `nx-ir-resource-limit` | A limit above was exceeded. |

## Tests

`cargo test -p nx-ir-runtime` runs the conformance corpus in `specs/ir-conformance` (every
entrypoint and lifecycle, with and without debug sections), the damage runs (every truncation of
every corpus image, and every cell of one image overwritten), the preparation and linking
tests, and `tests/allocation.rs`, which counts the bytes a refused concatenation allocates to show
that the budget is charged before a string is built. The tests that compile NX source, including
the differential run against the interpreter, need the compiler and live in `nx-codegen`
(`cargo test -p nx-codegen ir_runtime`).
