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

## Host values

What a host passes to an evaluation API, and what it gets back, are plain JavaScript data: `null`,
a boolean, a number, a string, an array, or a plain object, which is an object whose prototype is
`Object.prototype` or that has none. That is the JavaScript form of an NX value, and
`docs/nx-ir-format.md` (*Host values*) has the whole of it. JSON is one way to get such a value
and not the only one: an object the host built a moment ago is passed exactly as one it read with
`JSON.parse` is, with the same result and the same cost, and nothing is encoded on the way in.

```ts
// `greet` is declared `let greet(person:Person): string`, with
// `type Person = { name:string nickname?:string title:string = "Dr" }`.
evaluateFunction(program, "greet", [{ name: "Ada" }]);
evaluateFunction(program, "greet", [JSON.parse('{ "name": "Ada" }')]); // the same call
evaluateFunction(program, "greet", [{ name: "Ada", nickname: undefined }]); // and so is this
```

A member of a plain object that is `undefined` is a member left out, at any depth, which is what
an optional property of a TypeScript type produces. A field with a default takes its default, an
optional field is empty, and a required field is reported missing. The program is not given the
member, and the host's object is not changed: when a member is left out, the objects on the way
down to it are copied without it, and everything else is passed as it is.

Anything else is refused where it is passed, with `nx-ir-boundary-type` and the path to it: an
instance of a class, a `Date`, a `Map`, a `Set`, a typed array, a function, a symbol, a big
integer, and an item of an array that is `undefined`, which cannot be left out. That holds at any
depth, inside a value typed `object` too, and in every value the input limit covers. The runtime
converts none of these: a host that has a `Date` passes the text or the number it means, and one
that has an instance of a class spreads it.

```ts
class Person { name = "Ada"; }
// TypeScript refuses an instance where a host value is expected. Past a cast, the runtime does:
evaluateFunction(program, "greet", [new Person() as never]);
// nx-ir-boundary-type: Expected person to be null, a boolean, a number, a string, an array or a
// plain object, got an instance of Person, which is not a plain object.
evaluateFunction(program, "greet", [{ ...new Person() }]); // a plain object
```

What holds the values is read by the same rule: positional arguments, content and a batch are
each an array, and props, a state, a patch and arguments by name are each a plain object. The
evaluation functions take their input as `NxHostValue` and `NxHostRecord`, which admit a member
that is `undefined`, and return `NxCanonicalValue`, which has none, so what the runtime returns
can always be passed back.

The values the runtime returned are accepted back as they are, a state that holds a function value
included. The input is read once in a call, after the input limit and before anything is checked
or evaluated, and reading it charges nothing to `maxOperations`. An object that holds itself is
refused the same way. The same object held twice, side by side, is two values, read and measured
once for each place it is held: a value that shares one object in many places is as large as the
tree it spells, and `maxInputSize` is what bounds the reading of it, not `maxOperations`.

**What reading costs.** Plain data with nothing to leave out is checked in one pass and used as
it was passed, with no copy: up to about 25 ns for a record of three fields, measured by the
`input` phase of the harness below (*Performance*) on one machine. A value the check does not
settle is read again by a slower walk that keeps track of where it is and of the objects it is
inside, at about 100 ns for each record. Three things send a value there: a member that is
`undefined`, anywhere in it, nesting deeper than 32 levels, and an object made in another realm,
a `vm` context or a frame. It is the whole value that takes the slower walk, not the part that
caused it. A positional argument, an argument by name, an item of content and an entry of a batch
are each a value of their own; props, a state and a patch are each one value, so one `undefined`
member anywhere in a state sends the state. None of this is
noticed on a call of ordinary size, where it is microseconds. A host that passes thousands of
records on a path that runs often keeps to the fast one by leaving an absent member out instead
of setting it to `undefined`:

```ts
const row = { id, label, ...(note === undefined ? {} : { note }) }; // not { id, label, note }
```

The slower walk can be made cheaper, and has not been. `specs/future.md` (*Reading a host value
costs up to 25 ns a record, and about 100 on the slow path*) lists how: it can drop its set of
the objects it is inside when `maxInputSize` has already measured the value, since a value within
the limit holds nothing that holds itself; it can read an object's members as the fast check does
instead of listing their names first; and the input measure can report that it met only plain
data with no `undefined` member, which would let a call that sets a limit skip the reading
altogether.

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
that holds itself is refused by the limit and not by the engine. A member that is `undefined` is
a member left out and counts nothing. A value that is no canonical value, a `Date`, a `Map`, a
function, an instance of a class, counts as one and is not entered; a call within the limit then
refuses it (see *Host values*).
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

