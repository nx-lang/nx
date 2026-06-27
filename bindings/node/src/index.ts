import { NxDisposedResourceError, NxEvaluationError, NxNativeError } from "./errors.js";
import {
  loadNativeBinding,
  type NativeNxLibraryRegistry,
  type NativeNxProgramArtifact,
  type NativeNxProgramBuildContext,
  type NativeNxWorkspace,
  type NativeWorkspaceModule
} from "./native.js";
import type {
  NxByteEvaluationOptions,
  NxDiagnostic,
  NxDiagnosticLabel,
  NxEvaluationOptions,
  NxGeneratedNxIr,
  NxJsonValue,
  NxOutputFormat,
  NxSourceByteEvaluationOptions,
  NxSourceBuildOptions,
  NxSourceEvaluationOptions,
  NxSourceInput,
  NxTextSpan,
  NxWorkspaceBuildOptions,
  NxWorkspaceModuleInput
} from "./types.js";

export {
  NxDisposedResourceError,
  NxEvaluationError,
  NxNativeError
};
export type {
  NxByteEvaluationOptions,
  NxDiagnostic,
  NxDiagnosticLabel,
  NxEvaluationOptions,
  NxGeneratedNxIr,
  NxIrEntrypointMetadata,
  NxIrMetadata,
  NxIrReferenceMetadata,
  NxJsonValue,
  NxOutputFormat,
  NxSeverity,
  NxSourceByteEvaluationOptions,
  NxSourceBuildOptions,
  NxSourceEvaluationOptions,
  NxSourceInput,
  NxTextSpan,
  NxWorkspaceBuildOptions,
  NxWorkspaceModuleInput
} from "./types.js";

const workspaceNatives = new WeakMap<NxWorkspace, NativeNxWorkspace>();
const registryNatives = new WeakMap<NxLibraryRegistry, NativeNxLibraryRegistry>();
const buildContextNatives = new WeakMap<NxProgramBuildContext, NativeNxProgramBuildContext>();
const artifactNatives = new WeakMap<NxProgramArtifact, NativeNxProgramArtifact>();

const evaluationPrefix = "NX_EVALUATION:";
const nativePrefix = "NX_NATIVE:";
const disposedPrefix = "NX_DISPOSED:";

/**
 * In-memory NX workspace made from source modules with stable logical identities.
 *
 * Workspace modules are submitted directly to the native host; ordinary workspace builds do not require temp files.
 */
export class NxWorkspace {
  /**
   * Creates a workspace from source-backed modules.
   *
   * @throws NxEvaluationError when native workspace creation reports NX diagnostics, such as duplicate normalized
   * module identities.
   * @throws NxNativeError when the native binding cannot create the workspace.
   */
  public constructor(modules: Iterable<NxWorkspaceModuleInput>) {
    const nativeModules = Array.from(modules, normalizeWorkspaceModule);
    const native = invokeNative(() => new (loadNativeBinding().NativeNxWorkspace)(nativeModules));
    workspaceNatives.set(this, native);
  }

  /**
   * Validates this workspace against a supplied build context and returns all NX diagnostics as data.
   *
   * @throws NxDisposedResourceError when this workspace or the build context has already been disposed.
   * @throws NxNativeError when the native binding cannot run validation.
   */
  public validate(buildContext: NxProgramBuildContext): readonly NxDiagnostic[] {
    const diagnosticsJson = invokeNative(() =>
      getWorkspaceNative(this).validate(getBuildContextNative(buildContext))
    );
    return parseDiagnosticsJson(diagnosticsJson);
  }

  /**
   * Releases the native workspace resource.
   *
   * Calling `dispose` more than once is allowed. Operations after disposal throw `NxDisposedResourceError`.
   */
  public dispose(): void {
    const native = workspaceNatives.get(this);
    if (native !== undefined) {
      invokeNative(() => native.dispose());
      workspaceNatives.delete(this);
    }
  }

  /**
   * Releases the native workspace resource through JavaScript explicit resource management.
   */
  public [Symbol.dispose](): void {
    this.dispose();
  }
}

/**
 * Reusable registry of analyzed NX libraries that can create build contexts for compilation and evaluation.
 */
