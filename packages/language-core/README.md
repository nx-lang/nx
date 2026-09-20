# @nx-lang/language-core

The half of NX language serving that has nothing to do with a transport: the host context, the
content-keyed snapshot cache, the query dispatcher, and an in-process implementation of
[`@nx-lang/language-protocol`](../language-protocol)'s `NxLanguageService`.

Two things sit on top of it, and neither can disagree with the other about a line number, because
there is only one copy of the dispatcher and the queried text is analyzed exactly as it was sent:

- [`@nx-lang/language-http`](../language-http) adds request parsing, body limits, error responses and
  a Node listener, over snapshots from [`@nx-lang/sdk-node`](../../bindings/node).
- [`@nx-lang/sdk-wasm`](../../bindings/wasm) binds it to snapshots from a WebAssembly host, so a
  browser answers hover exactly as a server would.

The package imports nothing from `node:`. It runs in a browser, a Web Worker and Node alike.

## An in-process service

```ts
import { createSnapshotLanguageService } from "@nx-lang/language-core";

const service = createSnapshotLanguageService({
  createSnapshot: (documents, { implicitImports }) =>
    host.createLanguageSnapshot(documents, { implicitImports }),
  context: {
    documents: [{ uri: "nx://host/catalog.nx", identity: "catalog.nx", source: catalogSource }],
    implicitImports: ["catalog.nx"]
  },
  cacheSize: 8
});

const hover = await service.hover({ documents, uri, position }, signal);
```

The result implements `NxLanguageService`, so `@nx-lang/monaco` and any other client of that
interface takes it without knowing where the analysis happens. `dispose()` releases every cached
analysis; the service answers again, analyzing afresh, afterwards.

Answers are produced synchronously on the calling thread, so a cancellation can only be observed
before the work starts: an already-aborted `signal` rejects with an `AbortError` and asks nothing of
the snapshot. Whoever runs the service off the caller's thread — the playground runs it in a
worker — is what makes a cancel preemptive.

## Snapshots and the cache

A `SnapshotLike` is the small surface the dispatcher needs: `hover`, `completions`, `diagnostics`,
`documentSymbols`, `dispose`. Both SDKs' snapshots satisfy it, and a test can hand over a fake.

`documentSetKey` keys an analysis by the document set's own content — URI, identity and source, each
preceded by its length so no two sets can join into the same string. A document's `version` is left
out on purpose: it is the editor's counter, not content, and the same text under a new number has
the same analysis. The dispatcher puts the current request's versions back onto the answer, so a
client that discards answers by version still sees its own.

`SnapshotCache` holds the most recently used analyses and disposes what it evicts. A hover storm
sends the same text many times a second; keeping the last few sets is what makes answering cheap on
one thread.

## Host context

A host with declarations of its own — a catalog of external components, say — passes them as
`context`: `documents` that join every query's document set without the client sending them, and
`implicitImports`, the identities every queried document imports as if it began with a wildcard
import of each.

Nothing is prepended and no position is ever rewritten:

- a queried position is what the client sent, and the snapshot is asked exactly that;
- a hover range, a document symbol's ranges and a diagnostic's range come back in the queried
  document's own lines and columns;
- a diagnostic located in a context document is reported under **that document's** URI, with its own
  coordinates, so a fault in the host's declarations is never shown on a line the author wrote;
- a related location needs no adjustment, wherever it points.

A query whose documents include one with a context document's URI or identity is refused: the client
cannot replace the host's context. `contextCollision(documents, context)` is the check, and the HTTP
handler answers `400 invalid-request` naming the identity.

### Migrating from `prelude`

The `prelude` option prepended the host's text to the queried document and shifted every position.
It is removed, and a call that still passes it fails at construction with a message naming `context`.
The replacement is two lines:

```ts
// Before
createSnapshotLanguageService({ createSnapshot, prelude: { source: catalogSource } });

// After
createSnapshotLanguageService({
  createSnapshot,
  context: {
    documents: [{ uri: "nx://host/catalog.nx", identity: "catalog.nx", source: catalogSource }],
    implicitImports: ["catalog.nx"]
  }
});
```

`preludeOffsets`, `withPrelude`, `shiftPositionIn`, `shiftRangeOut` and `PRELUDE_ORIGIN` are removed
with it; nothing shifts any more, so there is nothing for them to do.

## Building and testing

```bash
pnpm --filter @nx-lang/language-core build
pnpm --filter @nx-lang/language-core test
```
