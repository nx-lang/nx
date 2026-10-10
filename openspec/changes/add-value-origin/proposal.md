## Why

The planned NX previewer shows a program's output beside the NX viewer, and the two share one
selection: clicking a question in the preview selects the NX that produced it, and selecting an
element in the viewer highlights what it produced. Neither direction is possible today, because an
evaluated value does not say where it came from. The IR runtimes' results carry no source
information, although an image built with its debug section holds a source span for every IR node,
which the runtimes already use to put spans on diagnostics.

The interpreter is being retired in favour of the IR runtimes, so origins are built in the
runtimes only, the same way in Rust and TypeScript.

## What Changes

- Both IR runtimes gain an origins report a host can pass in its options, beside the existing
  usage report. When a call ends, the report lists the origin of each record in the value the call
  returned: the record's JSON pointer within that value, and the module identity and byte span of
  the IR node that constructed it, read from the image's debug section.
- A record keeps the origin of its construction when it is bound, passed as a prop or argument,
  stored in state or returned. Records from an image without its debug section have no origin, and
  that is not an error.
- Without the report nothing changes: no result type, signature or value differs, and nothing is
  collected.
- The two runtimes report the same origins for the same call, checked over the conformance corpus.
- Origins are byte spans, so a tool matches them to the source tree's nodes by module and byte
  offsets.

The chain of call sites a value passed through is not part of this change. Neither is `origin` on
the annotated text `evaluateNx()` returns: that follows when `evaluateNx()` moves from the
interpreter onto an IR runtime, as a mapping from this report.

## Capabilities

### New Capabilities

- `value-origin`: what an origin is, which values carry one, how it survives being passed around,
  how a runtime reports it, and how a tool matches it to source.

### Modified Capabilities

- `typescript-ir-runtime`: the `origins` option and its report object.
- `rust-ir-runtime`: `RuntimeOptions::origins` and its report type.

## Impact

- `runtime/typescript`: the option, the report type, and keeping each record's constructing node
  with the record.
- `crates/nx-ir-runtime`: the same, with the constructing node held on the internal `Record`.
- No change to the IR format, the wire format, the compiler or the interpreter.