export class NxLibraryRegistry {
  /**
   * Creates an empty library registry.
   *
   * @throws NxNativeError when the native binding cannot create the registry.
   */
  public constructor() {
    const native = invokeNative(() => new (loadNativeBinding().NativeNxLibraryRegistry)());
    registryNatives.set(this, native);
  }

  /**
   * Loads an NX library root from disk into this registry.
   *
   * @throws NxEvaluationError when the library reports NX diagnostics.
   * @throws NxDisposedResourceError when this registry has already been disposed.
   * @throws NxNativeError when the native binding cannot load the library.
   */
  public loadFromDirectory(rootPath: string): void {
    invokeNative(() => getRegistryNative(this).loadLibraryFromDirectory(rootPath));
  }

  /**
   * Creates a program build context backed by the current registry contents.
   *
   * The returned build context owns a separate native resource and should be disposed by the caller.
   *
   * @throws NxDisposedResourceError when this registry has already been disposed.
   * @throws NxNativeError when the native binding cannot create the context.
   */
  public createBuildContext(): NxProgramBuildContext {
    const native = invokeNative(() => getRegistryNative(this).createBuildContext());
    return NxProgramBuildContext.fromNative(native);
  }

  /**
   * Releases the native registry resource.
   *
   * Calling `dispose` more than once is allowed. Operations after disposal throw `NxDisposedResourceError`.
   */
  public dispose(): void {
    const native = registryNatives.get(this);
    if (native !== undefined) {
      invokeNative(() => native.dispose());
      registryNatives.delete(this);
    }
  }

  /**
   * Releases the native registry resource through JavaScript explicit resource management.
   */
  public [Symbol.dispose](): void {
    this.dispose();
  }
}

/**
 * Registry-backed build scope used to validate workspaces and create reusable program artifacts.
 */
export class NxProgramBuildContext {
  private constructor(native: NativeNxProgramBuildContext) {
    buildContextNatives.set(this, native);
  }

  /**
   * Creates a build context from an existing registry or from a temporary empty registry.
   *
   * @throws NxDisposedResourceError when the supplied registry has already been disposed.
   * @throws NxNativeError when the native binding cannot create the context.
   */
  public static create(registry?: NxLibraryRegistry): NxProgramBuildContext {
    if (registry !== undefined) {
      return registry.createBuildContext();
    }

    const temporaryRegistry = new NxLibraryRegistry();
    try {
      return temporaryRegistry.createBuildContext();
    } finally {
      temporaryRegistry.dispose();
    }
  }

  /**
   * Creates a build context backed by the supplied library registry.
   *
   * @throws NxDisposedResourceError when the registry has already been disposed.
   * @throws NxNativeError when the native binding cannot create the context.
   */
  public static fromRegistry(registry: NxLibraryRegistry): NxProgramBuildContext {
    return registry.createBuildContext();
  }

  /**
   * Releases the native build-context resource.
   *
   * Program artifacts created from this context remain usable after the context is disposed. Calling `dispose` more
   * than once is allowed.
   */
  public dispose(): void {
    const native = buildContextNatives.get(this);
    if (native !== undefined) {
      invokeNative(() => native.dispose());
      buildContextNatives.delete(this);
    }
  }

  /**
   * Releases the native build-context resource through JavaScript explicit resource management.
   */
  public [Symbol.dispose](): void {
    this.dispose();
  }

  /**
   * Wraps a native build-context handle returned by the SDK binding.
   *
   * @internal
   */
  public static fromNative(native: NativeNxProgramBuildContext): NxProgramBuildContext {
    return new NxProgramBuildContext(native);
  }
}

/**
 * Reusable analyzed NX program artifact.
 *
 * Program artifacts can generate deterministic NX IR and evaluate supported entrypoints after their originating build
 * context has been disposed.
 */
export class NxProgramArtifact {
  private constructor(native: NativeNxProgramArtifact) {
    artifactNatives.set(this, native);
  }

  /**
   * Builds a reusable program artifact from a single NX source payload.
   *
   * @throws NxEvaluationError when NX analysis fails and reports diagnostics.
   * @throws NxDisposedResourceError when the supplied build context has already been disposed.
   * @throws NxNativeError when the native binding cannot build the artifact.
   */
  public static buildSource(source: NxSourceInput, options: NxSourceBuildOptions = {}): NxProgramArtifact {
    return withBuildContext(options.buildContext, (buildContext) => {
      const native = invokeNative(() =>
        getBuildContextNative(buildContext).buildSourceProgramArtifact(
          normalizeSourceInput(source),
          options.fileName
        )
      );
      return new NxProgramArtifact(native);
    });
  }

