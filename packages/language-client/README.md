# @nx-lang/language-client

The browser side of the [NX language protocol](../language-protocol): an HTTP implementation of
`NxLanguageService` that `POST`s each query as JSON to a base URL where
[`@nx-lang/language-http`](../language-http) is mounted. It owns per-request headers, a timeout,
and cancellation, and nothing else; an editor integration such as `@nx-lang/monaco` takes the
returned service and never sees a URL.

```ts
import { createHttpLanguageService } from "@nx-lang/language-client";

const service = createHttpLanguageService({ baseUrl: "/api/language" });

const hover = await service.hover({
  documents: [{ uri: "nx://tenant/form.nx", source, version }],
  uri: "nx://tenant/form.nx",
  position: { line: 2, character: 3 },
});
```

## Authentication

A host that protects the route supplies headers, statically or per request. The hook may be async,
so a token can be refreshed before it is sent:

```ts
const service = createHttpLanguageService({
  baseUrl: "https://api.example.com/nx/language",
  headers: async () => ({ authorization: `Bearer ${await auth.getAccessToken()}` }),
});
```

Every query request carries what the hook returns. `content-type` is always `application/json`.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `baseUrl` | required | Where the handler is mounted. A trailing slash is ignored. |
| `fetch` | `globalThis.fetch` | The `fetch` to use, for tests or a wrapped client. |
| `headers` | none | `HeadersInit`, or `(query) => HeadersInit \| Promise<HeadersInit>` called per request. |
| `timeoutMs` | 10 000 | How long to wait before aborting the request and rejecting. |

## Errors

| Rejection | When |
| --- | --- |
| `LanguageServiceHttpError` | The server answered with a non-success status. Carries `status`, `url`, `bodyExcerpt` (the first 200 characters) and, when the body was a protocol error body, its `code`. The promise never resolves with a partial or empty answer. |
| `UnsupportedQueryError` | The server answered a reserved query with the protocol's `unsupported-query` code. Distinguishable from a fault. |
| `LanguageServiceTimeoutError` | No answer within `timeoutMs`. The underlying request was aborted. Carries `timeoutMs`. |
| an abort error | The caller's `AbortSignal` was cancelled first. `isAbortError(error)` recognizes it. The promise never resolves afterwards, even if the answer arrives. |

Anything else — a network failure, a refused connection — is whatever `fetch` threw, passed
through.
