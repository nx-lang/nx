## Context

See proposal.md for why. What shapes the approach:

- `evaluateNx()` evaluates with the interpreter (`nx-interpreter`) and writes the value with
  `crates/nx-api/src/nx_text.rs`, which records for each node its role, type and the span of its
  declaration in the entry module. `Value::Record` holds a type name and fields, nothing about
  where it was built.
- The previewer runs components with the TypeScript IR runtime (`initializeComponent`,
  `dispatchComponentActions`), whose results are canonical values with no source information.
- An IR image built with `debug: true` carries a debug section: a `(start, end)` byte span for
  every declaration and every node, and the source text (`nx-ir-format`). The runtime already reads
  it to put spans on diagnostics.
- The source tree (`add-source-tree`) gives every node UTF-8 byte offsets alongside its editor
  range, so byte spans are the common currency.

## Goals / Non-Goals

**Goals:**

- From any record in a preview, find the element in the source that built it.
- From an element in the source, find every record it built in the current output.
- No cost to hosts that do not ask.

**Non-Goals:**

- Origins for scalars and sequences. Selection works on elements; a scalar's place is its
  property's, which its record gives.
- The chain of call sites a record passed through. A reused component would point every use back
  at its one body; the chain can be added later as a second field.
- The Rust IR runtime. It follows when a Rust host needs it, with the same shape.

## Decisions

### Origin is where the record was constructed, and it travels with the record

The alternative, the place where the record entered the output, would make a question shown
through `<Step question={roleQuestion} />` point at the `Step`, which is not what a reviewer
clicked. The construction site is what the source tree shows as the question's card.

### Byte spans, matched by module and offsets

The interpreter and the IR both keep byte ranges, and the source tree carries byte offsets on every
node, so no line and column conversion is needed and no tool has to guess between UTF-16 and bytes.

### Annotated text always carries origins; the runtime only on request

`evaluateNx()` exists to show a value to a person, so origins belong there by default, and the
text is unchanged. The runtime's results are a host's working data, often serialized; the option
keeps them byte-for-byte as they are unless a host asks.

### The runtime reports origins as a side list keyed by JSON pointer

Rendered output is a canonical value that hosts serialize and compare; adding a field to each
record would change it. A list of `{ path, module, start, end }` beside it changes nothing, and a
host indexes it once. Internally the runtime keeps the constructing node with each record object it
builds (a `WeakMap` from record to node), and collects the list when it returns.

## Risks / Trade-offs

- [Interpreter record values grow by a range] → One `TextRange` and a module index per record;
  measured on the parity corpus by the tasks.
- [Records rebuilt by update application lose their original origin] → By decision they take the
  origin of the expression that applied the update, which is the code that produced the new value.

## Open Questions

- Whether the value view's navigation event should prefer `origin` over `declaration` by default,
  or offer both and let the host choose. The tasks pass both through and leave the playground's
  behaviour as it is.
