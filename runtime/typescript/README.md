# @nx-lang/ir-runtime

Evaluates persisted NX IR in JavaScript, in a browser or under Node, with no compiler and no NX
checkout. The IR is the schema 5 image `@nx-lang/sdk-wasm`, `@nx-lang/sdk-node`, the .NET SDK
and the `nxlang` CLI emit for one module of a compiled program: a binary a runtime reads in place.
This package links images by name and turns them into values.

Use it when JavaScript only needs to *execute* an already compiled program. When JavaScript needs
to *compile* NX source or answer editor queries over it, use `@nx-lang/sdk-wasm`, which emits the
artifacts this package runs.

## Prepare, link, evaluate

An artifact carries one module and a table naming the modules it references. A host prepares each
module once and links an entry module against the prepared modules a resolver supplies:

```ts
import { createNxHost } from "@nx-lang/sdk-wasm";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

// The catalog ships with the host as bytes and is prepared once per page.
const catalog = prepareNxIrModule(catalogBytes);

// Each snippet's image names the catalog and the version it was compiled against. Here the SDK
// emits it; a host that stores images hands over the bytes it stored.
const artifact = host.buildWorkspaceArtifact({
  modules: [{ identity: "input.nx", source }],
  entry: "input.nx",
  implicitImports: ["drawnui.nx"],
});
const [{ bytes }] = artifact.generateNxIr();
const snippet = prepareNxIrModule(bytes);
const program = linkNxIrProgram(snippet, {
  resolve: (identity) => (identity === catalog.identity ? catalog : undefined),
});
const root = evaluateFunction(program, "root");
```

`prepareNxIrModule` takes a `Uint8Array` or an `ArrayBuffer`. The tables are read as 32-bit cells
through typed-array views over the bytes given, so nothing is copied and no table is decoded up
front; a string is decoded the first time it is named. One exception: a `Uint8Array` whose byte
offset is not a multiple of four is copied first, since a `Uint32Array` needs alignment. The SDKs
answer with fresh buffers, so a host that passes their bytes through pays no copy.

Linking checks, before anything is evaluated, that the resolver supplies every module the entry
names, that each one's version equals the version the entry recorded, and that every declaration
the entry references exists in the resolved module. A mismatch is an `NxIrRuntimeError` naming the
module and, for a version, both versions. A host that wants a snippet compiled against catalog
version 9 to run on catalog 10 passes `allowVersionMismatch: true`; the declaration check still
applies, so a control the newer catalog dropped is reported by name rather than failing midway
through drawing. Linking never copies a prepared module, so one prepared catalog serves any number
of programs.

One module never needs a resolver: the NX prelude, `@nx/prelude.nx`, which holds the declarations
every NX module sees without an import — `Range` today. This package ships the compiled prelude of
the compiler release it was built with, and linking serves it for that identity whenever the host's
resolver returns nothing for it, at any depth of the link. A host that wants its own prelude returns
a prepared module for that identity and linking uses that instead. The built-in prelude is prepared
at most once and reused across programs, so nothing is decoded for a program that never reaches it.

An artifact whose module table names only itself — or names only itself and the prelude — is a
program on its own: `prepareNxIrProgram` prepares and links it in one call, and a prepared module of
that shape can be passed to the evaluation APIs directly. A prepared module that names other modules
must be linked first, and the evaluation APIs say so.

`prepareNxIrModule` validates the whole image before it answers: the magic and schema version, the
recorded length against the bytes given, every section, offset array and string range, the string
blob's encoding, the runtime ABI and required features, and every entry of every table against its
layout, so every index the image holds is inside the table it names. What fails is an
`NxIrRuntimeError` naming the problem, and an image from another schema is refused naming both
versions, before anything is evaluated. A truncated or altered image is refused the same way, never
with an exception from inside the reader. `tryPrepareNxIrModule` and `tryLinkNxIrProgram` return
the same as a result instead of throwing.

## Components, instances and dispatch

`initializeComponent` normalizes the props, materializes the state, renders the body and returns
the rendered output, the initial state and an *instance*: an immutable value the host holds and
hands back to `dispatchComponentActions`, which runs a batch and returns the rendered output, the
effects, the next state and the next instance. Every handler in a lifecycle's rendered output is a
record `{ $type: "ActionHandler", action, token }`; the token is how the host names that handler in
a batch, and it is valid only for the instance returned with it. This is the `Counter` of
`examples/nx/component.nx`:

```ts
const { rendered, instance } = initializeComponent(program, "Counter", { step: 2 });
// rendered.children[1].onTapped is { $type: "ActionHandler", action: "Button.Tapped", token: "h1-1" }
const next = dispatchComponentActions(program, instance, [
  { $type: "ActionHandlerInvocation", token: "h1-1", action: { $type: "Button.Tapped" } },
]);
// next.state.count is 2; next.rendered carries tokens h2-1 and h2-2; next.effects is []
const reset = dispatchComponentActions(program, next.instance, [
  { $type: "ActionHandlerInvocation", token: "h2-2", action: { $type: "Button.Tapped" } },
]);
// reset.state.count is 0 and reset.effects is [{ $type: "Reset" }], for the host to route
```

A batch entry is either an action record the component emits, which runs the handler the parent
bound under `on<Emit>` and is a no-op when none was bound, or an `ActionHandlerInvocation` naming a
handler of the instance's most recent output by token, with the action to feed it. Entries run in
order. A handler the component's own body bound reads the state live, so two patches in one batch
compound, and every `<Component>.Update` it returns patches the state; every other returned record
is an effect. A handler bound anywhere else, at the root or in a parent whose content the component
renders, sees only what it captured, and everything it returns is an effect. A failure throws
`NxIrRuntimeError` before anything is returned, and the instance the call was given is unchanged
and still usable. Tokens are `h<generation>-<n>`: the generation is `1` at initialization and one
more after every dispatch, and the number is the handler's position in a walk that visits lists in
order and object keys in sorted order, the same walk the Rust runtime does, so the two runtimes
hand out the same tokens for the same program.

A host that initializes a child from a parent's rendered descriptor passes the descriptor's fields
as props and the parent's instance as `options.parent`:

```ts
const page = initializeComponent(program, "Page");
const { $type, ...props } = page.rendered.children[1];
const searchBox = initializeComponent(program, "SearchBox", props, { parent: page.instance });
const submitted = dispatchComponentActions(program, searchBox.instance, [
  { $type: "SearchSubmitted", searchString: "docs" },
]);
// submitted.effects is [{ $type: "DoSearch", search: "docs" }], the parent's handler's result
```

Every `ActionHandler` record among the props, at any depth, is replaced by the handler the parent
holds under its token; a token the parent does not hold is a diagnostic, and such a record with no
parent is an unknown field. A handler property is never in the child's scope: its body cannot read
`onSearchSubmitted`. The handler substituted is the parent's own value, not a copy: the handler a
child's table holds under one token is the same object the parent's table holds under another, so
a host holding both instances can find the instance whose body created a handler by looking for it
in each ancestor's table.

There is no "update props" operation. A host that has to re-render an instance whose props changed
initializes again with the new props and `options.state` set to the state the instance holds:

```ts
const kept = initializeComponent(program, "SearchBox", { placeholder: "Find" }, { state: instance.state });
```

The state is validated as a complete state for the component, as `evaluateComponent` validates
its state argument, and the new instance's tokens start at generation 1 again.

## Limits

Every evaluation API takes runtime options:

| Option | Default | What it does |
| --- | --- | --- |
| `maxOperations` | Unlimited | The most operations one call may cost. |
| `maxInputSize` | Unlimited | The largest input one call may be given: its arguments, props, content, state, batch and patch. |
| `maxCallDepth` | `100` | The deepest chain of calls one evaluation may make. |
| `maxRangeLength` | `1_000_000` | The most integers one range may hold when a `for` iterates it. |
| `usage` | None | An object the runtime reports what the call used to: `operations` and `inputSize`. |

**Set `maxOperations` for any code you did not write.** It is the only option that bounds work and
allocation; the other two do not. Three nested loops over ranges of a thousand run a billion
bodies inside both, and a function that doubles a list or a string on each of its 100 permitted
calls asks for 2^100 items. Unset, a call is unlimited, so a host that runs only its own programs
sees no change.

An operation is a unit of work on a value:

- a node evaluated;
- an item placed in a sequence a node builds (a list, a loop's result, an element's content), or
  in the list a call binds to a content parameter;
