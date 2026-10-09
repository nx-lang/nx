# nx-ir-runtime

The NX IR runtime for Rust. It prepares NX IR images, links them by name and evaluates them, with
the same results, handler tokens and diagnostic codes as the TypeScript runtime
`@nx-lang/ir-runtime`.

It depends on `nx-ir`, the image format, and `nx-value`, the host value type, and on no part of
the NX compiler. A host that holds compiled images runs them with this crate alone.

```toml
[dependencies]
nx-ir-runtime = "0.7"
nx-value = "0.7"
```

The three crates are versioned with the NX release they ship in and require each other at exactly
that version, so name `nx-value` at the version you name `nx-ir-runtime` at. While NX is `0.x`
the API may change between minor versions.

The image format is documented in [`docs/nx-ir-format.md`](https://github.com/nx-lang/nx/blob/main/docs/nx-ir-format.md). Images
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

Values cross the API as `nx_value::NxValue`, which is the Rust form of a canonical value
(`docs/nx-ir-format.md`, *Host values*). A host passes one as it built it, whether it read it from
JSON or computed it; nothing is encoded on the way in or out, and an `NxValue` can hold nothing
that is not a value.

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

## Instance tree

A component's rendered output can hold descriptors of other components the entry module declares:
a `Page` renders a `<Card />`, and what the host is handed is a record whose type is `Card`. Each
needs an instance of its own, initialized through the instance whose output held it, so that a
handler the parent bound reaches the child. The lifecycle above renders one instance at a time and
composes nothing. `InstanceTree` is the composition, made only of `initialize_component` and
`dispatch_component_actions`, so that a host with nested components does not write it.

The tree does not know what the host does with the output. A host may draw it; it may as well keep
a document, an index or any other structure derived from it, and update that as the output
changes. Either way the host names each node by a key of its own choosing for the node's place in
the output, and the node it was found under by that node's key; it reads what a node rendered,
interprets the records that are its own, and visits the authored descriptors it finds.

```rust,no_run
use nx_ir_runtime::{InstanceTree, NxIrRuntimeError, Program, Result, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;

/// Walks `value`, visiting every authored descriptor in it. `key` names the value's place in
/// the output, and `owner` is the node whose rendered output the value is part of.
fn walk(
    tree: &mut InstanceTree,
    value: &NxValue,
    key: &str,
    owner: Option<&str>,
    options: &RuntimeOptions,
) -> Result<()> {
    match value {
        NxValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(tree, item, &format!("{key}/{index}"), owner, options)?;
            }
        }
        // A component the entry module declares with a body, which is what the lifecycle can
        // initialize: its node, and what the node rendered, which stands in its place.
        NxValue::Record { type_name: Some(name), .. } if tree.can_instantiate(name) => {
            let rendered = tree.visit(key, value, owner, options)?;
            // Nothing at this node or under it has rendered since the last pass: what the host
            // made of it still stands, and the pass keeps everything under it unvisited.
            if !tree.is_settled(key) {
                // ... update whatever the host keeps for this node from `rendered` ...
                walk(tree, &rendered, &format!("{key}/body"), Some(key), options)?;
            }
        }
        // Anything else is the host's to interpret. A handler among its properties carries a
        // token of `owner`'s output: an event on it is `tree.dispatch(owner, token, action, ..)`.
        NxValue::Record { properties, .. } => {
            for (name, property) in properties {
                walk(tree, property, &format!("{key}/{name}"), owner, options)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// One pass, as one change to the tree: a pass that fails leaves it as it was.
fn pass(tree: &mut InstanceTree, root: &NxValue, options: &RuntimeOptions) -> Result<()> {
    tree.atomically(|tree| {
        tree.begin();
        walk(tree, root, "root", None, options)?;
        tree.finish();
        Ok::<_, NxIrRuntimeError>(())
    })
}

fn run(program: Program) -> Result<()> {
    let options = RuntimeOptions::default();
    let root = program.evaluate_function("root", &[], &options)?;
    let mut tree = InstanceTree::new(program);
    pass(&mut tree, &root, &options)?;

    // An event, named by the node whose output held the handler and the token read there.
    let tapped = NxValue::Record {
        type_name: Some("Button.Tapped".to_string()),
        properties: BTreeMap::new(),
    };
    for effect in tree.dispatch("root", "h1-1", tapped, &options)? {
        // What no component above the one that produced it bound a handler for.
        println!("{} emitted {:?}", effect.component, effect.action);
    }
    // Then a pass, before the next event. It walks from the root to the nodes that rendered
    // and what is under them, and nothing else.
    pass(&mut tree, &root, &options)
}
```

What the tree does:

- **A node is one use of a component at one place.** The first `visit` at a key initializes the
  component from the descriptor's fields, through the parent's instance. A descriptor of another
  component at that key, or the same key under another parent, replaces the node, and the nodes
  under it, with a new one in its initial state.
- **Props flow down and state stays.** A visit with the descriptor the node was last initialized
  from, read from the same instance of the parent, does nothing. Any other descriptor initializes
  the node again with the state it holds. A descriptor from a parent that was initialized again is
  another descriptor even where it reads the same: a render numbers its tokens from one, so two
  renders can spell different handlers alike.
- **An instance lives as long as its place in the output.** `finish` drops what a pass left
  unvisited under a node it visited that had rendered since the last pass: the host has just
  walked that node's new output and did not find the place in it. Under a visited node that had
  not rendered, and under a node the pass never reached, everything is kept, visited or not. A
  node under no node is dropped unless the pass visits it. `remove` drops a node the output still
  has a place for and the host no longer wants, such as an item instantiated only while in view.
  An event may be dispatched while a pass is under way. A visit then counts for what the node held
  when it was made: a node that rendered again after its visit is still to be walked, and a node
  under it that was visited only before that was read from output that is gone.
- **Nesting ends.** A node may be `max_component_depth` deep, 100 unless the host says otherwise,
  counting itself and the nodes above it. A `visit` for a deeper one fails with
  `nx-ir-resource-limit` naming `maxComponentDepth` and changes nothing. Each visit is a call of
  its own, so this is the limit a component that renders itself meets, and a pass run inside
  `atomically` then leaves the tree as it was.
- **A pass walks only what changed.** `is_settled` says of a node that neither it nor anything
  under it has rendered since a pass last visited it, so a pass need not go below it. After a
  dispatch deep in the tree, a pass visits the nodes from the root down to the one that rendered,
  and then everything under that one. While a node does not render again, `rendered` hands out the
  same allocation, so `Arc::ptr_eq` tells a host whether what it last read there still stands.
- **A handler runs where it was bound.** `dispatch` runs the handler against the node whose body
  created it: the node named, or the ancestor farthest from it that holds the same handler, which
  is where a handler passed down through content or props came from. A `Page`'s button rendered
  inside a `Stack`'s output patches the `Page`. The same handler is the very one that was handed
  down: a component nested under itself, with equal props and state, is still two instances.
- **Run a pass after every dispatch.** A dispatch gives each node it ran against a new instance,
  and the nodes under it hold handlers of the old one until a pass visits them again. An event
  that reaches the tree before that pass, for a handler handed down from an instance since
  replaced or for an emit whose bound handler was, fails with `nx-ir-handler-token` and changes
  nothing. A host that queues input runs a pass between two events, or runs one and sends the
  refused event again. It is never run against a node that was only handed the handler, and its
  emit is never returned to the host as though no parent had bound it.
- **Take the new tokens when the pass ends, even if drawing waits.** The node a dispatch ran
  against has rendered again, so the tokens its old output carried are retired at once. A host
  that draws later than it dispatches (after the handler returns, on the next frame) must not also
  leave the tokens its controls hold until then: a scroll or a drag reports several events within
  one frame, and the second would name a token the first retired. Read the handlers of the new
  output into what the controls dispatch with as part of the pass, and leave only the drawing for
  later.
- **An emitted action goes to the handler the parent bound.** Each effect a dispatch returns whose
  type the node's component emits, and that the parent bound a handler for, is dispatched in turn
  against the node that created that handler. Every other effect comes back as a `HostEffect`. A
  node is dispatched against at most once in a chain, with everything routed to it in one batch.
- **A change is all or nothing.** A `dispatch` that fails anywhere in its chain leaves every node
  as it was, so the tokens the host holds still name handlers. `atomically` gives a pass, or a
  dispatch and the pass after it, the same guarantee.
- **A handler bound outside any component is inert.** A descriptor visited under no node came
  from pure evaluation, so a handler record in it has no token and names nothing. The tree leaves
  such a handler out of what it initializes the node from and lists it in `inert()` for the pass,
  as `Card.onLogged`.

What it does not do:

- **It does not walk.** The tree never looks into rendered output for descriptors, and it holds
  no node the host did not visit. The host walks, because the host decides what a place is: which
  records are its own, what key a list item has, and whether an item out of view has an instance
  at all.
- **The unit of change is a component.** There is no tracking of which state a body read. A
  dispatch renders the whole body of each component it runs against. A node that renders hands
  every node under it a descriptor read from another instance, so each is initialized again, with
  the state it held, when the pass visits it, and so on down: a render reaches everything under
  the node that rendered, whether or not the props it handed down changed. What is beside that
  node, and above it unless an emit carried a change there, does not render.

### Cells a host binds outside a pass

A host that draws a virtualized list does not walk the list's cells. Its layout engine realizes a
cell when it scrolls into view and asks the host to fill it then, between passes. A component in
that cell cannot be a node of the drawing's tree: no pass visits it, so the next `finish` would
drop it as a node under no node that the pass did not visit.

Keep those nodes in a second tree over the same program, and never end a pass on it. With no
`finish`, nothing is dropped: a node lives until the host removes it. Call `begin` before each
bind all the same. It drops nothing; it empties what the tree recorded since the last one, so
that `inert` is what this bind found and does not grow for as long as the program stands. The template is a function value, so a
cell's content comes from `call_function`, and what it returns is visited under no node at a key
the host makes from the list and the item.

```rust,no_run
use nx_ir_runtime::{InstanceTree, Program, Result, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;

/// The layout engine bound a cell of `list` to item `index`: the template's output for it.
fn bind(
    cells: &mut InstanceTree,
    template: &NxValue,
    list: &str,
    index: usize,
    item: &NxValue,
    options: &RuntimeOptions,
) -> Result<NxValue> {
    let args = BTreeMap::from([
        ("Item".to_string(), item.clone()),
        ("Index".to_string(), NxValue::Int(index as i64)),
    ]);
    let content = cells.program().call_function(template, &args, options)?;
    // Not a pass that will end: this empties the tree's record of inert handlers.
    cells.begin();
    match &content {
        // A component: its node is the cell's, and keeps its state while the cell is out of view.
        NxValue::Record { type_name: Some(name), .. } if cells.can_instantiate(name) => {
            let rendered = cells.visit(&format!("{list}/{index}"), &content, None, options)?;
            Ok(rendered.as_ref().clone())
        }
        _ => Ok(content),
    }
}

fn run(program: Program) {
    let drawing = InstanceTree::new(program.clone());
    // The same program, and no pass that ends: `finish` is never called on this one.
    let mut cells = InstanceTree::new(program);
    // When the list leaves the drawing or its collection changes, drop what was kept for it.
    cells.remove("list/0");
    # let _ = drawing;
}
```

- **A cell bound again to the same item is left alone.** The template returns the same descriptor,
  so the visit does nothing and the node keeps its state. That is what lets a recycled cell show an
  item it showed before as the user left it.
- **An event names the cell's node.** `dispatch` against the second tree with the key the node was
  visited at. A handler the template bound outside any component has no instance to run against;
  a visit under no node leaves it out and reports it through `inert`.
- **The host decides how long a cell's state lives.** `remove` drops a node and what is under it.
  Removing when the collection changes gives a cell the life of its item; removing when the cell
  scrolls away gives it the life of what is on screen. A `remove` looks at every node of the tree,
  so where one list's cells are all a tree holds, replace the tree instead of removing them one by
  one. And nothing but `remove` drops a node here: a component nested in a cell's component stays
  after its place has left the output, where a pass would have dropped it.
- **Nothing reaches from one tree to the other.** A component in a cell cannot emit to a component
  in the drawing: an emit nobody bound in the cell's own tree comes back as a `HostEffect`.
- **The component depth starts again in every cell.** A cell's content is visited under no node,
  so `max_component_depth` counts from the cell and knows nothing of the list the cell is in. A
  template that draws a list of itself nests cells in cells without ever nesting components, and
  no limit of the tree ends it. A host whose layout puts cells inside cells carries its own depth
  from a list into the cells it binds.

Every operation that evaluates takes `RuntimeOptions` and passes them to each lifecycle call it
makes, so the operation budget and the input limit apply to each of those calls, not to the
operation as a whole. A key that names no node fails with `nx-ir-instance-key`. The tree is plain
data over shared values: cloning it copies no value, and it is `Send + Sync`.

## Limits

| Limit | Default | Set by | Name in a diagnostic |
| --- | --- | --- | --- |
| Operations one call may cost | Unlimited | `RuntimeOptions::max_operations` | `maxOperations` |
| Input one call may be given | Unlimited | `RuntimeOptions::max_input_size` | `maxInputSize` |
| Call depth | 100 | `RuntimeOptions::max_call_depth` | `maxCallDepth` |
| Component instances nested in an instance tree, counting the node visited and those above it | 100 | `RuntimeOptions::max_component_depth` | `maxComponentDepth` |
| Integers one range may hold when a loop iterates it | 1,000,000 | `RuntimeOptions::max_range_length` | `maxRangeLength` |
| Nested expressions, across every call of one evaluation | 1,000 | fixed | `maxExpressionNesting` |
| Native stack one call may use, from where it began | 1 MiB | `RuntimeOptions::max_stack_bytes` | `maxStackBytes` |
| Nesting of a value crossing the host boundary, and of component state after each patch | 256 | fixed | `maxValueNesting` |
| Nesting of the values of a serialized instance | 1,024 | fixed | |
| Values one value of a serialized instance holds, written out | 16,777,216 | fixed | |

Exceeding any of them fails with `nx-ir-resource-limit`, or with `nx-ir-component` for a
serialized instance. The nesting limits and the stack budget are what keep an image, a host value
or a raised call depth from exhausting the native stack, so the thread that calls the runtime
needs the stack budget free: 1 MiB unless the host says otherwise. Checking a value against a
declared type and converting one at the host boundary are under the stack budget as well, so an
unoptimized build, whose frames are larger, refuses a deeply nested value sooner than 256 levels,
and can meet the stack budget before a thousand expressions nest.

### Small stacks

The runtime recurses on the native stack and cannot ask how much of it a thread has, so the budget
is the host's statement of what is free where it calls. The default suits a native thread. It does
not suit a WebAssembly module, whose whole stack is often 1 MiB or less and partly used by the time
the host calls the runtime: under the default budget the runtime believes it has more than there
is, and an evaluation that recurses far enough overruns the stack, which ends the module without a
diagnostic. A host on such a stack does one of two things:

- **States what is free.** Set `max_stack_bytes` to the stack that is free where the call is made,
  less a margin. An evaluation that needs more then fails with `nx-ir-resource-limit` naming
  `maxStackBytes` and the budget that was in force. A budget may be lower than a legitimate
  program needs, in which case that program gets the diagnostic too.
- **Links a larger stack.** With the deepest evaluation's need free below the deepest point the
  host calls from, the nesting limit is met before the stack is.

The budget covers one call, from where that call begins. It does not cover the host. A host that
walks rendered output by recursion and visits components as it goes (the instance tree's `walk`
above) uses stack between its calls that no call counts, and every `visit` is a call of its own
with its own call depth, so no single call of a component that renders itself ever goes deep.
What ends that tree is `max_component_depth`: a `visit` for a node more than that many component
instances deep fails with `nx-ir-resource-limit` naming `maxComponentDepth`. What a level costs in
stack is the walk's to say, so lower the limit where the walk's frames are large, and raise it for
deep data on a stack that has the room. The limit counts component instances only.
Records the host interprets itself can nest as deeply as a program writes them with no component
between, so a host that recurses over those bounds that depth itself, and states the stack budget
where each call is made, not once where the walk began.

How much the deepest evaluation needs was measured on `wasm32-unknown-emscripten`, in a release
build linked with a 1 MiB stack and run in Chromium, with a function that recurses until the
nesting limit stops it:

| Stack free where the call began | Default budget | Budget = free stack less a margin |
| --- | --- | --- |
| 162 KiB and more | the nesting diagnostic | the nesting diagnostic |
| 153 KiB | the page ended | a `maxStackBytes` diagnostic |
| 100 KiB, then 24, 15, 11, 7 and 2 KiB | the page ended at 100 KiB; not tried below | a `maxStackBytes` diagnostic |

So that evaluation needs a little under 160 KiB in an optimized WebAssembly build, and with the
budget stated every call came back as a diagnostic, for margins of 8, 16, 32 and 64 KiB. An unoptimized
build's frames are several times larger: a native debug build meets the default budget of 1 MiB
before a thousand expressions nest. The numbers are for that function and that build, so read them
as the size of the need, not as a constant. They were taken with a probe that used up a chosen
amount of stack in frames of its own and called `evaluate_function` from there, on a fresh page
for each depth, since an overrun ends the module.

The margin is for what the runtime uses between two checks of its budget, which it makes at every
expression and every value it walks, and to report a failure, which takes the same stack however
deep the value it is in. A budget equal to the free
stack is too large by that much: the check passes on the last frame that fits and the next one
does not.

On `wasm32-unknown-emscripten` the free stack is one call away:

```rust,ignore
use nx_ir_runtime::RuntimeOptions;

/// The limits of one call into the runtime, made from here.
fn options() -> RuntimeOptions {
    unsafe extern "C" {
        fn emscripten_stack_get_free() -> usize;
    }
    // What the runtime uses between two checks of its budget, and to report that it met it.
    const MARGIN: usize = 32 << 10;
    RuntimeOptions {
        max_stack_bytes: unsafe { emscripten_stack_get_free() }.saturating_sub(MARGIN),
        ..RuntimeOptions::default()
    }
}
```

Build the options where the call is made, not once at startup: what is free depends on how deep
the host is when it calls. The budget covers every walk a call that takes options makes:
evaluation, the check of a value against a type, the conversion at the host boundary, a
comparison of two values, and the check of how deeply state nests. The calls that take no options
(`apply`, `merge`, `diff`, `Program::changed`, `restore_component_instance`, and
`ComponentInstance::same_handler` and `bound_handler_is`) walk under the default budget, so a host
that cannot give them a mebibyte keeps the values it hands them shallow. The crate itself has no `unsafe` and nothing specific to a platform, so
it builds for `wasm32-unknown-emscripten` and `wasm32-wasip1` as it is, and CI checks both.

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
in proportion, and the time and the memory of a call are proportional to its count. It is a claim
about every step of the runtime, and review has several times found a step that broke it, each
time through a large or unusual value a host passed at `object`. Four things back it now: the
input limit, which bounds what a step nobody has found can cost; a differential test that runs
generated host values through this runtime and the TypeScript one and requires the same count,
input size and failures; bounds on this runtime's allocations for every generated case; and a
time report, which does not block a merge, of cases whose time grows faster than their count. What
is still open is the list of known findings in `crates/nx-codegen/tests/cost/mod.rs`, and steps
driven by a value a program builds, which only fixed probes exercise; `docs/nx-ir-format.md` has
the details under *Validation against generated host values*. So treat the budget as the first
limit on code you did not write, set `max_input_size` beside it to bound what you hand that code
(see *The input limit* below), and keep a limit of your own on time and memory as well. The same
holds for a
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

### The input limit

`RuntimeOptions::max_input_size` bounds what the host hands one call. The *input size* is defined
in `docs/nx-ir-format.md`, under *Input size*, and is measured the way a value written for the
host is charged: one for each value, a list, an empty one and a `null` included, and one more for
every 64 UTF-16 code units of a string, of a type name and of each field name. It covers every host
value passed to a method that takes options: the arguments of `evaluate_function`; the function
record and the arguments of `call_function`; the props and the content of
`construct_component_descriptor`; the props of `initialize_component` and the state of its
`ComponentInit`; the props and the state of `evaluate_component`; the batch of
`dispatch_component_actions`; the state of `normalize_component_state`; and the state and the patch
of `apply_component_state_patch`. A map of props, of state or of arguments by name is measured as
one record. An instance is not input, and `restore_component_instance`, which takes no options,
keeps the bounds it has.

A call whose input is larger fails with `nx-ir-resource-limit` naming `maxInputSize`, before the
program is looked at and before anything is converted, so it is reported ahead of any other fault
of the call, a batch is measured whole before its first entry runs, and input that is refused is
not copied. The diagnostic names no declaration. The measuring pass builds nothing, does not
recurse and stops as soon as the size passes the limit; text far longer than the limit allows is
refused from its byte length without being read. The input limit is applied before the nesting
bound: input over the limit is reported for its size, and input within it that nests more than 256
deep is still refused for its nesting. Unset, nothing is measured, and the limit charges nothing
to the budget. The TypeScript runtime measures the same size, so the same input is refused under
the same limit in both.

Choose it together with the budget. Every step the runtime takes is meant to be charged in
proportion to the values it touches, and the limit is there for one that is not and that nobody
has found: with a budget `B` and a limit `C`, a step that does work in proportion to a host value
for a fixed charge costs at most about `B × C` in one call. At a budget of 100,000 and a limit of
1,000 that is about 10^8 values or 64-code-unit pieces of text, where without a limit it had no
ceiling. What the limit does not bound is a step driven by a value the program builds itself,
which is bounded by the budget squared, and the state an instance holds, which is bounded only
when every call that wrote it had a budget. So keep the limit on time and memory as well.

`input_size` measures one value as the limit does, without a call, and `record_input_size` a map
of named values as the one record a call measures it as, so a host can hold one part of what it
passes to a number of its own. With a limit each stops as soon as the size passes it and returns
some number greater than the limit. A list measured as one value is one more than its entries add
to a call that takes them as arguments, content or a batch.

### What a call used

Share a `Usage` with a call through `RuntimeOptions::usage` and the runtime reports what the call
used: `operations()` when a budget is set, and `input_size()` when an input limit is set and the
input was within it. The report is cleared when the call begins and filled when it ends, whether
it succeeds or fails. For a call that succeeds the operations are its count, the least budget it
succeeds under; for one that fails they are what was charged before the failure, never more than
the budget.

```rust
use nx_ir_runtime::{Program, Result, RuntimeOptions, Usage};
use nx_value::NxValue;
use std::sync::Arc;

fn run_tool(program: &Program, input: NxValue) -> Result<NxValue> {
    let usage = Arc::new(Usage::new());
    let options = RuntimeOptions {
        max_operations: Some(100_000),
        max_input_size: Some(1_000),
        usage: Some(Arc::clone(&usage)),
        ..RuntimeOptions::default()
    };
    let result = program.evaluate_function("tool", &[input], &options);
    println!(
        "tool used {:?} operations on an input of {:?}",
        usage.operations(),
        usage.input_size()
    );
    result
}
```

Nothing is counted for the report alone: with no budget `operations()` is `None`, so a host that
wants the count and no limit sets a budget it cannot reach. That is how to choose a budget from
measurement, by running the programs you mean to allow and reading what they cost:

```rust
use nx_ir_runtime::{Program, Result, RuntimeOptions, Usage};
use nx_value::NxValue;
use std::sync::Arc;

fn budget_for(program: &Program, representative_inputs: &[NxValue]) -> Result<u64> {
    let usage = Arc::new(Usage::new());
    let options = RuntimeOptions {
        max_operations: Some(u64::MAX),
        usage: Some(Arc::clone(&usage)),
        ..RuntimeOptions::default()
    };
    let mut most = 0;
    for input in representative_inputs {
        program.evaluate_function("tool", std::slice::from_ref(input), &options)?;
        most = most.max(usage.operations().unwrap_or(0));
    }
    // Headroom over the largest count measured.
    Ok(most.saturating_mul(4))
}
```

`RuntimeOptions` is `Clone` and no longer `Copy`, since it can hold the shared report. Calls that
run at the same time and share one `Usage` overwrite each other; give each its own.

## Diagnostics

Every API returns `Result<_, NxIrRuntimeError>`. The error holds one or more diagnostics, each
with a `code`, a `message`, the declaration the failing expression belongs to as `identity::name`
when evaluation had reached one, and the expression's span when the image carries its debug
section. An `nx-ir-resource-limit` diagnostic also carries `limit`, a `Limit` holding the name the
table under *Limits* gives and the limit's value, so a host tells an exhausted budget from runaway
recursion without reading the message; no other diagnostic carries one. The runtime never reads a
source file and never panics on an image, a host value or an instance.

A diagnostic for a failure in an argument of the function a host called, through `call_function`
or `evaluate_function`, carries `argument`, the declared name of the parameter the value was
passed for, whether the arguments were passed by name or by position:

```rust
use nx_ir_runtime::{Program, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;

/// The parameter a refused call's argument was passed for, if the failure is in an argument.
fn refused_argument(
    program: &Program,
    function: &NxValue,
    args: &BTreeMap<String, NxValue>,
) -> Option<String> {
    // `{ "teamSize": "five" }` for `findPlans(teamSize:int)` fails with `nx-ir-boundary-type`,
    // and its diagnostic's `argument` is `teamSize`.
    let error = program
        .call_function(function, args, &RuntimeOptions::default())
        .err()?;
    error.diagnostics.first()?.argument.clone()
}
```

It is `Some` for a value that does not fit its parameter's type, at any depth inside the value,
for a `Function` record in the value that names no function, and for a required parameter given
nothing. It says the failure is in what the host passed, so it is `None` for everything else,
even when the runtime finds the failure while it checks an argument: a failure a default raises
(a record's defaults are filled in while the argument that holds the record is checked), a
resource limit whenever it is reached, a failure in the body or the result, more positional
arguments than the function has parameters, the `Function` record `call_function` is given to say
which function to call, and a failure of any other entry point. The TypeScript runtime names the
same argument for the same call, and the conformance corpus holds the two to it.

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
| `nx-ir-instance-key` | An instance tree was asked for a node at a key that names none, or to visit a node under itself. The TypeScript runtime has no instance tree and does not report it. |
| `nx-ir-division-by-zero`, `nx-ir-number`, `nx-ir-operator`, `nx-ir-type` | An operator met an operand it cannot take. |
| `nx-ir-member`, `nx-ir-slot`, `nx-ir-for`, `nx-ir-record`, `nx-ir-union`, `nx-ir-intrinsic` | An expression could not be evaluated as written. |
| `nx-ir-resource-limit` | A limit above was exceeded. |

## Tests

`cargo test -p nx-ir-runtime` runs the conformance corpus in `specs/ir-conformance` (every
entrypoint and lifecycle, with and without debug sections), the damage runs (every truncation of
every corpus image, and every cell of one image overwritten), the preparation and linking
tests, and `tests/allocation.rs`, which counts the bytes a refused concatenation allocates to show
that the budget is charged before a string is built, and the bytes refused input allocates to show
that it is measured before it is copied. The tests that compile NX source, including
the differential run against the interpreter, need the compiler and live in `nx-codegen`
(`cargo test -p nx-codegen ir_runtime`), as do the instance tree's, one for each scenario of its
specification (`cargo test -p nx-codegen ir_instance_tree`). So do the cost tests, which run generated host values
through this runtime and the TypeScript one and hold this runtime's allocations to its counts
(`cargo test -p nx-codegen --test cost_differential` and `--test cost_allocation`).
