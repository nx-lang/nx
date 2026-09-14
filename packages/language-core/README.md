# @nx-lang/language-core

The half of NX language serving that has nothing to do with a transport: the prelude coordinate
arithmetic, the content-keyed snapshot cache, the query dispatcher, and an in-process implementation
of [`@nx-lang/language-protocol`](../language-protocol)'s `NxLanguageService`.

Two things sit on top of it, and neither can disagree with the other about a line number, because
there is only one copy of the shift:

- [`@nx-lang/language-http`](../language-http) adds request parsing, body limits, error responses and
  a Node listener, over snapshots from [`@nx-lang/sdk-node`](../../bindings/node).
- [`@nx-lang/sdk-wasm`](../../bindings/wasm) binds it to snapshots from a WebAssembly host, so a
  browser answers hover exactly as a server would.

The package imports nothing from `node:`. It runs in a browser, a Web Worker and Node alike.

## An in-process service

```ts
import { createSnapshotLanguageService } from "@nx-lang/language-core";

const service = createSnapshotLanguageService({
  createSnapshot: (documents) => host.createLanguageSnapshot(documents),
  prelude: { source: catalogSource },
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

## Preludes

A host that prepends context declarations to the queried document passes them as `prelude`. Every
answer comes back in the document's own coordinates:

- a queried position is shifted into the combined text before the snapshot sees it;
- a hover range, a document symbol's ranges and a diagnostic's range are shifted back out;
- a diagnostic whose range lies inside the prelude is the host's fault, not the author's, and is
  reported as a workspace diagnostic labelled `prelude` with no range, rather than positioned on a
  line the author cannot see;
- a related location pointing into the queried document is shifted wherever it appears.

`preludeOffsets` normalizes the prelude to end with a newline and adds a blank line after it, so the
document's first line is always its own whatever the prelude ended with. Offsets are counted in
lines and in UTF-8 bytes.

## Building and testing

```bash
pnpm --filter @nx-lang/language-core build
pnpm --filter @nx-lang/language-core test
```