- 64 UTF-16 code units of a string a concatenation produces;
- a value checked against a declared type, where a record is one and then each of its fields;
- a pair of values an equality compares, and 64 code units of text it reads to compare them;
- a value written for the host, and 64 code units of text written.

Text is a string, and also a record's type name and field names, which a host can make as long as
it likes in a value it passes at `object`; declared names are short and cost nothing.

The rules are in `docs/nx-ir-format.md`, under *Evaluation cost*. The count depends on the images
and the input alone, so the Rust runtime, `nx-ir-runtime`, counts the same number and stops at the
same place. One budget covers one call and everything it does: the arguments and props it checks,
parameter and field defaults, value initializers, a component's state defaults and body, for
`dispatchComponentActions` every handler of the batch and the render after it, and the result it
writes. Each call starts with the whole budget, so options can be shared. A charge that would
exceed the budget fails the call before the operation runs, with `nx-ir-resource-limit` naming the
declaration and, for a charge made for a node of an image with a debug section, the span:

```ts
evaluateFunction(program, "tool", [input], { maxOperations: 100_000 });
```

A program costs what it evaluates, what it is given and what it returns: a loop over a thousand
items with a small body is a few thousand operations, and so are a thousand rows passed in or
returned. As a rough guide to what a budget buys, `for i in 0..1000000 { i * i }` is five million
operations, four million to run and one million to write its result, and takes about 130 ms in
Node 24 on a current laptop, some 40,000 operations a millisecond.

The rules are written so that every step that touches a value in proportion to its size is charged
in proportion, and the time and the memory of a call are proportional to its count. It is a claim
about every step of the runtime, and review has several times found a step that broke it, each
time through a large or unusual value a host passed at `object`. Four things back it now: the
input limit, which bounds what a step nobody has found can cost; a differential test that runs
generated host values through this runtime and the Rust one and requires the same count, input
size and failures; bounds on the Rust runtime's allocations for every generated case; and a time
report, which does not block a merge, of cases whose time grows faster than their count. What is
still open is the list of known findings in `crates/nx-codegen/tests/cost/mod.rs`, and steps
driven by a value a program builds, which only fixed probes exercise; `docs/nx-ir-format.md` has
the details under *Validation against generated host values*. So treat the budget as the first
limit on code you did not write, set `maxInputSize` beside it to bound what you hand that code
(see *The input limit* below), and keep a limit of your own on time and memory as well where you
have one: a CPU limit, or a separate isolate. The same holds for a
value held in many places too: a record that names one value twice, forty levels deep, costs a few
hundred operations to build and is 2^40 values to anything that walks it, so the type check, the
equality and the result that walk it are what pay. Two things follow that are worth knowing when
choosing a budget:

- Typed data is checked each time it meets a type. A list of `n` items returned through `k` calls
  that declare their result costs about `k × n`, and nested records are checked again by each
  record built around them.
- Two lists are compared item by item up to the first pair that differs, so two long lists that
  differ early are cheap to compare. Two records are compared field by field to the end under a
  budget, even after one field differs, so that the cost does not depend on the order fields are
  held in. The result is the same either way.

A value that is not a non-negative safe integer, `NaN` included, is refused with `nx-ir-options`
before anything is evaluated, never read as unlimited.

### The input limit

`maxInputSize` bounds what the host hands one call. The *input size* is defined in
`docs/nx-ir-format.md`, under *Input size*, and is measured the way a value written for the host
is charged: one for each value, a list, an empty one and a `null` included, and one more for every
64 UTF-16 code units of a string, of a `$type` and of each field name. It covers the arguments of
`evaluateFunction`; the function record and the arguments of `callFunction`; the props and the
content of `constructComponentDescriptor`; the props of `initializeComponent` and the `state` its
options carry; the props and the state of `evaluateComponent`; the batch of
`dispatchComponentActions`; the state of `normalizeComponentState`; and the state and the patch of
`applyComponentStatePatch`. Props, a state, a patch and arguments by name are each measured as one
record, and props left out are an empty record, which is one. An instance, and the `parent` of
`initializeComponent`, are not input: the runtime made them.

```ts
evaluateFunction(program, "tool", [input], { maxOperations: 100_000, maxInputSize: 1_000 });
```