A diagnostic for a failure in an argument of the function a host called, through `callFunction`
or `evaluateFunction`, carries `argument`: the declared name of the parameter the value was passed
for, whether the arguments were passed by name or by position.

```ts
// `findPlans` is declared `let findPlans(teamSize:int, note?:string): string`.
const findPlans = { $type: "Function", module: "main.nx", name: "findPlans" };
try {
  callFunction(program, findPlans, { teamSize: "five" });
} catch (error) {
  if (error instanceof NxIrRuntimeError) {
    const [diagnostic] = error.diagnostics;
    console.log(diagnostic.code, diagnostic.argument); // nx-ir-boundary-type teamSize
  }
}
```

It is set for a value that does not fit its parameter's type, at any depth inside the value
(`nx-ir-boundary-type`, `nx-ir-boundary-field`), for a value inside it that is no host value at
all, a `Date` say (`nx-ir-boundary-type`, see *Host values*), for a `Function` record in the value that names
no function (`nx-ir-function-value`), and for a required parameter given nothing
(`nx-ir-arguments`). It says the failure is in what the host passed, so nothing else carries it,
even when the runtime finds the failure while it checks an argument:

- a failure a default raises, a parameter's or a record field's, by its expression, by a function
  it calls or by its value not fitting its type: a record's defaults are filled in while the
  argument that holds the record is checked, and they are the program's own;
- a resource limit, the budget spent while an argument is checked included;
- a failure in the function's body or in its result, more positional arguments than the function
  has parameters, and the `Function` record `callFunction` is given to say which function to call;
- a failure of any other entry point.

The Rust runtime names the same argument for the same call.

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

## Performance

`bench/` holds a harness that times each call a host makes, on programs of the size hosts run. It
is not part of the published package; run it from a checkout.

```bash
pnpm --filter @nx-lang/ir-runtime bench          # every call, cold and warm, as a table and as JSON
pnpm --filter @nx-lang/ir-runtime bench:compare  # this tree against the revision it is based on
```

**What is timed.** Two programs of the conformance corpus, read from their committed images, so
nothing is compiled: `large-catalog`, a snippet against a library of 45 external components, and
`question-flow`, a component with state over a library of 31 question kinds, whose script answers
30 questions. A third workload makes calls that take input, against functions the corpus already
has.

| Phase | The call |
| --- | --- |
| `load` | Importing the runtime module. Cold only. |
| `prepare` | `prepareNxIrModule` for every image of the program |
| `link` | `linkNxIrProgram` of the entry against the prepared modules |
| `evaluate` | `evaluateFunction` of a function that takes no arguments |
| `initialize` | `initializeComponent` with props |
| `resume` | `initializeComponent` with `options.state`, a state stored after 15 answers: what a host that keeps only the state does |
| `evaluate with state` | `evaluateComponent` with props and that state |
| `dispatch` | `dispatchComponentActions` with one answer: the 30 answers in order, reported for one |
| `input` | `evaluateFunction` given 10,000 records: at `object` as plain data, with one member `undefined` in the last record, and inside 33 objects; and as records of a declared type, plain and with one member `undefined` |
| `tool call` | `callFunction` by name with arguments, and with a context record |

Every phase that evaluates is timed twice: with no limits, and with `maxOperations`,
`maxInputSize` and `usage` set to values the call does not reach. The difference is what setting
limits costs, and the second is where the report's operations and input size come from. A count
is exact and the same on every machine; where the corpus records one, the harness's test checks
that the report gives it.

**What a number means.** *Warm* is a call after it has run until its time stopped falling: the
median and the 95th percentile of 200 samples, a call shorter than 0.1 ms being sampled in
batches. Each call is warmed and timed in an isolate of its own (a `worker_threads` worker), so
no call runs on what another taught the engine; a host's isolate runs a mix of calls, and its
times are not below these. An isolate now and then settles on slower code for a call than the
others do, so `bench` times each call in three and reports the one whose median is in the
middle. *Cold* is the first execution of a call in a fresh isolate after the
runtime module was loaded there, over 15 isolates: it includes the engine compiling the runtime's
code for that call. It approximates the first request an isolate serves and is not another
engine's cold start. `ns/op` is the warm time with no limits divided by the operations, which is
the number to compare across phases.

