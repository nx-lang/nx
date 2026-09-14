import type { NxDiagnostic } from "./types.js";

/**
 * Error thrown when NX build or IR generation fails with structured diagnostics.
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
 * Error thrown when the WebAssembly module cannot be loaded or answers in an unexpected shape.
 */
export class NxWasmError extends Error {
  /**
   * Creates a wasm binding error.
   */
  public constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "NxWasmError";
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

/**
 * Error thrown when the module traps, and on every later call to the host that trapped.
 *
 * <para>The module targets `wasm32-wasip1`, where a panic aborts rather than unwinds, so a trap
 * ends the instance and its memory. The call that trapped is lost; the caller recovers by creating
 * a fresh host from the same compiled module, which needs no download and no recompilation.</para>
 */
export class NxHostCrashedError extends Error {
  /**
   * Name of the module operation that trapped.
   */
  public readonly operation: string;

  /**
   * Creates a crashed-host error naming the operation that ended the instance.
   */
  public constructor(operation: string, options?: ErrorOptions) {
    super(
      `The NX wasm host crashed during '${operation}' and cannot be reused. ` +
        `Create a new host from the compiled module.`,
      options
    );
    this.name = "NxHostCrashedError";
    this.operation = operation;
  }
}