  /**
   * Builds a reusable program artifact from an in-memory workspace and explicit entry identity.
   *
   * @throws NxEvaluationError when NX analysis fails, including when the entry identity is missing.
   * @throws NxDisposedResourceError when the workspace or build context has already been disposed.
   * @throws NxNativeError when the native binding cannot build the artifact.
   */
  public static buildWorkspace(workspace: NxWorkspace, options: NxWorkspaceBuildOptions): NxProgramArtifact {
    const native = invokeNative(() =>
      getBuildContextNative(options.buildContext).buildWorkspaceProgramArtifact(
        getWorkspaceNative(workspace),
        options.entryIdentity
      )
    );
    return new NxProgramArtifact(native);
  }

  /**
   * Logical file or workspace identity selected when this artifact was built.
   */
  public get entryIdentity(): string {
    return invokeNative(() => getArtifactNative(this).entryIdentity);
  }

  /**
   * Generates deterministic NX IR JSON and metadata from this artifact.
   *
   * @throws NxEvaluationError when IR generation reports NX diagnostics.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxNativeError when the native binding returns an invalid or unexpected payload.
   */
  public generateNxIr(): NxGeneratedNxIr {
    return parseJson<NxGeneratedNxIr>(invokeNative(() => getArtifactNative(this).generateNxIr()));
  }

  /**
   * Evaluates the supported `root()` entrypoint and returns a JSON-compatible value.
   *
   * @throws NxEvaluationError when evaluation reports NX diagnostics or a non-`root` entrypoint is requested.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxNativeError when the native binding returns an invalid or unexpected payload.
   */
  public evaluateJson(options: NxEvaluationOptions = {}): NxJsonValue {
    assertSupportedRootEntrypoint(options);
    return parseJson<NxJsonValue>(invokeNative(() => getArtifactNative(this).evaluateJson()));
  }

  /**
   * Evaluates the supported `root()` entrypoint and returns bytes in the requested output format.
   *
   * @throws NxEvaluationError when evaluation reports NX diagnostics or a non-`root` entrypoint is requested.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxNativeError when the native binding cannot evaluate the artifact.
   */
  public evaluateBytes(options: NxByteEvaluationOptions = {}): Buffer {
    assertSupportedRootEntrypoint(options);
    return invokeNative(() => getArtifactNative(this).evaluateBytes(options.outputFormat ?? "messagePack"));
  }

  /**
   * Releases the native program-artifact resource.
   *
   * Calling `dispose` more than once is allowed. Operations after disposal throw `NxDisposedResourceError`.
   */
  public dispose(): void {
    const native = artifactNatives.get(this);
    if (native !== undefined) {
      invokeNative(() => native.dispose());
      artifactNatives.delete(this);
    }
  }

  /**
   * Releases the native program-artifact resource through JavaScript explicit resource management.
   */
  public [Symbol.dispose](): void {
    this.dispose();
  }
}

/**
 * Validates an in-memory workspace against a supplied build context and returns all NX diagnostics as data.
 */
export function validateWorkspace(
  workspace: NxWorkspace,
  buildContext: NxProgramBuildContext
): readonly NxDiagnostic[] {
  return workspace.validate(buildContext);
}

/**
 * Builds a reusable program artifact from a single NX source payload.
 *
 * @throws NxEvaluationError when NX analysis reports diagnostics.
 */
export function buildProgramArtifactFromSource(
  source: NxSourceInput,
  options: NxSourceBuildOptions = {}
): NxProgramArtifact {
  return NxProgramArtifact.buildSource(source, options);
}

/**
 * Builds a reusable program artifact from an in-memory workspace and explicit entry identity.
 *
 * @throws NxEvaluationError when NX analysis reports diagnostics.
 */
export function buildProgramArtifactFromWorkspace(
  workspace: NxWorkspace,
  options: NxWorkspaceBuildOptions
): NxProgramArtifact {
  return NxProgramArtifact.buildWorkspace(workspace, options);
}

