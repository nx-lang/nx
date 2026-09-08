# @nx-lang/language-protocol

The contract between an editor and the NX language service: the shape of a language query and its
answer, the position encoding both sides use, and the `NxLanguageService` interface every client
implementation satisfies. Types, a few constants and guards, and the error classes; no runtime
dependency.

Implementations of the interface live elsewhere:

- `@nx-lang/language-client` — an HTTP implementation for browsers, talking to
- `@nx-lang/language-http` — a mountable Node handler that answers queries through `@nx-lang/sdk-node`.

An editor integration such as `@nx-lang/monaco` depends on this package alone, so an HTTP-backed,
worker-backed, or in-process service is interchangeable.

## The document-set model

Every query carries the complete set of documents it is to be answered from, plus the URI of the
document the query is about:

```ts
interface LanguageQuery {
  documents: { uri: string; source: string; version?: number; identity?: string }[];
  uri: string; // one of documents[].uri
}
```

The service keeps nothing between queries. Two identical queries produce identical answers whatever
was asked in between, which is what lets several server instances answer without session affinity.
A single-document host sends its one document; a host editing one file of a set that import each
other sends the whole set, with the edited document carrying the editor's unsaved text, and
completions include what the edited document imports from its siblings.

`version` is whatever the editor counts; it is echoed back in the answer so a client can discard an
answer that arrives after a newer version was sent. `identity` is the workspace identity NX resolves
imports by (`tenant/form.nx`); it is derived from the URI when omitted.

## Positions are UTF-16 code units

A `TextPosition` is a zero-based `line` and a zero-based `character` counted in UTF-16 code units
from the line start, the way `String.prototype.length` counts and the way Monaco, CodeMirror and the
Language Server Protocol default count. An emoji earlier on the line is two units. A range carries
its start and end positions and, alongside them, `startByte` and `endByte` offsets into the
document's UTF-8 text, which is what the language service measured.

```ts
// let s = "😀" + name
//                ^ name starts at character 15 (the emoji is 2 units) and byte 18 (it is 4 bytes)
{ start: { line: 0, character: 15 }, end: { line: 0, character: 19 }, startByte: 18, endByte: 22 }
```

## Queries

Four queries are defined; each has a request and an answer type that round-trip through JSON. The
examples below are compiled in `test/examples.test.ts`, so they cannot drift from the types.

### `hover`

```ts
const request: HoverRequest = {
  documents,
  uri: "nx://tenant/form.nx",
  position: { line: 1, character: 3 },
};

const answer: Hover | null = {
  uri: "nx://tenant/form.nx",
  identity: "tenant/form.nx",
  version: 7,
  range: { start: { line: 1, character: 1 }, end: { line: 1, character: 7 }, startByte: 34, endByte: 40 },
  contents: "```nx\n<Button label:string />\n```",
};
```

`contents` is markdown with NX fragments in ```` ```nx ```` fences. When the service has nothing to
say the answer is `null`, not an error.

### `completions`

```ts
const request: CompletionsRequest = {
  documents,
  uri: "nx://tenant/form.nx",
  position: { line: 1, character: 23 },
};

const answer: CompletionList = {
  uri: "nx://tenant/form.nx",
  identity: "tenant/form.nx",
  version: 7,
  items: [{ label: "label", kind: "Property", detail: "label:string" }],
};
```

`kind` is one of `Keyword`, `Type`, `Declaration`, `Component`, `Property`, `Member`.

### `diagnostics`

```ts
const request: DiagnosticsRequest = { documents, uri: "nx://tenant/form.nx" };

const answer: DiagnosticReport = {
  documents: [
    { uri: "nx://tenant/ui.nx", identity: "tenant/ui.nx", version: 3, diagnostics: [] },
    {
      uri: "nx://tenant/form.nx",
      identity: "tenant/form.nx",
      version: 7,
      diagnostics: [
        {
          range: { start: { line: 1, character: 8 }, end: { line: 1, character: 13 }, startByte: 41, endByte: 46 },
          severity: "Error",
          code: "type-mismatch",
          message: "Expected string, found int",
          related: [],
        },
      ],
    },
  ],
  workspace: [],
};
```

The report covers every document in the set, one entry each, so a multi-document host can set
markers on all of them from one answer. `workspace` holds diagnostics that belong to no document,
such as an import of a module that is in neither the set nor a library. `severity` is one of
`Error`, `Warning`, `Info`, `Hint`.

### `documentSymbols`

```ts
const request: DocumentSymbolsRequest = { documents, uri: "nx://tenant/ui.nx" };

const answer: DocumentSymbol[] = [
  {
    name: "Button",
    kind: "Component",
    range: { start: { line: 0, character: 0 }, end: { line: 0, character: 47 }, startByte: 0, endByte: 47 },
    selectionRange: { start: { line: 0, character: 12 }, end: { line: 0, character: 18 }, startByte: 12, endByte: 18 },
  },
];
```

`kind` is one of `Function`, `Value`, `TypeAlias`, `Record`, `Action`, `Union`, `Component`,
`Element`.

### Reserved names

`definition`, `references`, `rename`, `signatureHelp`, `inlayHints` and `semanticTokens` are
reserved. An implementation that receives one answers with the `unsupported-query` error rather
than a transport failure or a malformed answer, so a client can tell "not yet" from "broken".
`isLanguageQueryName` and `isReservedLanguageQueryName` classify a name.

## The service interface

```ts
interface NxLanguageService {
  hover(request: HoverRequest, signal?: AbortSignal): Promise<Hover | null>;
  completions(request: CompletionsRequest, signal?: AbortSignal): Promise<CompletionList>;
  diagnostics(request: DiagnosticsRequest, signal?: AbortSignal): Promise<DiagnosticReport>;
  documentSymbols(request: DocumentSymbolsRequest, signal?: AbortSignal): Promise<DocumentSymbol[]>;
}
```

Cancelling `signal` before the answer arrives rejects the promise with an abort error, which
`isAbortError` recognizes, and the promise never resolves afterwards. An editor integration is
tested against an in-process fake of this interface and behaves the same with the HTTP client for
identical answers.

## Errors

| Class | When |
| --- | --- |
| `UnsupportedQueryError` | The service recognized the query name but does not support it. Carries `query`. |
| `LanguageServiceHttpError` | An HTTP implementation received a non-success status. Carries `status`, `url`, a `bodyExcerpt`, and the body's `code` when it was an error body. |
| `LanguageServiceTimeoutError` | The request did not answer within the client's timeout; the request was aborted. Carries `timeoutMs`. |

An HTTP implementation answers a failed query with the JSON body

```ts
{ error: { code: LanguageServiceErrorCode; message: string; query?: string } }
```

where `code` is one of `unsupported-query`, `unknown-query`, `method-not-allowed`,
`invalid-request`, `payload-too-large`, `internal-error`. `isLanguageServiceErrorBody` recognizes
the shape.