A call whose input is larger fails with `nx-ir-resource-limit` whose `limit` is
`{ name: "maxInputSize", value }`, before the program is looked at and before any value is checked
against a type, so it is reported ahead of any other fault of the call, and a batch is measured
whole before its first entry runs. The diagnostic names no declaration. Measuring stops as soon as
the size passes the limit and does not recurse, so a value that is too large, that nests deeply or
that holds itself is refused by the limit and not by the engine. A value that is no canonical
value, a `Date`, a `Map`, a function, an instance of a class, counts as one and is not entered.
Unset, nothing is measured. The limit charges nothing to `maxOperations`, and a value that is not a
non-negative safe integer is refused with `nx-ir-options`.

Choose it together with the budget. Every step the runtime takes is meant to be charged in
proportion to the values it touches, and the limit is there for one that is not and that nobody
has found: with a budget `B` and a limit `C`, a step that does work in proportion to a host value
for a fixed charge costs at most about `B × C` in one call. At `maxOperations: 100_000` and
`maxInputSize: 1_000` that is about 10^8 values or 64-code-unit pieces of text, seconds at the
worst, where without a limit it had no ceiling. A tool's arguments are tens to hundreds of values,
so a limit in the low thousands refuses nothing ordinary. What the limit does not bound: a step
driven by a value the program builds itself, which is bounded by the budget squared; the state an
instance holds, which is bounded only when every call that wrote it had a budget; and the one
listing of an object's names that refusing a very wide object costs, about half a second for two
million names. So keep the limit on time and memory as well.

`measureInputSize(value, limit?)` measures one value as the limit does, without a call, so a host
can hold one part of what it passes, a model's arguments say, to a number of its own. With a limit
it stops as soon as the size passes it and returns some number greater than the limit. An object
is measured as the one record props, a state, a patch and arguments by name are; an array measured
as one value is one more than its items add to a call that takes them as arguments, content or a
batch. Pass a limit for any value that did not come from JSON: a value that holds itself has no
finite size, and measuring one with no limit does not return.

```ts
if (measureInputSize(modelArguments, 1_000) > 1_000) {
  return { error: "invalid-input", message: "The arguments are too large." };
}
```

### What a call used

Give a call a `usage` object and the runtime reports what it used: `operations` when
`maxOperations` is set, and `inputSize` when `maxInputSize` is set and the input was within it.
The runtime removes both members when the call begins and sets them when it ends, whether it
returns or throws, so the object never carries an earlier call's numbers. For a call that returns,
`operations` is its count, the least budget it succeeds under; for one that throws it is what was
charged before the failure, never more than the budget.

```ts
const usage: NxRuntimeUsage = {};
try {
  return evaluateFunction(program, "tool", [input], { maxOperations: 100_000, maxInputSize: 1_000, usage });
} finally {
  console.log(`tool used ${usage.operations} operations on an input of ${usage.inputSize ?? "more than 1000"}`);
}
```

Nothing is counted for the report alone: with no budget `operations` stays absent, so a host that
wants the count and no limit sets a budget it cannot reach. That is how to choose a budget from
measurement, by running the programs you mean to allow and reading what they cost:

```ts
const usage: NxRuntimeUsage = {};
let most = 0;
for (const input of representativeInputs) {
  evaluateFunction(program, "tool", [input], { maxOperations: Number.MAX_SAFE_INTEGER, usage });
  most = Math.max(most, usage.operations!);
}
const maxOperations = most * 4; // headroom over the largest count measured
```

The object must be one the runtime can write to: a frozen or non-extensible object, or a value
that is no object, is refused with `nx-ir-options` before anything runs. A write that fails when
the call ends is dropped, so it never replaces what the call returned or threw. Calls that overlap
and share one object, tool calls started together and awaited later, leave the numbers of
whichever ended last; give each call its own.

A range makes an enormous loop one token long — `for i in 0..2000000` is four tokens — so the count
is checked before the body runs at all, and a range above the limit fails with
`nx-ir-resource-limit` naming the limit. The default admits any loop a program is likely to write on
purpose. A host with a legitimately larger loop raises the limit:

```ts
evaluateFunction(program, "rows", [], { maxRangeLength: 5_000_000 });
```