/**
 * Builds a short-lived source artifact, generates deterministic NX IR JSON and metadata, then disposes the artifact.
 *
 * @throws NxEvaluationError when build or IR generation reports diagnostics.
 */
export function generateNxIrFromSource(
  source: NxSourceInput,
  options: NxSourceBuildOptions = {}
): NxGeneratedNxIr {
  const artifact = NxProgramArtifact.buildSource(source, options);
  try {
    return artifact.generateNxIr();
  } finally {
    artifact.dispose();
  }
}

/**
 * Builds a short-lived source artifact, evaluates `root()`, then disposes the artifact.
 *
 * @throws NxEvaluationError when build or evaluation reports diagnostics, or when a non-`root` entrypoint is requested.
 */
export function evaluateJsonFromSource(
  source: NxSourceInput,
  options: NxSourceEvaluationOptions = {}
): NxJsonValue {
  assertSupportedRootEntrypoint(options);
  const artifact = NxProgramArtifact.buildSource(source, options);
  try {
    return artifact.evaluateJson(options);
  } finally {
    artifact.dispose();
  }
}

/**
 * Builds a short-lived source artifact, evaluates `root()` to bytes, then disposes the artifact.
 *
 * @throws NxEvaluationError when build or evaluation reports diagnostics, or when a non-`root` entrypoint is requested.
 */
export function evaluateBytesFromSource(
  source: NxSourceInput,
  options: NxSourceByteEvaluationOptions = {}
): Buffer {
  assertSupportedRootEntrypoint(options);
  const artifact = NxProgramArtifact.buildSource(source, options);
  try {
    return artifact.evaluateBytes(options);
  } finally {
    artifact.dispose();
  }
}

function withBuildContext<T>(
  buildContext: NxProgramBuildContext | undefined,
  callback: (buildContext: NxProgramBuildContext) => T
): T {
  if (buildContext !== undefined) {
    return callback(buildContext);
  }

  const ownedBuildContext = NxProgramBuildContext.create();
  try {
    return callback(ownedBuildContext);
  } finally {
    ownedBuildContext.dispose();
  }
}

function normalizeWorkspaceModule(module: NxWorkspaceModuleInput): NativeWorkspaceModule {
  return {
    identity: module.identity,
    source: normalizeSourceInput(module.source)
  };
}

function normalizeSourceInput(source: NxSourceInput): string | Buffer {
  if (typeof source === "string") {
    return source;
  }

  if (Buffer.isBuffer(source)) {
    return source;
  }

  return Buffer.from(source.buffer, source.byteOffset, source.byteLength);
}

function getWorkspaceNative(workspace: NxWorkspace): NativeNxWorkspace {
  const native = workspaceNatives.get(workspace);
  if (native === undefined) {
    throw new NxDisposedResourceError("NxWorkspace");
  }
  return native;
}

function getRegistryNative(registry: NxLibraryRegistry): NativeNxLibraryRegistry {
  const native = registryNatives.get(registry);
  if (native === undefined) {
    throw new NxDisposedResourceError("NxLibraryRegistry");
  }
  return native;
}

function getBuildContextNative(buildContext: NxProgramBuildContext): NativeNxProgramBuildContext {
  const native = buildContextNatives.get(buildContext);
  if (native === undefined) {
    throw new NxDisposedResourceError("NxProgramBuildContext");
  }
  return native;
}

function getArtifactNative(artifact: NxProgramArtifact): NativeNxProgramArtifact {
  const native = artifactNatives.get(artifact);
  if (native === undefined) {
    throw new NxDisposedResourceError("NxProgramArtifact");
  }
  return native;
}

function invokeNative<T>(callback: () => T): T {
  try {
    return callback();
  } catch (error) {
    throw normalizeNativeError(error);
  }
}

