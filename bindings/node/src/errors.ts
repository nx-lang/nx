import type { NxDiagnostic } from "./types.js";

/**
 * Error thrown when NX validation, build, IR generation, or evaluation fails with structured diagnostics.
 */
export class NxEvaluationError extends Error {
  /**
   * Diagnostics reported by the NX host for the failed operation.
   */
  public readonly diagnostics: readonly NxDiagnostic[];

  /**
   * Creates an error that preserves NX diagnostics for programmatic handling.
   */
  public constructor(message: string, diagnostics: readonly NxDiagnostic[], options?: ErrorOptions) {
    super(message, options);
    this.name = "NxEvaluationError";
    this.diagnostics = diagnostics;
  }
}

/**
 * Error thrown when the native Node binding cannot be loaded or returns an unexpected interop failure.
 */
export class NxNativeError extends Error {
  /**
   * Creates a native binding error.
   */
  public constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "NxNativeError";
  }
}

/**
 * Error thrown when an operation is attempted on a disposed NX SDK resource.
 */
export class NxDisposedResourceError extends Error {
  /**
   * Name of the disposed resource type.
   */
  public readonly resourceName: string;

  /**
   * Creates a disposed-resource error for the named resource type.
   */
  public constructor(resourceName: string, options?: ErrorOptions) {
    super(`${resourceName} has been disposed.`, options);
    this.name = "NxDisposedResourceError";
    this.resourceName = resourceName;
  }
}