Two limits are not options. Expressions may nest at most 1,000 deep across every call of one
evaluation, the bound the Rust runtime holds, so raising `maxCallDepth` does not let recursion reach
the engine's stack. And a `RangeError` the JavaScript engine raises during evaluation — a call
stack, a string or an array past what the engine holds — is reported as `nx-ir-resource-limit`
rather than thrown as itself. That failure depends on where the runtime runs, which its limit's
name, `engine`, says.

## Diagnostics

A runtime diagnostic names the declaration the failing expression belongs to, as
`identity::name`. When the artifact carries its debug section, which the CLI writes and the SDKs
emit on request, the diagnostic also carries the expression's span in the module's source. The
runtime never reads a source file.

Every `nx-ir-resource-limit` diagnostic carries `limit`, the limit that was reached, so a host tells
an exhausted budget from runaway recursion without reading the message:

| `limit.name` | `limit.value` | Reached when |
| --- | --- | --- |
| `maxOperations` | The budget | The call cost more operations than its budget. The diagnostic has no `source` when the charge was for a type check or for the result written, which belong to no node. |
| `maxInputSize` | The limit | The host passed the call more input than the limit allows. Nothing was evaluated, so the diagnostic has no `declaration` and no `source`. |
| `maxCallDepth` | The depth | Calls nested deeper than the option allows. |
| `maxRangeLength` | The length | A loop's range holds more integers than the option allows. |
| `maxExpressionNesting` | `1000` | Expressions nested deeper than the fixed bound. |
| `engine` | none | The JavaScript engine refused: its stack, or a string or an array too long for it. |

No other diagnostic carries `limit`. Runtime options the runtime cannot use are refused with
`nx-ir-options` before anything is evaluated.

## Exports

| Export | What it does |
| --- | --- |
| `prepareNxIrModule`, `tryPrepareNxIrModule` | Validate and index one artifact. |
| `linkNxIrProgram`, `tryLinkNxIrProgram` | Link a prepared entry module against resolved modules. |
| `prepareNxIrProgram`, `tryPrepareNxIrProgram` | Prepare and link a self-contained artifact. |
| `evaluateFunction` | Evaluate a function entrypoint by name with positional arguments. The arguments may stop before trailing parameters that are optional or have a default; the function fills those itself. |
| `constructComponentDescriptor`, `initializeComponent`, `evaluateComponent` | Build a component's descriptor, initialize it into an instance, and evaluate it from explicit state. |
| `dispatchComponentActions` | Run a batch of actions and handler invocations against an instance. |
| `callFunction` | Call the function a `{ $type: "Function", module, name }` record names — a rendered template, say — with arguments keyed by parameter name; an argument the function does not declare is dropped, a parameter it declares and the arguments lack is a diagnostic naming it. |
| `NxFunctionRecord` | The type of that record: `$type` the literal `"Function"`, `module` and `name`. It is what a member declared at a function type renders as, and what a host supplies there. |
| `normalizeComponentState`, `applyComponentStatePatch` | Bring component state into its declared shape and apply a patch. |
| `measureInputSize` | The size of one value as `maxInputSize` measures it, without a call; given a limit, it stops as soon as the size passes it. |
| `NxRuntimeOptions`, `NxRuntimeUsage` | The options every evaluation function takes, and the type of the `usage` object among them that the runtime reports a call's `operations` and `inputSize` to. |
| `applyUpdate`, `mergeUpdates`, `diffRecords`, `changedFields` | Record update arithmetic over host-held values. |
| `float32Text` | The canonical text of a `float32` carried as a `number`: the shortest digits that round-trip as a `float32`, which is what a `text` node naming `float32` prints. |
| `NX_IR_SCHEMA_VERSION`, `NX_IR_RUNTIME_ABI` | The schema and ABI this runtime accepts. |
| `NX_IR_REQUIRED_FEATURE_*` | The required features this runtime knows, including `ranges-v1` for iteration over a range. An image listing a feature this runtime does not know is refused by name. |
| `NX_PRELUDE_MODULE_IDENTITY`, `NX_DEFAULT_MAX_CALL_DEPTH`, `NX_DEFAULT_MAX_RANGE_LENGTH` | The prelude's reserved identity, and the defaults of the call-depth and range-length limits. |
| `nodeKinds`, `typeKinds`, `constantKinds`, `declarationKinds` | The kind numbers of the schema. |
| `NxIrRuntimeError` | Thrown for an artifact the runtime cannot run, with its diagnostics. |
| `NxIrLimit` | The type of a resource-limit diagnostic's `limit`: a `name` and, for a numeric limit, its `value`. |