function normalizeNativeError(error: unknown): Error {
  if (
    error instanceof NxEvaluationError ||
    error instanceof NxNativeError ||
    error instanceof NxDisposedResourceError
  ) {
    return error;
  }

  const message = error instanceof Error ? error.message : String(error);
  const evaluationPayload = payloadAfterPrefix(message, evaluationPrefix);
  if (evaluationPayload !== undefined) {
    const diagnostics = parseDiagnosticsJson(evaluationPayload);
    return new NxEvaluationError(evaluationMessage(diagnostics), diagnostics, causeOption(error));
  }

  const nativePayload = payloadAfterPrefix(message, nativePrefix);
  if (nativePayload !== undefined) {
    return new NxNativeError(nativePayload, causeOption(error));
  }

  const disposedPayload = payloadAfterPrefix(message, disposedPrefix);
  if (disposedPayload !== undefined) {
    return new NxDisposedResourceError(disposedPayload, causeOption(error));
  }

  return new NxNativeError(`NX native binding failed: ${message}`, causeOption(error));
}

function payloadAfterPrefix(message: string, prefix: string): string | undefined {
  const index = message.indexOf(prefix);
  if (index < 0) {
    return undefined;
  }

  return message.slice(index + prefix.length);
}

function causeOption(error: unknown): ErrorOptions | undefined {
  return error instanceof Error ? { cause: error } : undefined;
}

function evaluationMessage(diagnostics: readonly NxDiagnostic[]): string {
  const first = diagnostics[0];
  return first === undefined ? "NX evaluation failed." : first.message;
}

function assertSupportedRootEntrypoint(options: NxEvaluationOptions): void {
  const entrypoint = options.entrypoint ?? "root";
  if (entrypoint === "root") {
    return;
  }

  throw new NxEvaluationError(
    `Named entrypoint evaluation is not supported by the native Node SDK: ${entrypoint}`,
    [
      {
        severity: "error",
        code: "unsupported-entrypoint",
        message: `Named entrypoint evaluation is not supported by the current native host API: ${entrypoint}`,
        labels: [],
        help: "Expose the entrypoint as root() or add named-entrypoint support to the Rust host API first."
      }
    ]
  );
}

function parseJson<T>(json: string): T {
  try {
    return JSON.parse(json) as T;
  } catch (error) {
    throw new NxNativeError("NX native binding returned invalid JSON.", causeOption(error));
  }
}

function parseDiagnosticsJson(json: string): readonly NxDiagnostic[] {
  const rawDiagnostics = parseJson<unknown>(json);
  if (!Array.isArray(rawDiagnostics)) {
    throw new NxNativeError("NX native binding returned diagnostics in an unexpected shape.");
  }

  return rawDiagnostics.map(normalizeDiagnostic);
}

function normalizeDiagnostic(raw: unknown): NxDiagnostic {
  const value = asRecord(raw, "diagnostic");
  return {
    severity: normalizeSeverity(value.severity),
    message: typeof value.message === "string" ? value.message : "",
    labels: Array.isArray(value.labels) ? value.labels.map(normalizeDiagnosticLabel) : [],
    ...(typeof value.code === "string" ? { code: value.code } : {}),
    ...(typeof value.help === "string" ? { help: value.help } : {}),
    ...(typeof value.note === "string" ? { note: value.note } : {})
  };
}

function normalizeDiagnosticLabel(raw: unknown): NxDiagnosticLabel {
  const value = asRecord(raw, "diagnostic label");
  return {
    file: typeof value.file === "string" ? value.file : "",
    span: normalizeTextSpan(value.span),
    primary: value.primary === true,
    ...(typeof value.message === "string" ? { message: value.message } : {})
  };
}

function normalizeTextSpan(raw: unknown): NxTextSpan {
  const value = asRecord(raw, "text span");
  return {
    startByte: numericField(value, "start_byte", "startByte"),
    endByte: numericField(value, "end_byte", "endByte"),
    startLine: numericField(value, "start_line", "startLine"),
    startColumn: numericField(value, "start_column", "startColumn"),
    endLine: numericField(value, "end_line", "endLine"),
    endColumn: numericField(value, "end_column", "endColumn")
  };
}

function normalizeSeverity(value: unknown): "error" | "warning" | "info" | "hint" {
  return value === "warning" || value === "info" || value === "hint" ? value : "error";
}

function numericField(value: Record<string, unknown>, snake: string, camel: string): number {
  const field = value[snake] ?? value[camel];
  return typeof field === "number" ? field : 0;
}

function asRecord(value: unknown, name: string): Record<string, unknown> {
  if (typeof value === "object" && value !== null) {
    return value as Record<string, unknown>;
  }

  throw new NxNativeError(`NX native binding returned an invalid ${name}.`);
}
