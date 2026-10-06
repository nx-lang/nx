import type { NxIrDiagnostic } from "@nx-lang/ir-runtime";

/**
 * A span of a module's source: byte offsets and 1-based lines and columns, as the declaration
 * schema export answers a function's `declaration`.
 */
export interface NxAgentTextSpan {
  readonly startByte: number;
  readonly endByte: number;
  readonly startLine: number;
  readonly startColumn: number;
  readonly endLine: number;
  readonly endColumn: number;
}

export type NxAgentDiagnosticCode =
  | "nx-agent-not-an-agent"
  | "nx-agent-duplicate-document-title"
  | "nx-agent-duplicate-tool-name"
  | "nx-agent-invalid-tool-name"
  | "nx-agent-missing-description"
  | "nx-agent-inexpressible-parameter"
  | "nx-agent-inexpressible-result"
  | "nx-agent-unknown-function"
  | "nx-agent-context-parameter"
  | "nx-agent-unknown-tool-type"
  | "nx-agent-describer"
  | "nx-agent-http-base-url"
  | "nx-agent-http-path"
  | "nx-agent-http-arguments-type"
  | "nx-agent-unsupported-format"
  | "nx-agent-invalid-definition"
  | "nx-agent-missing-executor"
  | "nx-agent-missing-request-function";

/** A problem with an agent or a stored definition, found before any tool runs. */
export interface NxAgentDiagnostic {
  /** `nx-agent-inexpressible-result` is the only warning. */
  readonly severity: "error" | "warning";
  readonly code: NxAgentDiagnosticCode;
  readonly message: string;
  /** Where the value is inside the agent, such as `tools[1]`. Empty for the agent itself. */
  readonly path: string;
  /** The tool's model-facing name, when one was resolved. */
  readonly tool?: string;
  /**
   * The function a diagnostic about a tool's function is about: its module, and the span of its
   * declaration in that module's source when the schema source answered one.
   */
  readonly declaration?: { readonly module: string; readonly span?: NxAgentTextSpan };
}

/** A configuration error: an agent that does not normalize, or a definition that cannot run. */
export class NxAgentError extends Error {
  public readonly diagnostics: readonly NxAgentDiagnostic[];

  public constructor(diagnostics: readonly NxAgentDiagnostic[]) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
    this.name = "NxAgentError";
    this.diagnostics = diagnostics;
  }
}

/** The limit a tool call reached: its name and, where it has one, its value. */
export interface ToolLimit {
  readonly name: string;
  readonly value?: number;
}

/** What a call that reached the IR runtime used, as the runtime reports it. */
export interface ToolUsage {
  readonly operations?: number;
  readonly inputSize?: number;
}

/**
 * The codes the package gives a failed tool call. A host function may throw an
 * `NxAgentToolError` with a code of its own, which is kept.
 */
export type ToolErrorCode =
  | "invalid-input"
  | "invalid-context"
  | "evaluation-failed"
  | "resource-limit"
  | "invalid-request"
  | "request-failed"
  | "aborted";

/** The error of a failed tool call, as a result holds it. */
export interface ToolError {
  readonly code: ToolErrorCode | (string & {});
  readonly message: string;
  /** The IR runtime's diagnostics, where it produced them. */
  readonly diagnostics?: readonly NxIrDiagnostic[];
  readonly limit?: ToolLimit;
}

/**
 * A tool failure with a `code`. A host's request function or executor throws one to choose the
 * code of the failed result, and the AI SDK adapter throws one for every failed result.
 */
export class NxAgentToolError extends Error {
  public readonly code: string;
  // Declared, not defined: a field would put the key on every error, holding `undefined`.
  public declare readonly diagnostics?: readonly NxIrDiagnostic[];
  public declare readonly limit?: ToolLimit;

  /** A member of `details` that is `undefined` is left off the error, as one that is absent is. */
  public constructor(
    code: string,
    message: string,
    details: { readonly diagnostics?: readonly NxIrDiagnostic[] | undefined; readonly limit?: ToolLimit | undefined } = {},
  ) {
    super(message);
    this.name = "NxAgentToolError";
    this.code = code;
    if (details.diagnostics !== undefined) {
      this.diagnostics = details.diagnostics;
    }
    if (details.limit !== undefined) {
      this.limit = details.limit;
    }
  }
}
