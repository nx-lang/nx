/**
 * A stateless, Fetch-shaped handler that answers `@nx-lang/language-protocol` queries through the
 * NX Node SDK.
 *
 * A host mounts it under any base path; the handler routes on the final path segment, accepts only
 * `POST` with a JSON body, and answers every request with JSON. Every answer is computed from the
 * request body alone, so instances behind a load balancer need no affinity; a small content-keyed
 * cache makes a burst of queries over unchanged text cost one analysis.
 */
import type { IncomingMessage, ServerResponse } from "node:http";
import { Readable } from "node:stream";
import {
  isLanguageQueryName,
  isReservedLanguageQueryName,
  type CompletionList,
  type DiagnosticReport,
  type DocumentDiagnostics,
  type DocumentSymbol,
  type EditorDiagnostic,
  type Hover,
  type LanguageDocument,
  type LanguageQueryName,
  type LanguageServiceErrorBody,
  type LanguageServiceErrorCode,
  type RelatedLocation,
  type TextPosition,
  type WorkspaceDiagnostic,
} from "@nx-lang/language-protocol";
import { NxLanguageSnapshot, type NxProgramBuildContext } from "@nx-lang/sdk-node";
import { SnapshotCache, documentSetKey, type SnapshotLike } from "./cache.js";
import {
  preludeOffsets,
  shiftPositionIn,
  shiftRangeOut,
  withPrelude,
  type PreludeOffsets,
} from "./prelude.js";

export { documentSetKey, SnapshotCache, type SnapshotLike } from "./cache.js";
export {
  preludeOffsets,
  shiftPositionIn,
  shiftRangeOut,
  withPrelude,
  type PreludeOffsets,
} from "./prelude.js";

/** The label identity a diagnostic carries when it lies inside the prelude rather than the document. */
export const PRELUDE_ORIGIN = "prelude";

/** What `createNxLanguageHandler` accepts. */
export interface NxLanguageHandlerOptions {
  /** Libraries every query can see. Without one, names declared only in a library are unresolved. */
  buildContext?: NxProgramBuildContext;
  /**
   * NX source placed ahead of the queried document before analysis, for hosts whose context
   * declarations cannot yet be imported without loss. Positions and ranges are shifted so the
   * client sees only its own coordinates.
   */
  prelude?: { source: string };
  /** Largest request body accepted, in bytes. Default 1 MiB. */
  maxBodyBytes?: number;
  /** How many analyzed document sets to keep. Default 8. */
  cacheSize?: number;
  /** Called with any failure the handler turns into a 500, so the host can log it. */
  onError?: (error: unknown, context: { query: string }) => void;
  /**
   * Builds the snapshot for a document set. Defaults to `new NxLanguageSnapshot(documents, { buildContext })`;
   * a test injects a counting or throwing factory here.
   */
  createSnapshot?: (
    documents: readonly LanguageDocument[],
    options: { buildContext?: NxProgramBuildContext },
  ) => SnapshotLike;
}

/** A Fetch-shaped request handler. */
export type NxLanguageHandler = (request: Request) => Promise<Response>;

const DEFAULT_MAX_BODY_BYTES = 1024 * 1024;
const DEFAULT_CACHE_SIZE = 8;

/** Creates the handler. */
export function createNxLanguageHandler(options: NxLanguageHandlerOptions = {}): NxLanguageHandler {
  const maxBodyBytes = options.maxBodyBytes ?? DEFAULT_MAX_BODY_BYTES;
  const cache = new SnapshotCache<SnapshotLike>(options.cacheSize ?? DEFAULT_CACHE_SIZE);
  const prelude = options.prelude === undefined ? undefined : preludeOffsets(options.prelude.source);
  const createSnapshot =
    options.createSnapshot ??
    ((documents, snapshotOptions) => new NxLanguageSnapshot(documents, snapshotOptions));
  const snapshotOptions: { buildContext?: NxProgramBuildContext } =
    options.buildContext === undefined ? {} : { buildContext: options.buildContext };

  return async (request: Request): Promise<Response> => {
    const query = lastPathSegment(request.url);

    if (request.method !== "POST") {
      return errorResponse(405, "method-not-allowed", `Use POST; ${request.method} is not accepted.`, query, {
        allow: "POST",
      });
    }
    if (!isLanguageQueryName(query)) {
      if (isReservedLanguageQueryName(query)) {
        return errorResponse(501, "unsupported-query", `The '${query}' query is not supported yet.`, query);
      }
      return errorResponse(404, "unknown-query", `'${query}' is not a language query.`, query);
    }

    let body: unknown;
    try {
      body = await readJsonBody(request, maxBodyBytes);
    } catch (error) {
      if (error instanceof BodyTooLargeError) {
        return errorResponse(413, "payload-too-large", error.message, query);
      }
      return errorResponse(400, "invalid-request", "Request body is not valid JSON.", query);
    }

    const validated = validateQuery(query, body);
    if ("problem" in validated) {
      return errorResponse(400, "invalid-request", validated.problem, query);
    }
    const { documents, uri, position } = validated;

    try {
      const answer = answerQuery(query, { documents, uri, position }, prelude, (analyzed) =>
        cache.getOrCreate(documentSetKey(analyzed), () => createSnapshot(analyzed, snapshotOptions)),
      );
      return jsonResponse(200, answer);
    } catch (error) {
      if (isSdkInputError(error)) {
        return errorResponse(400, "invalid-request", (error as Error).message, query);
      }
      options.onError?.(error, { query });
      return errorResponse(500, "internal-error", "The language service failed to answer.", query);
    }
  };
}

