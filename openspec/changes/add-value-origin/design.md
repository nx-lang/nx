## Context

See proposal.md for why. What shapes the approach:

- The interpreter is being retired; the previewer and hosts run programs on the IR runtimes, in
  TypeScript (`runtime/typescript`) and Rust (`crates/nx-ir-runtime`), which give the same results
  and are checked against one conformance corpus.
- An IR image built with `debug: true` carries a debug section: a `(start, end)` byte span for
  every declaration and every node, and the source text (`nx-ir-format`). Both runtimes already
  read it to put a `SourceSpan` on diagnostics.
- Both runtimes already have a report a host passes in its options and reads when the call returns:
  `usage`, cleared at the start of each call and filled at its end.
- The source tree (`add-source-tree`) gives every node UTF-8 byte offsets alongside its editor
  range, so byte spans are the common currency.

## Goals / Non-Goals

**Goals:**

- From any record in a preview, find the element in the source that built it.
- From an element in the source, find every record it built in the current output.
- The same answer from either runtime.
- No cost to hosts that do not ask.

**Non-Goals:**

- Origins for scalars and sequences. Selection works on elements; a scalar's place is its
  property's, which its record gives.
- The chain of call sites a record passed through. A reused component would point every use back
  at its one body; the chain can be added later as a second field of an entry.
- `origin` on `evaluateNx()`'s annotated text. It runs on the interpreter today; when it moves onto
  an IR runtime, its record nodes take their origins from this report.

## Decisions

### Origin is where the record was constructed, and it travels with the record

The alternative, the place where the record entered the output, would make a question shown
through `<Step question={roleQuestion} />` point at the `Step`, which is not what a reviewer
clicked. The construction site is what the source tree shows as the question's card.

### Only the IR runtimes, not the interpreter

The interpreter is being retired, so work there would be thrown away, and building origins in the
runtimes covers the previewer, every SDK that runs a program, and `evaluateNx()` once it moves.

### Reported like usage, through an object in the options

Rendered output is a canonical value that hosts serialize and compare, and the Rust
`evaluate_function` returns a bare `NxValue`; putting origins in results would change both. A
report object in the options is the pattern both runtimes already have for `usage`: no signature or
result changes, a host that does not pass one pays nothing, and the clear-then-fill rule is already
specified.

### Entries keyed by JSON pointer, in walk order

The pointer names a record in the value exactly as the host received it, so a host indexes the list
once. Walk order makes the two runtimes' lists comparable with plain equality.

### The constructing node travels on the record

TypeScript keeps the node in a `WeakMap` from each record object it builds, so record objects are
unchanged. Rust adds an optional module and node index to its internal `Record`, set only when the
call has a report. Both collect the list in the same walk that converts the result for the host.

### Byte spans, matched by module and offsets

The IR keeps byte ranges and the source tree carries byte offsets on every node, so no line and
column conversion is needed and no tool has to guess between UTF-16 and bytes.

## Risks / Trade-offs

- [Records rebuilt by update application lose their original origin] → By decision they take the
  origin of the expression that applied the update, which is the code that produced the new value.
- [Recording costs work on every record construction] → Only when a report was given; measured on
  the question-flow lifecycle with the performance harness.
