/**
 * The contract between an editor and the NX language service.
 *
 * Every query carries the documents it is asked against, so a service that implements this
 * contract keeps no state between queries. Positions count UTF-16 code units, the encoding every
 * browser editor and the Language Server Protocol default to. The answer shapes are exactly what
 * the NX language service serializes, with camelCase property names, so an in-process, worker or
 * HTTP implementation of {@link NxLanguageService} can hand results through without translation.
 */

// ---------------------------------------------------------------------------------------------
// Documents and positions
// ---------------------------------------------------------------------------------------------

/** One document in the set a query is asked against. */
export interface LanguageDocument {
  /**
   * The logical URI the document is addressed by, such as `nx://tenant/form.nx` or a `file:` URI.
   * Two documents in one query never share a URI.
   */
  uri: string;
  /** The document's complete source text as the editor holds it, unsaved edits included. */
  source: string;
  /**
   * The editor's version of this text. Echoed back in the answer so the client can discard an
   * answer that arrives after a newer version was sent.
   */
  version?: number;
  /**
   * The workspace identity the NX module resolves imports by, such as `tenant/form.nx`. Derived
   * from the URI when omitted.
   */
  identity?: string;
}

/** A zero-based position: a line, and an offset within it counted in UTF-16 code units. */
export interface TextPosition {
  line: number;
  /** Offset within the line in UTF-16 code units, the way `String.prototype.length` counts. */
  character: number;
}

/**
 * A range in a document, in editor positions and in UTF-8 byte offsets.
 *
 * The byte offsets index the document's UTF-8 text and are what the language service measured;
 * the positions are the same span in the editor's units.
 */
