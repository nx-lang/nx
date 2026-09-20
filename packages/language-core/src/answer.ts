/**
 * Answering one language query against a snapshot.
 *
 * <para>This is the half of NX language serving that has nothing to do with a transport. The HTTP
 * handler calls it after parsing a request; the in-process service calls it directly. Neither can
 * disagree with the other about a line number, because the queried text is analyzed exactly as the
 * caller sent it: a host's own declarations join the document set as context documents and are named
 * as implicit imports, so nothing is prepended and no position is ever rewritten.</para>
 */
import type {
  CompletionList,
  DiagnosticReport,
  DocumentSymbol,
  Hover,
  LanguageDocument,
  LanguageQueryName,
  TextPosition
} from "@nx-lang/language-protocol";

import type { SnapshotLike } from "./cache.js";

/**
 * A host's own declarations, served with every query.
 *
 * <para>`documents` join every query's document set without the client sending them, and
 * `implicitImports` are the identities every queried document imports without writing an import
 * line. A diagnostic located in a context document is reported under that document's own URI.</para>
 */
export interface HostContext {
  readonly documents?: readonly LanguageDocument[];
  readonly implicitImports?: readonly string[];
}

/** What one query needs to be answered: the set it is asked over, and where in it. */
export interface AnswerRequest {
  documents: LanguageDocument[];
  uri: string;
  position: TextPosition | undefined;
}

/**
 * The document set one query is analyzed over: the client's documents and the host's context.
 *
 * <para>A client document whose identity equals a context document's is a malformed request — the
 * client cannot replace the host's context — and is reported by the caller that validated it. This
 * returns the identity of such a collision, so one check serves both callers.</para>
 */
export function contextCollision(
  documents: readonly LanguageDocument[],
  context: HostContext | undefined
): ContextCollision | undefined {
  const contextUris = new Set((context?.documents ?? []).map((document) => document.uri));
  const contextIdentities = new Set(
    (context?.documents ?? [])
      .map(documentIdentity)
      .filter((identity): identity is string => identity !== undefined)
  );
  for (const document of documents) {
    if (contextUris.has(document.uri)) {
      return { kind: "uri", value: document.uri };
    }
    const identity = documentIdentity(document);
    if (identity !== undefined && contextIdentities.has(identity)) {
      return { kind: "identity", value: identity };
    }
  }
  return undefined;
}

/** What a client document and a context document collided on. */
export interface ContextCollision {
  readonly kind: "uri" | "identity";
  readonly value: string;
}

/** How a caller names a collision in a refusal, as a query or as a request. */
export function contextCollisionMessage(
  collision: ContextCollision,
  supplied: "query" | "request"
): string {
  return `A document with the ${collision.kind} '${collision.value}' is part of the host's context and cannot be supplied by a ${supplied}.`;
}

/**
 * A document's NX identity: the one it declares, or the one the SDK would derive from its URI.
 *
 * <para>The derivation is the SDK's, for the case a context document is declared the way both
 * READMEs declare one — a URI and a source, with no identity of its own. Without it a client could
 * replace a context document by naming its identity under a URI of the client's own. There is no
 * workspace root here, so a `file:` URI keeps its file name, exactly as a snapshot built without a
 * root does.</para>
 */
function documentIdentity(document: LanguageDocument): string | undefined {
  if (document.identity !== undefined) {
    return document.identity;
  }
  let url: URL;
  try {
    url = new URL(document.uri);
  } catch {
    // An unparseable URI is the SDK's to refuse, with its own message naming the URI.
    return undefined;
  }
  const pathSegments = url.pathname.split("/").filter((segment) => segment !== "");
  if (url.protocol === "file:") {
    return pathSegments.at(-1);
  }
  const segments = url.hostname === "" ? pathSegments : [url.hostname, ...pathSegments];
  return segments.length === 0 ? url.protocol.replace(":", "") : segments.join("/");
}

/** Answers one validated query, over the client's documents and the host's context. */
export function answerQuery(
  query: LanguageQueryName,
  request: AnswerRequest,
  context: HostContext | undefined,
  snapshotFor: (documents: LanguageDocument[]) => SnapshotLike
): unknown {
  const contextDocuments = context?.documents ?? [];
  const documents =
    contextDocuments.length === 0 ? request.documents : [...contextDocuments, ...request.documents];
  const snapshot = snapshotFor(documents);
  // The snapshot may have been built by an earlier request over the same text; its answers carry
  // that request's versions. The client discards answers by version, so it gets this request's.
  const versions = new Map(request.documents.map((document) => [document.uri, document.version ?? null]));
  // A document the request did not carry is a context document, whose version is the host's and not
  // this request's, so it is left as the snapshot reported it.
  const withVersion = <T extends { uri: string; version: number | null }>(answer: T): T =>
    versions.has(answer.uri) ? { ...answer, version: versions.get(answer.uri) ?? null } : answer;

  switch (query) {
    case "hover": {
      const hover = snapshot.hover(request.uri, request.position as TextPosition) as Hover | null;
      return hover === null ? null : withVersion(hover);
    }
    case "completions":
      return withVersion(snapshot.completions(request.uri, request.position as TextPosition) as CompletionList);
    case "diagnostics": {
      const report = snapshot.diagnostics() as DiagnosticReport;
      // Every document's diagnostics are in its own coordinates, a context document's included, so
      // there is nothing to move: only the client's own documents take this request's versions.
      return { ...report, documents: report.documents.map(withVersion) };
    }
    case "documentSymbols":
      return snapshot.documentSymbols(request.uri) as DocumentSymbol[];
  }
}