**Comparing two revisions.** `bench:compare` answers whether a change made a call slower. It
reads the base revision's committed build (`dist/src`) and its corpus images from Git, the merge
base with `origin/main` unless `--base <ref>` names another, and in each of 7 rounds times every
call once for the base and once for the working tree, one straight after the other. It compares
three kinds of time:

| Time | What it is | Named beyond |
| --- | --- | --- |
| Warm | A call's warm median in a round, with no limits and with limits | 7% |
| Load | The time the runtime module takes to load in a fresh isolate, the median of a round's isolates | 7% |
| Cold | A call's first execution in a round's fresh isolate, one sample a round | 20% |

A step is named as slower, or faster, when the median of its seven ratios is beyond that fraction
and at least 6 of the 7 rounds are beyond half of it the same way. One round's ratio is noisy and
the median of seven is not: a build compared with itself forty times, on a desktop and on GitHub
runners, was named for none of 1,200 warm, 340 cold and 20 load steps, and the same ratios made
10% larger are named 84% of the time for a warm step and 15% larger 96%; a cold step has to be
30% slower to be named nine times in ten. So a slowdown of a few percent passes, which is what
the comparison with the last release, below, is for.

The command exits with 1 when a step is slower, and with 2 when it could not compare at all.
Both operation counts are in the report, so a call that costs more operations shows without any
noise. A call is reported as not comparable when its program's sources differ between the two
revisions, when the base has no such program, when the base's runtime lacks a function the call
uses, or when the call fails on either side. Nothing measured on another machine is kept or
compared with, and there is no baseline file. `--base-runtime <dir>` and `--head-runtime <dir>`
compare two build directories directly.

CI runs the comparison in a job that reports in its summary and does not block a merge: on every
pull request against the revision the change is based on, and on every push to `main` against
the last release (`--base v<x.y.z>`), which is what names several small slowdowns that no one
pull request was named for. The release steps in `docs/deployment.md` say to read it before a
tag.

**Running the steps in another engine.** `bench/core.mjs` builds the steps and is the only part
a host needs. It loads no module, reads no clock and uses nothing of Node, and it is handed the
runtime, so it runs the host's own bundled build:

```js
import * as runtime from "@nx-lang/ir-runtime";
import { buildSteps } from "./core.mjs";

// `corpus` holds each program of `PROGRAMS` by name, read however the host reads files:
// { manifest: <its program.json>, images: { <module identity>: <its .stripped.nxir as a Uint8Array> } }
const steps = buildSteps(runtime, corpus);
const step = steps.find((step) => step.phase === "dispatch" && step.variant === "unlimited");
step.ready(); // prepares, links and initializes, outside what is timed
for (let run = 0; run < 1000; run += 1) {
  step.run(); // the 30 dispatches; `step.divisor` is 30
}
```

How to time them is the host's to decide. In a deployed Cloudflare Worker `performance.now()` and
`Date.now()` do not advance while code runs, so a loop like this one is timed from outside: by
the caller of a request that runs it a known number of times, or from the CPU time the platform
reports for the request.

## Layout

| Path | What it holds |
| --- | --- |
| `src/index.ts` | The whole runtime |
| `test/runtime.test.ts` | Preparation, linking and boundary tests over images written by the test |
| `test/corpus.test.mjs` | Evaluates every image of `specs/ir-conformance` against the interpreter's results, drives every lifecycle it names, and refuses every truncation and cell overwrite of them |
| `test/emitted-ir.test.mjs` | Compiles NX through the CLI and runs the emitted IR, comparing with the native evaluator |
| `test/cost-runner.mjs` | This runtime's side of the cost validation: run by the harness in `crates/nx-codegen/tests/cost_differential.rs` over generated cases, not by `pnpm test` |
| `bench/core.mjs` | The steps the performance harness times; runs in any JavaScript engine |
| `bench/run.mjs`, `bench/compare.mjs` | The Node drivers behind `bench` and `bench:compare`, with `measure.mjs`, `sample.mjs`, `worker.mjs`, `corpus.mjs`, `report.mjs` and `verdict.mjs`, the rule a step is named by |
| `bench/core.test.mjs` | Runs every step once against the committed build and checks the counts the corpus records, that the corpus's generated catalog is what its script writes, and the rule a step is named by |