export interface EditorRange {
  start: TextPosition;
  end: TextPosition;
  startByte: number;
  endByte: number;
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

/** What every query carries: the documents to analyze and which one the query is about. */
export interface LanguageQuery {
  /** The complete document set the answer is computed from. */
  documents: LanguageDocument[];
  /** The URI of the document the query is about; one of `documents`. */
  uri: string;
}

/** Hover at a position. */
export interface HoverRequest extends LanguageQuery {
  position: TextPosition;
}

/** Completions at a position. */
export interface CompletionsRequest extends LanguageQuery {
  position: TextPosition;
}

/** Diagnostics for the document set. */
export type DiagnosticsRequest = LanguageQuery;

/** Top-level symbols of one document. */
export type DocumentSymbolsRequest = LanguageQuery;

/**
 * Every piece of one document as a typed node, with the declarations the nodes refer to.
 *
 * Unstable: the answer's shape may change until a later release commits to it.
 */
export type SourceTreeRequest = LanguageQuery;

/** The queries this protocol defines. */
export const LANGUAGE_QUERIES = [
  "hover",
  "completions",
  "diagnostics",
  "documentSymbols",
  "sourceTree",
] as const;

/** A query name this protocol defines. */
export type LanguageQueryName = (typeof LANGUAGE_QUERIES)[number];

/**
 * Query names reserved for features the protocol does not define yet.
 *
 * An implementation that receives one answers with an {@link UnsupportedQueryError} rather than a
 * transport failure, so a client can tell "not yet" from "broken".
 */
export const RESERVED_LANGUAGE_QUERIES = [
  "definition",
  "references",
  "rename",
  "signatureHelp",
  "inlayHints",
  "semanticTokens",
] as const;

/** A reserved query name. */
export type ReservedLanguageQueryName = (typeof RESERVED_LANGUAGE_QUERIES)[number];

/** The request shape of each defined query. */
export interface LanguageQueryRequests {
  hover: HoverRequest;
  completions: CompletionsRequest;
  diagnostics: DiagnosticsRequest;
  documentSymbols: DocumentSymbolsRequest;
  sourceTree: SourceTreeRequest;
}

/** The answer shape of each defined query. */
export interface LanguageQueryAnswers {
  hover: Hover | null;
  completions: CompletionList;
  diagnostics: DiagnosticReport;
  documentSymbols: DocumentSymbol[];
  sourceTree: SourceTree;
}

/** Returns whether `name` is a query this protocol defines. */
export function isLanguageQueryName(name: string): name is LanguageQueryName {
  return (LANGUAGE_QUERIES as readonly string[]).includes(name);
}

/** Returns whether `name` is reserved for a future query. */
export function isReservedLanguageQueryName(name: string): name is ReservedLanguageQueryName {
  return (RESERVED_LANGUAGE_QUERIES as readonly string[]).includes(name);
}

// ---------------------------------------------------------------------------------------------
// Answers
// ---------------------------------------------------------------------------------------------

/** Hover content for a position. The answer to a hover query is this or `null`. */
export interface Hover {
  uri: string;
  identity: string;
  version: number | null;
  /** The range the content applies to. */
  range: EditorRange;
  /** Markdown, with NX fragments in ```nx fences. */
  contents: string;
}

/** The kinds of completion item the language service produces. */
export type CompletionItemKind =
  | "Keyword"
  | "Type"
  | "Declaration"
  | "Component"
  | "Property"
  | "Member";

/** One completion candidate. */
export interface CompletionItem {
  label: string;
  kind: CompletionItemKind;
  detail: string | null;
  /**
   * The documentation of what the item names, as markdown rendered the way hover renders it.
   * Absent when it has none.
   */
  documentation?: string;
}

/** The answer to a completions query. */
export interface CompletionList {
  uri: string;
  identity: string;
  version: number | null;
  items: CompletionItem[];
}

/** Diagnostic severity. */
export type DiagnosticSeverity = "Error" | "Warning" | "Info" | "Hint";

/** A location a diagnostic refers to in some document of the set. */
export interface RelatedLocation {
  uri: string;
  identity: string;
  range: EditorRange;
  message: string | null;
}

/** One diagnostic positioned in a document. */
export interface EditorDiagnostic {
  range: EditorRange;
  severity: DiagnosticSeverity;
  code: string | null;
  message: string;
  related: RelatedLocation[];
}

/** The diagnostics of one document in the set. */
export interface DocumentDiagnostics {
  uri: string;
  identity: string;
  version: number | null;
  diagnostics: EditorDiagnostic[];
}

/** A label of a diagnostic that names a module identity rather than a document range. */
export interface WorkspaceDiagnosticLabel {
  identity: string;
  message: string | null;
}

/** A diagnostic not attributable to a document in the set. */
export interface WorkspaceDiagnostic {
  severity: DiagnosticSeverity;
  code: string | null;
  message: string;
  labels: WorkspaceDiagnosticLabel[];
}

/**
 * The answer to a diagnostics query: the diagnostics of every document in the set, and separately
 * those that belong to no document.
 */
export interface DiagnosticReport {
  documents: DocumentDiagnostics[];
  workspace: WorkspaceDiagnostic[];
}

/** The kinds of top-level symbol the language service reports. */
export type DocumentSymbolKind =
  | "Function"
  | "Value"
  | "TypeAlias"
  | "Record"
  | "Action"
  | "Union"
  | "Component"
  | "Element";

/** One top-level symbol of a document. */
export interface DocumentSymbol {
  name: string;
  kind: DocumentSymbolKind;
  /** The whole declaration. */
  range: EditorRange;
  /** The declared name. */
  selectionRange: EditorRange;
}

/**
 * What a source node is: a construct a reader recognizes rather than a grammar production.
 *
 * `binding` is a name a `for` binds; `matchArm` is an arm of an `is` expression or of a condition
 * list; `unparsed` is a region the parser could not read.
 */
export type SourceRole =
  | "import"
  | "declaration"
  | "parameter"
  | "stateField"
  | "field"
  | "unionCase"
  | "emit"
  | "typeReference"
  | "element"
  | "attribute"
  | "text"
  | "embed"
  | "literal"
  | "empty"
  | "case"
  | "reference"
  | "member"
  | "operator"
  | "call"
  | "sequence"
  | "condition"
  | "match"
  | "matchArm"
  | "loop"
  | "binding"
  | "comment"
  | "docComment"
  | "unparsed";

/** A variation of a construct that matters to a renderer and not to its role. */
export type SourceFlag =
  | "braced"
  | "parenthesized"
  | "content"
  | "handler"
  | "stateUpdate"
  | "inherited"
  | "optional"
  | "raw"
  | "abstract"
  | "external"
  | "export"
  | "private";

/**
 * One piece of the source. Every token of the document belongs to exactly one node, the smallest
 * whose range holds it.
 */
export interface SourceNode {
  role: SourceRole;
  /** The range the construct covers, children included. */
  range: EditorRange;
  /** The index of the enclosing node in `nodes`; absent for a top-level node. */
  parent?: number;
  /**
   * The node's path within the document, such as `roleQuestion.value.choices[2].label`: stable
   * under edits elsewhere, and unique within the tree. Compare keys for equality; do not split
   * them, since a segment written from the source, such as an arm's pattern, may hold a dot. The
   * tree's structure is in `parent`.
   */
  key: string;
  /** The name the construct declares or names: a declaration's, an attribute's, a tag, a case. */
  name?: string;
  /** What the construct says as written: a literal, an operator's token, a type, a text run, a comment. */
  value?: string;
  /** The text type of a typed body, as `markdown` in `<Note:markdown>`. */
  textType?: string;
  /** The construct's type spelled in NX, wherever the type checker gave it one. */
  type?: string;
  /** The index in `declarations` of what the construct refers to; for a declaration, its own entry. */
  declaration?: number;
  flags?: SourceFlag[];
}

/** The kinds of declaration a source tree's declaration table describes. */
export type SourceDeclarationKind =
  | "record"
  | "action"
  | "union"
  | "alias"
  | "component"
  | "function"
  | "value";

/** One property of a declaration, or one field of a union case or of a component's state. */
export interface SourceProperty {
  name: string;
  /** The declared type spelled in NX. */
  type: string;
  /** The source text of the default, when the property has one. */
  default?: string;
  /** The doc comment, rendered as hover renders it. */
  doc?: string;
  /** `optional`, `content` and `inherited`, where they apply. */
  flags?: SourceFlag[];
}

/** One case of a union. */
export interface SourceCase {
  name: string;
  doc?: string;
  properties?: SourceProperty[];
}

/** One declaration a source node refers to, from this module or any other. */
export interface SourceDeclaration {
  /** The identity of the declaring module. */
  module: string;
  name: string;
  kind: SourceDeclarationKind;
  /** The whole declaration, when the queried document declares it. */
  range?: EditorRange;
  /** The doc comment, rendered as hover renders it. */
  doc?: string;
  /** Properties in declaration order, inherited ones first. */
  properties?: SourceProperty[];
  /** A component's state fields. */
  state?: SourceProperty[];
  /** The indices in `declarations` of the declaration's bases, nearest first. */
  bases?: number[];
  /** A union's cases. */
  cases?: SourceCase[];
  /** The type an alias names, or a value's type, spelled in NX. */
  type?: string;
  /** The range of a value declaration's value, when the queried document declares it. */
  valueRange?: EditorRange;
}

/**
 * The answer to a source tree query: the document's nodes in order of start offset, each parent
 * before its children, and the declarations they refer to.
 *
 * Unstable: the shape may change until a later release commits to it.
 */
export interface SourceTree {
  uri: string;
  identity: string;
  version: number | null;
  nodes: SourceNode[];
  declarations: SourceDeclaration[];
}

// ---------------------------------------------------------------------------------------------
// Service interface
// ---------------------------------------------------------------------------------------------

/**
 * One method per defined query.
 *
 * Editor integrations depend on this interface alone. Cancelling `signal` before an answer arrives
 * rejects the promise with an abort error (see {@link isAbortError}) and the promise never resolves
 * afterwards.
 */
export interface NxLanguageService {
  hover(request: HoverRequest, signal?: AbortSignal): Promise<Hover | null>;
  completions(request: CompletionsRequest, signal?: AbortSignal): Promise<CompletionList>;
  diagnostics(request: DiagnosticsRequest, signal?: AbortSignal): Promise<DiagnosticReport>;
  documentSymbols(request: DocumentSymbolsRequest, signal?: AbortSignal): Promise<DocumentSymbol[]>;
  /** Unstable: the answer's shape may change until a later release commits to it. */
  sourceTree(request: SourceTreeRequest, signal?: AbortSignal): Promise<SourceTree>;
}

// ---------------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------------

/** The error codes an implementation answers with when it cannot answer a query. */
export type LanguageServiceErrorCode =
  | "unsupported-query"
  | "unknown-query"
  | "method-not-allowed"
  | "invalid-request"
  | "payload-too-large"
  | "internal-error";

/** The JSON body an HTTP implementation answers a failed query with. */
export interface LanguageServiceErrorBody {
  error: {
    code: LanguageServiceErrorCode;
    message: string;
    /** The query name, when the failure is about which query was asked. */
    query?: string;
  };
}

/** Returns whether `value` has the shape of {@link LanguageServiceErrorBody}. */
export function isLanguageServiceErrorBody(value: unknown): value is LanguageServiceErrorBody {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const error = (value as { error?: unknown }).error;
  return (
    typeof error === "object" &&
    error !== null &&
    typeof (error as { code?: unknown }).code === "string" &&
    typeof (error as { message?: unknown }).message === "string"
  );
}

/** A query the implementation recognizes but does not support: a reserved name, or a defined one it opted out of. */
export class UnsupportedQueryError extends Error {
  override readonly name = "UnsupportedQueryError";
  readonly query: string;