/** Answers one validated query, applying the prelude around the snapshot when there is one. */
function answerQuery(
  query: LanguageQueryName,
  request: { documents: LanguageDocument[]; uri: string; position: TextPosition | undefined },
  prelude: PreludeOffsets | undefined,
  snapshotFor: (documents: LanguageDocument[]) => SnapshotLike,
): unknown {
  const documents =
    prelude === undefined
      ? request.documents
      : request.documents.map((document) =>
          document.uri === request.uri ? { ...document, source: withPrelude(prelude, document.source) } : document,
        );
  const position =
    request.position === undefined || prelude === undefined
      ? request.position
      : shiftPositionIn(request.position, prelude);
  const snapshot = snapshotFor(documents);
  // The snapshot may have been built by an earlier request over the same text; its answers carry
  // that request's versions. The client discards answers by version, so it gets this request's.
  const versions = new Map(request.documents.map((document) => [document.uri, document.version ?? null]));
  const withVersion = <T extends { uri: string; version: number | null }>(answer: T): T => ({
    ...answer,
    version: versions.get(answer.uri) ?? null,
  });

  switch (query) {
    case "hover": {
      const hover = snapshot.hover(request.uri, position as TextPosition) as Hover | null;
      if (hover === null) {
        return null;
      }
      if (prelude === undefined) {
        return withVersion(hover);
      }
      const range = shiftRangeOut(hover.range, prelude);
      return range === null ? null : withVersion({ ...hover, range });
    }
    case "completions":
      return withVersion(snapshot.completions(request.uri, position as TextPosition) as CompletionList);
    case "diagnostics": {
      const report = snapshot.diagnostics() as DiagnosticReport;
      const shifted = prelude === undefined ? report : shiftReport(report, request.uri, prelude);
      return { ...shifted, documents: shifted.documents.map(withVersion) };
    }
    case "documentSymbols": {
      const symbols = snapshot.documentSymbols(request.uri) as DocumentSymbol[];
      if (prelude === undefined) {
        return symbols;
      }
      return symbols.flatMap((symbol) => {
        const range = shiftRangeOut(symbol.range, prelude);
        const selectionRange = shiftRangeOut(symbol.selectionRange, prelude);
        return range === null || selectionRange === null ? [] : [{ ...symbol, range, selectionRange }];
      });
    }
  }
}

/**
 * Moves the queried document's diagnostics back into its own coordinates.
 *
 * A diagnostic inside the prelude is the host's fault, not the author's: it is reported as a
 * workspace diagnostic labelled with the prelude origin and no range, rather than positioned on a
 * line the author cannot see. A related location that points into the queried document is shifted
 * wherever it appears, since a sibling's diagnostic names the same combined text.
 */
function shiftReport(report: DiagnosticReport, uri: string, prelude: PreludeOffsets): DiagnosticReport {
  const workspace: WorkspaceDiagnostic[] = [...report.workspace];
  const documents: DocumentDiagnostics[] = report.documents.map((document) => {
    if (document.uri !== uri) {
      return {
        ...document,
        diagnostics: document.diagnostics.map((diagnostic) => ({
          ...diagnostic,
          related: shiftRelated(diagnostic.related, uri, prelude),
        })),
      };
    }
    const diagnostics: EditorDiagnostic[] = [];
    for (const diagnostic of document.diagnostics) {
      const range = shiftRangeOut(diagnostic.range, prelude);
      if (range === null) {
        workspace.push({
          severity: diagnostic.severity,
          code: diagnostic.code,
          message: diagnostic.message,
          labels: [{ identity: PRELUDE_ORIGIN, message: null }],
        });
        continue;
      }
      diagnostics.push({ ...diagnostic, range, related: shiftRelated(diagnostic.related, uri, prelude) });
    }
    return { ...document, diagnostics };
  });
  return { documents, workspace };
}

