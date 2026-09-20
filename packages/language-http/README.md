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
| `context` | none | `{ documents, implicitImports }`: the host's own declarations, served with every query. See below. |
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

### Host context

A host with declarations of its own — a catalog of external components, say — serves them with every
query through `context`:

```ts
const handler = createNxLanguageHandler({
  context: {
    documents: [{ uri: "nx://host/catalog.nx", identity: "catalog.nx", source: catalogSource }],
    implicitImports: ["catalog.nx"]
  }
});
```

- The context documents join every query's document set without the client sending them, and the
  identities in `implicitImports` are in scope in every queried document as if it began with a
  wildcard import of each.
- The queried document's text is analyzed **exactly as the client sent it**. Every position in a
  query and every range in an answer is in that document's own lines, columns and byte offsets, with
  nothing prepended and nothing shifted.
- A diagnostic located in a context document is reported under **that document's** URI, in its own
  coordinates, so a fault in the host's declarations is never shown on a line the author wrote.
- Document symbols are the queried document's own, since only its text is queried.
- A request whose `documents` include one with a context document's URI or identity is refused with
  `400 invalid-request` naming the identity: the client cannot replace the host's context.

#### Migrating from `prelude`

The `prelude` option prepended the host's text to the queried document and shifted every position. It
is removed, and a handler constructed with it fails immediately with a message naming `context`:

```ts
// Before
createNxLanguageHandler({ prelude: { source: catalogSource } });

// After
createNxLanguageHandler({
  context: {
    documents: [{ uri: "nx://host/catalog.nx", identity: "catalog.nx", source: catalogSource }],
    implicitImports: ["catalog.nx"]
  }
});
```

The shift helpers (`preludeOffsets`, `withPrelude`, `shiftPositionIn`, `shiftRangeOut`) and
`PRELUDE_ORIGIN` are removed with it. Nothing shifts any more, so a host that compiles the same
documents needs no shared arithmetic: it compiles the same set it queries.

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