  constructor(query: string, message = `The language service does not support the '${query}' query.`) {
    super(message);
    this.query = query;
  }
}

/** A non-success HTTP answer, with the status and the start of the body for the log. */
export class LanguageServiceHttpError extends Error {
  override readonly name = "LanguageServiceHttpError";
  readonly status: number;
  readonly url: string;
  readonly bodyExcerpt: string;
  /** The error code from the JSON body, when the body was one. */
  readonly code: LanguageServiceErrorCode | undefined;

  constructor(options: { status: number; url: string; bodyExcerpt: string; code?: LanguageServiceErrorCode }) {
    super(
      `Language service request to ${options.url} failed with status ${options.status}` +
        (options.bodyExcerpt ? `: ${options.bodyExcerpt}` : ""),
    );
    this.status = options.status;
    this.url = options.url;
    this.bodyExcerpt = options.bodyExcerpt;
    this.code = options.code;
  }
}

/** A query that did not answer within the client's timeout. The underlying request was aborted. */
export class LanguageServiceTimeoutError extends Error {
  override readonly name = "LanguageServiceTimeoutError";
  readonly timeoutMs: number;
  readonly url: string;

  constructor(options: { timeoutMs: number; url: string }) {
    super(`Language service request to ${options.url} timed out after ${options.timeoutMs} ms.`);
    this.timeoutMs = options.timeoutMs;
    this.url = options.url;
  }
}

/**
 * Returns whether `error` is the rejection a cancelled signal produces.
 *
 * `fetch` rejects with a `DOMException` named `AbortError`; an in-process implementation may throw
 * `signal.reason`, which is the same when the caller used `AbortController.abort()` without a reason.
 */
export function isAbortError(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { name?: unknown }).name === "AbortError"
  );
}