The opened image (`NxIrImage`), the prepared types (`NxPreparedModule`, `NxPreparedProgram`,
`PreparedDeclaration`) and the instance (`NxComponentInstance`) are exported so a host can read the
program it runs and type what it holds; the instance's fields are the runtime's, not an API. The image's layout is
documented in `docs/nx-ir-format.md`; `nxlang ir explain`, or `explainNxIr` from either SDK,
renders one as text.

A host that needs a function's types or documentation, to describe it to a language model or an MCP
client say, does not read them from the prepared declarations: the image erases generic type
arguments, records no alias targets, carries a function's result type only when the source declares
one, and holds no doc comments. It derives JSON Schema with a compiler SDK when it compiles, through
`functionSchema` and `typeSchema` on a program artifact from `@nx-lang/sdk-wasm` or
`@nx-lang/sdk-node`, and stores the schemas with the image. A schema derived from the same artifact
as the image agrees with this runtime's boundary validation: arguments valid against it are
accepted, and what the function returns is valid against its result schema.

## Integers outside the safe range

A JavaScript number holds integers exactly only up to 2^53, and this runtime carries every number as
one. So it cannot hold a larger integer exactly, and a program that reaches a literal outside the
safe range fails there with `nx-ir-number`, naming the function and the literal. Only literals are
refused: arithmetic whose result passes 2^53 gives the nearest number a double holds, as JavaScript
does, and a number a host passes at a parameter typed `int` is taken as the number it is. The image
still prepares, and every path that does not reach the literal runs. The Rust runtime, which has
64-bit integers, runs the same program.

Canonical JSON spells such an integer as `{ "$type": "nx.int", "value": "<digits>" }`. A host that
passes that record at a parameter typed `object` passes a record like any other: it is returned
unchanged and what it holds is paid for like any record's fields. At a parameter typed `int` it is
refused, as any value that is not a number is.

## Function records

A member declared at a function type renders as `{ $type: "Function", module, name }`, and
`callFunction` calls the function it names with arguments keyed by that function's own parameter
names, each validated against the parameter's declared type:

```ts
import { callFunction, evaluateFunction, type NxFunctionRecord } from "@nx-lang/ir-runtime";

// type Tool = { fn: <function ... />: object* }
// let double(n:int): int = {n * 2}
// let root() = <Tool fn={double} />
const tool = evaluateFunction(program, "root") as { fn: NxFunctionRecord };
callFunction(program, tool.fn, { n: 4 }); // 8
```

A member typed `<function ... />: R` takes a function of any parameters. Supplied by a host, the
record must name a function declaration of the linked program; the runtime does not compare that
function's result with `R`, so a host that accepts records it did not read from the program checks
the value the function returns.

Read function records only from members declared at a function type. An element named `Function`
renders a record of the same shape, and at an `object`-typed member nothing tells the two apart or
resolves the record against the program.

A host that gives such functions to an AI model as tools can use
[`@nx-lang/agent`](../../packages/agent), which is unstable. It is built on `callFunction`, and
adds each tool's JSON Schemas, a budget and input limits for every call, and results that say why
a call failed.

## Versions

Install this package and `@nx-lang/sdk-wasm` at the same release version. The two are released
together from one tag, and the artifacts the SDK emits at that version satisfy this runtime's
format, schema and feature checks. The repository's tests run every SDK's emitted IR and the
conformance corpus through this runtime on every build.

## Layout

| Path | What it holds |
| --- | --- |
| `src/index.ts` | The whole runtime |
| `test/runtime.test.ts` | Preparation, linking and boundary tests over images written by the test |
| `test/corpus.test.mjs` | Evaluates every image of `specs/ir-conformance` against the interpreter's results, drives every lifecycle it names, and refuses every truncation and cell overwrite of them |
| `test/emitted-ir.test.mjs` | Compiles NX through the CLI and runs the emitted IR, comparing with the native evaluator |
| `test/cost-runner.mjs` | This runtime's side of the cost validation: run by the harness in `crates/nx-codegen/tests/cost_differential.rs` over generated cases, not by `pnpm test` |