/** Related locations with those in the queried document shifted; one inside the prelude is dropped. */
function shiftRelated(related: readonly RelatedLocation[], uri: string, prelude: PreludeOffsets): RelatedLocation[] {
  const shifted: RelatedLocation[] = [];
  for (const location of related) {
    if (location.uri !== uri) {
      shifted.push(location);
      continue;
    }
    const range = shiftRangeOut(location.range, prelude);
    if (range !== null) {
      shifted.push({ ...location, range });
    }
  }
  return shifted;
}

// ---------------------------------------------------------------------------------------------
// Request parsing
// ---------------------------------------------------------------------------------------------

class BodyTooLargeError extends Error {
  constructor(limit: number) {
    super(`Request body exceeds ${limit} bytes.`);
  }
}

function lastPathSegment(url: string): string {
  let pathname: string;
  try {
    pathname = new URL(url, "http://localhost").pathname;
  } catch {
    return "";
  }
  const segments = pathname.split("/").filter((segment) => segment.length > 0);
  return decodeURIComponent(segments[segments.length - 1] ?? "");
}

async function readJsonBody(request: Request, maxBodyBytes: number): Promise<unknown> {
  const declared = Number(request.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > maxBodyBytes) {
    throw new BodyTooLargeError(maxBodyBytes);
  }
  if (request.body === null) {
    throw new SyntaxError("empty body");
  }
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  // Past the limit the rest of the body is read and dropped rather than cancelled, up to a
  // ceiling: a connection whose request was fully consumed can carry the client's next, smaller
  // request, while one cut off mid-upload has to be closed. A body that keeps coming past the
  // ceiling is not a client that will shrink its request, and is cut off.
  const drainCeiling = Math.max(4 * maxBodyBytes, 8 * 1024 * 1024);
  let tooLarge = false;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    size += value.byteLength;
    if (size > drainCeiling) {
      await reader.cancel();
      throw new BodyTooLargeError(maxBodyBytes);
    }
    if (size > maxBodyBytes) {
      tooLarge = true;
      chunks.length = 0;
      continue;
    }
    chunks.push(value);
  }
  if (tooLarge) {
    throw new BodyTooLargeError(maxBodyBytes);
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

type ValidatedQuery =
  | { documents: LanguageDocument[]; uri: string; position: TextPosition | undefined }
  | { problem: string };

/** Checks the body has what the query needs, naming the first thing missing. */
function validateQuery(query: LanguageQueryName, body: unknown): ValidatedQuery {
  if (typeof body !== "object" || body === null || Array.isArray(body)) {
    return { problem: "Request body must be a JSON object with 'documents' and 'uri'." };
  }
  const record = body as Record<string, unknown>;
  if (!Array.isArray(record.documents)) {
    return { problem: "Request is missing 'documents', an array of { uri, source }." };
  }
  if (record.documents.length === 0) {
    return { problem: "'documents' must contain at least one document." };
  }
  const documents: LanguageDocument[] = [];
  for (const [index, candidate] of record.documents.entries()) {
    if (typeof candidate !== "object" || candidate === null) {
      return { problem: `documents[${index}] must be an object with 'uri' and 'source'.` };
    }
    const entry = candidate as Record<string, unknown>;
    if (typeof entry.uri !== "string") {
      return { problem: `documents[${index}] is missing 'uri'.` };
    }
    if (typeof entry.source !== "string") {
      return { problem: `documents[${index}] ('${entry.uri}') is missing 'source'.` };
    }
    if (entry.version !== undefined && typeof entry.version !== "number") {
      return { problem: `documents[${index}] ('${entry.uri}') has a non-numeric 'version'.` };
    }
    if (entry.identity !== undefined && typeof entry.identity !== "string") {
      return { problem: `documents[${index}] ('${entry.uri}') has a non-string 'identity'.` };
    }
    documents.push({
      uri: entry.uri,
      source: entry.source,
      ...(entry.version === undefined ? {} : { version: entry.version }),
      ...(entry.identity === undefined ? {} : { identity: entry.identity }),
    });
  }
  if (typeof record.uri !== "string") {
    return { problem: "Request is missing 'uri', the document the query is about." };
  }
  const uri = record.uri;
  if (!documents.some((document) => document.uri === uri)) {
    return { problem: `'uri' (${uri}) names no document in 'documents'.` };
  }
  let position: TextPosition | undefined;
  if (query === "hover" || query === "completions") {
    const candidate = record.position as Record<string, unknown> | undefined;
    if (
      typeof candidate !== "object" ||
      candidate === null ||
      !Number.isInteger(candidate.line) ||
      !Number.isInteger(candidate.character) ||
      (candidate.line as number) < 0 ||
      (candidate.character as number) < 0
    ) {
      return { problem: `The '${query}' query needs 'position', { line, character } with non-negative integers.` };
    }
    position = { line: candidate.line as number, character: candidate.character as number };
  }
  return { documents, uri, position };
}

/** The SDK reports bad snapshot input (an unparseable URI, a duplicate identity) as an evaluation error. */
function isSdkInputError(error: unknown): boolean {
  return error instanceof Error && error.name === "NxEvaluationError";
}

// ---------------------------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------------------------

function jsonResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...headers },
  });
}

