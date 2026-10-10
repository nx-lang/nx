## Why

The planned NX previewer shows a program's output beside the NX viewer, and the two share one
selection: clicking a question in the preview selects the NX that produced it, and selecting an
element in the viewer highlights what it produced. Neither direction is possible today, because an
evaluated value does not say where it came from. The annotated text `evaluateNx()` returns says,
for each node, where its *type* is declared, which is the same for every `SingleChoice` in a survey.
The TypeScript runtime's rendered output carries no source information at all, although an image
built with its debug section holds a source span for every IR node.

## What Changes

- Each record node of the annotated text `evaluateNx()` returns gains an `origin`: the module
  identity and source span of the element expression that constructed the record. A record passed
  through props, stored in state or returned from a function keeps the origin of its construction.
- The TypeScript IR runtime gains an option, `origins`, under which its evaluation and component
  lifecycle results carry a list of origins for the records in the value they return, each naming
  the record by its JSON pointer in that value and giving the module identity and the byte span of
  the IR node that constructed it. An image without its debug section yields no origins and is not
  an error. Without the option the results are unchanged.
- Origins are byte spans, so a tool matches them to the source tree's nodes by module and byte
  offsets.

The chain of call sites a value passed through, and origins in the Rust IR runtime, are not part of
this change.

## Capabilities

### New Capabilities

- `value-origin`: what an origin is, which values carry one, how it survives being passed around,
  and how a tool matches it to source.

### Modified Capabilities

- `sdk-wasm`: annotated record nodes carry their origin.
- `typescript-ir-runtime`: the `origins` option and the origin list on results.

## Impact

- `crates/nx-interpreter`: a record value records the range of the element expression that
  constructed it; `crates/nx-api/src/nx_text.rs` writes it on record nodes. The printed text is
  unchanged.
- `bindings/wasm`: `NxValueNode.origin` in the TypeScript types.
- `runtime/typescript`: the option, the result field, and the bookkeeping that keeps a record's
  constructing node with the record.
- `packages/value-view`: the element passes `origin` through on its navigation event, so a host
  can select the construction rather than the type declaration.
- No change to the IR format, the wire format or evaluation results.
