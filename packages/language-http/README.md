# @nx-lang/language-http

A stateless, mountable Node handler that answers [`@nx-lang/language-protocol`](../language-protocol)
queries — hover, completions, diagnostics, document symbols — through
[`@nx-lang/sdk-node`](../../bindings/node). Any Node host offers NX language features by mounting
one function under a path of its choosing; no sessions, no document lifecycle, no language-server
process.

```ts
import { createNxLanguageHandler } from "@nx-lang/language-http";

const handler = createNxLanguageHandler();
// (request: Request) => Promise<Response>
```

The handler is Fetch-shaped: it takes a standard `Request` and returns a standard `Response`. It
routes on the **final path segment** (`.../hover`, `.../completions`, `.../diagnostics`,
`.../documentSymbols`), so the host chooses the base path. It accepts only `POST` with a JSON body,
and every answer — success or failure — is JSON.

## Mounting

### Plain `node:http`

`toNodeListener` presents the handler with Node's listener signature. This is the mount the DrawnUI
fiddle server uses:

```js
import { createServer } from "node:http";
import { createNxLanguageHandler, toNodeListener } from "@nx-lang/language-http";

const language = toNodeListener(createNxLanguageHandler());

createServer((request, response) => {
  if (request.url?.startsWith("/api/language/")) {
    language(request, response);
    return;
  }
  // ...the host's other routes
}).listen(3000);
```

### Hono

Hono hands out the raw `Request`, so the handler mounts directly:

```ts
import { Hono } from "hono";
import { createNxLanguageHandler } from "@nx-lang/language-http";

const app = new Hono();
const handler = createNxLanguageHandler({ buildContext });

app.all("/api/language/*", (c) => handler(c.req.raw));
```

Anything that authenticates or rate-limits the route goes in front of it as it would for any other
route; the handler itself trusts every request it receives.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `buildContext` | none | An `NxProgramBuildContext` from `@nx-lang/sdk-node`. Every query sees the libraries it makes visible, with the visibility rules the compiler applies. Without one, names declared only in a library are unresolved — hover says nothing about them and diagnostics report the import as missing, exactly as the compiler would. |
| `prelude` | none | `{ source }`: NX text placed ahead of the queried document before analysis. See below. |
| `maxBodyBytes` | 1 MiB | Requests with a larger body are refused with `413`. |
| `cacheSize` | 8 | How many analyzed document sets are kept. |
| `onError` | none | Called with the error whenever the handler answers `500`, so the host can log it. |
| `createSnapshot` | `new NxLanguageSnapshot(...)` | The snapshot factory. Tests inject a counting or failing one. |

### Build context

```ts
import { NxLibraryRegistry } from "@nx-lang/sdk-node";

const registry = new NxLibraryRegistry();
registry.loadFromDirectory("/srv/nx/builtins");
const buildContext = registry.createBuildContext();

const handler = createNxLanguageHandler({ buildContext });
```

A host that already validates configurations against a registry-backed context passes the same
context here, and hover, completions and diagnostics know the same modules validation does.

### Prelude

Some hosts have context declarations that the compiler cannot yet import as a separate module
without loss (NXE12/NXE13: an imported external component loses its defaults and inherited
properties). Such a host concatenates its declarations ahead of the author's text. The `prelude`
option does that inside the handler, once, with the arithmetic tested:

- The prelude is applied to the document named by the query's `uri`; other documents in the set are
  untouched.
- The prelude always ends in a newline and is followed by one blank line, so the document's first
  line is its own line whatever the prelude ended with.
- Every incoming position is shifted into the combined text and every outgoing range shifted back.
  Only lines and bytes move; a column is relative to its line start and the prelude contributes whole
  lines. A related location that points into the queried document is shifted wherever it appears,
  including in a sibling document's diagnostics.
- A diagnostic whose range lies inside the prelude is the host's fault, not the author's. It is
  reported in the answer's `workspace` list, labelled with identity `prelude` and no range, rather
  than positioned on a line the author cannot see.
- Document symbols declared by the prelude are omitted from the document's symbols.

The helpers the handler uses — `preludeOffsets(source)`, `withPrelude`, `shiftPositionIn`,
`shiftRangeOut` — are exported so a host that also compiles the same combined text can share one
implementation of the shift.

## Behavior

**Stateless.** Every answer is computed from the request body alone. Nothing is kept between
requests except the cache below, which is a pure function of request content. Several instances
behind a load balancer need no session affinity; two fresh handlers return the same answer to the
same query.

**Cached by content.** The analysis of a document set is kept under a hash of every document's URI,
identity and full text, most-recently-used first, bounded by `cacheSize`. A burst of queries over
unchanged text — hover on several positions, then a completion — builds the analysis once; a
one-character change builds a new one and can never be served the old. A document's `version` is
not part of the key: the same text under a new version number reuses the analysis, and each answer
carries the versions of the request it answers. Evicted snapshots are disposed.

**Single-threaded.** Analysis runs synchronously in the native binding on the event loop, typically
tens of milliseconds for a few hundred lines. A host that serves many concurrent editors should run
the handler in a worker or a separate process, as it would any CPU-bound route.

**Bounded and contained.** A body over `maxBodyBytes` answers `413`; a body that is not JSON, or a
JSON object missing `documents`, `uri`, `source`, or (for `hover` and `completions`) `position`
answers `400` with a message naming what is missing; a query name that is not defined answers `404`,
and a reserved one (`definition`, `references`, `rename`, `signatureHelp`, `inlayHints`,
`semanticTokens`) answers `501` with the protocol's `unsupported-query` code; a method other than
`POST` answers `405`. A throw inside the language service answers `500` with a JSON body, is passed
to `onError`, and leaves the process serving the next request.

Error bodies have the protocol's shape:

```json
{ "error": { "code": "invalid-request", "message": "Request is missing 'uri', the document the query is about.", "query": "hover" } }
```