function errorResponse(
  status: number,
  code: LanguageServiceErrorCode,
  message: string,
  query: string,
  headers: Record<string, string> = {},
): Response {
  const body: LanguageServiceErrorBody = { error: { code, message, query } };
  return jsonResponse(status, body, headers);
}

// ---------------------------------------------------------------------------------------------
// Node adapter
// ---------------------------------------------------------------------------------------------

/** A Node `http` request listener. */
export type NodeListener = (request: IncomingMessage, response: ServerResponse) => void;

/**
 * Presents the handler with Node's `http` listener signature.
 *
 * The incoming message is wrapped as a `Request` with its body streamed, so the handler's body
 * limit applies as the bytes arrive; the `Response` is written back status, headers and body.
 */
export function toNodeListener(handler: NxLanguageHandler): NodeListener {
  return (incoming, outgoing) => {
    let request: Request;
    try {
      request = toRequest(incoming);
    } catch (error) {
      // Only the path matters for routing, but a request line Node's parser lets through can
      // still be one `Request` refuses. That is the client's problem, not the process's.
      const body: LanguageServiceErrorBody = {
        error: {
          code: "invalid-request",
          message: `The request could not be read: ${error instanceof Error ? error.message : String(error)}`,
          query: "",
        },
      };
      writeJson(outgoing, 400, body);
      return;
    }

    handler(request)
      .then(async (response) => {
        const body = Buffer.from(await response.arrayBuffer());
        const responseHeaders: Record<string, string> = {};
        response.headers.forEach((value, name) => {
          responseHeaders[name] = value;
        });
        responseHeaders["content-length"] = String(body.byteLength);
        if (!incoming.complete) {
          // The body was refused before it was fully read. Answer, then close: a client told 413
          // can shrink its request, but a connection still carrying the rest of the old body
          // cannot be reused for the next one.
          responseHeaders["connection"] = "close";
          outgoing.on("finish", () => incoming.destroy());
        }
        outgoing.writeHead(response.status, responseHeaders);
        outgoing.end(body);
      })
      .catch((error: unknown) => {
        if (!outgoing.headersSent) {
          writeJson(outgoing, 500, {
            error: { code: "internal-error", message: error instanceof Error ? error.message : String(error) },
          });
        } else {
          outgoing.end();
        }
      });
  };
}

/**
 * Node's incoming message as a `Request` with its body streamed.
 *
 * The URL is built on a fixed origin from the path alone: routing needs only the final path
 * segment, and the `Host` header is the client's to fill with anything, including text that is
 * not a valid authority.
 */
function toRequest(incoming: IncomingMessage): Request {
  const url = new URL(incoming.url ?? "/", "http://localhost");
  const method = incoming.method ?? "GET";
  const headers = new Headers();
  for (const [name, value] of Object.entries(incoming.headers)) {
    if (typeof value === "string") {
      headers.set(name, value);
    } else if (Array.isArray(value)) {
      for (const item of value) {
        headers.append(name, item);
      }
    }
  }
  const hasBody = method !== "GET" && method !== "HEAD";
  return new Request(url, {
    method,
    headers,
    ...(hasBody ? { body: Readable.toWeb(incoming) as unknown as BodyInit, duplex: "half" } : {}),
  } as RequestInit);
}

function writeJson(outgoing: ServerResponse, status: number, body: unknown): void {
  const text = JSON.stringify(body);
  outgoing.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": String(Buffer.byteLength(text)),
  });
  outgoing.end(text);
}
