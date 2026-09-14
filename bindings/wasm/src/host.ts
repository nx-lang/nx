import type {
  CompletionList,
  DiagnosticReport,
  DocumentSymbol,
  Hover,
  TextPosition
} from "@nx-lang/language-protocol";

import {
  abiVersion,
  resultLengthOffset,
  resultPointerOffset,
  resultStatusOffset,
  statusEvaluationError,
  statusInternalError,
  statusOk,
  type CreateWasiProvider,
  type NxWasmExports
} from "./abi.js";
import {
  NxDisposedResourceError,
  NxEvaluationError,
  NxHostCrashedError,
  NxWasmError
} from "./errors.js";
import type {
  NxDiagnostic,
  NxDiagnosticLabel,
  NxGeneratedNxIr,
  NxLanguageDocumentInput,
  NxSourceBuildOptions,
  NxTextSpan
} from "./types.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/**
 * One instance of the NX WebAssembly module, and the resources built inside it.
 *
 * <para>A host owns a single instance's memory. When a call traps, the instance is gone: the host
 * reports [`NxHostCrashedError`] on that call and on every later one, and the caller creates a
 * replacement from the same compiled module.</para>
 */
export interface NxHost {
  /**
   * Whether a call into this host has trapped. A crashed host answers nothing further.
   */
  readonly crashed: boolean;

  /**
   * Bytes of WebAssembly memory the instance currently holds.
   *
   * <para>The module's allocator reuses freed memory, so this settles once the working set is
   * reached rather than growing with every compile. A caller that keeps one host alive for a long
   * time can watch it to decide when to replace the host.</para>
   */
  readonly memoryBytes: number;

  /**
   * Builds a reusable program artifact from one NX source text.
   *
   * @throws NxEvaluationError when NX analysis reports diagnostics.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  buildProgramArtifact(source: string, options?: NxSourceBuildOptions): NxProgramArtifact;

  /**
   * Analyzes in-memory documents into an immutable snapshot that answers editor queries.
   *
   * @throws NxEvaluationError when a URI is unparseable or two documents share an identity.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  createLanguageSnapshot(documents: Iterable<NxLanguageDocumentInput>): NxLanguageSnapshot;

  /**
   * Abandons the instance and every resource in it.
   *
   * Calling `dispose` more than once is allowed. Operations afterwards throw
   * `NxDisposedResourceError`.
   */
  dispose(): void;
}

/**
 * A reusable analyzed NX program that can emit deterministic NX IR.
 */
export interface NxProgramArtifact {
  /**
   * Generates deterministic NX IR JSON and metadata from this artifact.
   *
   * @throws NxEvaluationError when IR generation reports NX diagnostics.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  generateNxIr(): NxGeneratedNxIr;

  /**
   * Releases the artifact inside the module. Calling `dispose` more than once is allowed.
   */
  dispose(): void;

  [Symbol.dispose](): void;
}

/**
 * An immutable analysis of a document set that answers hover, completions, diagnostics and
 * document symbols.
 *
 * Positions count UTF-16 code units, the way JavaScript strings and browser editors count.
 */
export interface NxLanguageSnapshot {
  /**
   * Hover content at `position` in the document at `uri`, or `null` when there is nothing to say.
   */
  hover(uri: string, position: TextPosition): Hover | null;

  /**
   * Completion candidates at `position` in the document at `uri`.
   */
  completions(uri: string, position: TextPosition): CompletionList;

  /**
   * Diagnostics for every document in the snapshot, plus diagnostics that belong to no document.
   */
  diagnostics(): DiagnosticReport;

  /**
   * Top-level symbols of the document at `uri`.
   */
  documentSymbols(uri: string): DocumentSymbol[];

  /**
   * Releases the snapshot inside the module. Calling `dispose` more than once is allowed.
   */
  dispose(): void;

  [Symbol.dispose](): void;
}

/**
 * Instantiates `module` with the WASI imports `createWasiProvider` supplies and returns a host.
 *
 * <para>Each entry point of the package calls this with its own provider; the ABI version check and
 * everything above it is shared.</para>
 *
 * @throws NxWasmError when the module's ABI version is not the one this loader was written against,
 * or the module does not export what the ABI requires.
 */
export function createHost(
  module: WebAssembly.Module,
  createWasiProvider: CreateWasiProvider
): NxHost {
  const provider = createWasiProvider();
  const instance = new WebAssembly.Instance(module, provider.imports);
  const exports = instance.exports as unknown as NxWasmExports;

  // The ABI is checked before the module's initializer runs, so a module that is not an NX SDK
  // module is refused rather than started.
  if (typeof exports.nx_wasm_abi_version !== "function") {
    throw new NxWasmError(
      "The NX wasm module does not export nx_wasm_abi_version, so it is not an NX SDK module."
    );
  }

  const moduleAbiVersion = exports.nx_wasm_abi_version();
  if (moduleAbiVersion !== abiVersion) {
    throw new NxWasmError(
      `The NX wasm module reports ABI version ${moduleAbiVersion}, but this loader was written ` +
        `against ABI version ${abiVersion}. Rebuild the module and the package together.`
    );
  }

  if (!(exports.memory instanceof WebAssembly.Memory)) {
    throw new NxWasmError("The NX wasm module does not export its memory, so it cannot be used.");
  }

  provider.initialize(instance);
  return new WasmHost(exports);
}

class WasmHost implements NxHost {
  readonly #exports: NxWasmExports;
  #crashedOperation: string | undefined;
  #disposed = false;

  constructor(exports: NxWasmExports) {
    this.#exports = exports;
  }

  get crashed(): boolean {
    return this.#crashedOperation !== undefined;
  }

  get memoryBytes(): number {
    return this.#exports.memory.buffer.byteLength;
  }

  buildProgramArtifact(source: string, options: NxSourceBuildOptions = {}): NxProgramArtifact {
    const handle = this.#handle("nx_wasm_program_build", (argument) =>
      this.#exports.nx_wasm_program_build(argument.pointer, argument.length),
      { source, fileName: options.fileName ?? "input.nx" }
    );
    return new WasmProgramArtifact(this, handle);
  }

  createLanguageSnapshot(documents: Iterable<NxLanguageDocumentInput>): NxLanguageSnapshot {
    const payload = Array.from(documents, (document) => ({
      uri: document.uri,
      source: document.source,
      ...(document.identity === undefined ? {} : { identity: document.identity }),
      ...(document.version === undefined ? {} : { version: document.version })
    }));
    const handle = this.#handle("nx_wasm_snapshot_new", (argument) =>
      this.#exports.nx_wasm_snapshot_new(argument.pointer, argument.length),
      payload
    );
    return new WasmLanguageSnapshot(this, handle);
  }

  dispose(): void {
    this.#disposed = true;
  }

  /**
   * Runs one module operation that takes no JSON argument and parses its payload.
   *
   * @internal
   */
  call<T>(operation: string, run: (exports: NxWasmExports) => number): T {
    const result = this.#enter(operation, () => run(this.#exports));
    return this.#read(operation, result) as T;
  }

  /**
   * Runs one module operation over a JSON argument and parses its payload.
   *
   * @internal
   */
  callWithArgument<T>(
    operation: string,
    run: (exports: NxWasmExports, pointer: number, length: number) => number,
    argument: unknown
  ): T {
    const bytes = encoder.encode(JSON.stringify(argument));
    const pointer = this.#write(operation, bytes);
    try {
      const result = this.#enter(operation, () =>
        run(this.#exports, pointer, bytes.length)
      );
      return this.#read(operation, result) as T;
    } finally {
      this.#release(operation, pointer, bytes.length);
    }
  }

  /**
   * Releases a handle, tolerating a crashed or disposed host so `dispose` never throws.
   *
   * @internal
   */
  free(operation: string, run: (exports: NxWasmExports) => void): void {
    if (this.#crashedOperation !== undefined || this.#disposed) {
      return;
    }

    try {
      run(this.#exports);
    } catch (error) {
      this.#crashedOperation = operation;
      throw new NxHostCrashedError(operation, causeOption(error));
    }
  }

  #handle(
    operation: string,
    run: (argument: { pointer: number; length: number }) => number,
    argument: unknown
  ): number {
    const handle = this.callWithArgument<unknown>(
      operation,
      (_exports, pointer, length) => run({ pointer, length }),
      argument
    );
    if (typeof handle !== "number" || !Number.isInteger(handle) || handle <= 0) {
      throw new NxWasmError(`${operation} answered with something that is not a handle.`);
    }
    return handle;
  }

  #write(operation: string, bytes: Uint8Array): number {
    if (bytes.length === 0) {
      return 0;
    }

    const pointer = this.#enter(operation, () => this.#exports.nx_wasm_alloc(bytes.length));
    if (pointer === 0) {
      throw new NxWasmError(`The NX wasm module could not allocate ${bytes.length} bytes.`);
    }

    new Uint8Array(this.#exports.memory.buffer).set(bytes, pointer);
    return pointer;
  }

  #release(operation: string, pointer: number, length: number): void {
    if (pointer === 0 || this.#crashedOperation !== undefined) {
      return;
    }

    this.#enter(operation, () => {
      this.#exports.nx_wasm_free(pointer, length);
      return 0;
    });
  }

  /**
   * Makes the one call that can trap, and turns a trap into a crashed host.
   *
   * Memory is read only through fresh views afterwards, because a growth during the call detaches
   * every buffer taken before it.
   */
  #enter(operation: string, run: () => number): number {
    if (this.#crashedOperation !== undefined) {
      throw new NxHostCrashedError(this.#crashedOperation);
    }
    if (this.#disposed) {
      throw new NxDisposedResourceError("NxHost");
    }

    try {
      return run();
    } catch (error) {
      this.#crashedOperation = operation;
      throw new NxHostCrashedError(operation, causeOption(error));
    }
  }

  #read(operation: string, result: number): unknown {
    if (result === 0) {
      throw new NxWasmError(`${operation} answered with a null result record.`);
    }

    const view = new DataView(this.#exports.memory.buffer);
    const status = view.getUint32(result + resultStatusOffset, true);
    const pointer = view.getUint32(result + resultPointerOffset, true);
    const length = view.getUint32(result + resultLengthOffset, true);
    const payload =
      pointer === 0 || length === 0
        ? ""
        : decoder.decode(new Uint8Array(this.#exports.memory.buffer, pointer, length).slice());

    this.#enter(operation, () => {
      this.#exports.nx_wasm_result_free(result);
      return 0;
    });

    switch (status) {
      case statusOk:
        return parseJson(operation, payload);
      case statusEvaluationError: {
        const diagnostics = normalizeDiagnostics(parseJson(operation, payload));
        throw new NxEvaluationError(evaluationMessage(diagnostics), diagnostics);
      }
      case statusInternalError:
        throw new NxWasmError(String(parseJson(operation, payload)));
      default:
        throw new NxWasmError(`${operation} answered with unknown status ${status}.`);
    }
  }
}

class WasmProgramArtifact implements NxProgramArtifact {
  readonly #host: WasmHost;
  #handle: number | undefined;

  constructor(host: WasmHost, handle: number) {
    this.#host = host;
    this.#handle = handle;
  }

  generateNxIr(): NxGeneratedNxIr {
    const handle = this.#live();
    return this.#host.call<NxGeneratedNxIr>("nx_wasm_program_nx_ir", (exports) =>
      exports.nx_wasm_program_nx_ir(handle)
    );
  }

  dispose(): void {
    const handle = this.#handle;
    if (handle === undefined) {
      return;
    }

    this.#handle = undefined;
    this.#host.free("nx_wasm_program_free", (exports) => exports.nx_wasm_program_free(handle));
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  #live(): number {
    if (this.#handle === undefined) {
      throw new NxDisposedResourceError("NxProgramArtifact");
    }
    return this.#handle;
  }
}

class WasmLanguageSnapshot implements NxLanguageSnapshot {
  readonly #host: WasmHost;
  #handle: number | undefined;

  constructor(host: WasmHost, handle: number) {
    this.#host = host;
    this.#handle = handle;
  }

  hover(uri: string, position: TextPosition): Hover | null {
    const handle = this.#live();
    return this.#host.callWithArgument<Hover | null>(
      "nx_wasm_snapshot_hover",
      (exports, pointer, length) => exports.nx_wasm_snapshot_hover(handle, pointer, length),
      { uri, line: position.line, character: position.character }
    );
  }

  completions(uri: string, position: TextPosition): CompletionList {
    const handle = this.#live();
    return this.#host.callWithArgument<CompletionList>(
      "nx_wasm_snapshot_completions",
      (exports, pointer, length) => exports.nx_wasm_snapshot_completions(handle, pointer, length),
      { uri, line: position.line, character: position.character }
    );
  }

  diagnostics(): DiagnosticReport {
    const handle = this.#live();
    return this.#host.call<DiagnosticReport>("nx_wasm_snapshot_diagnostics", (exports) =>
      exports.nx_wasm_snapshot_diagnostics(handle)
    );
  }

  documentSymbols(uri: string): DocumentSymbol[] {
    const handle = this.#live();
    return this.#host.callWithArgument<DocumentSymbol[]>(
      "nx_wasm_snapshot_document_symbols",
      (exports, pointer, length) =>
        exports.nx_wasm_snapshot_document_symbols(handle, pointer, length),
      { uri }
    );
  }

  dispose(): void {
    const handle = this.#handle;
    if (handle === undefined) {
      return;
    }

    this.#handle = undefined;
    this.#host.free("nx_wasm_snapshot_free", (exports) => exports.nx_wasm_snapshot_free(handle));
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  #live(): number {
    if (this.#handle === undefined) {
      throw new NxDisposedResourceError("NxLanguageSnapshot");
    }
    return this.#handle;
  }
}

function parseJson(operation: string, payload: string): unknown {
  if (payload === "") {
    return null;
  }

  try {
    return JSON.parse(payload) as unknown;
  } catch (error) {
    throw new NxWasmError(`${operation} answered with malformed JSON.`, causeOption(error));
  }
}

function causeOption(error: unknown): ErrorOptions | undefined {
  return error instanceof Error ? { cause: error } : undefined;
}

function evaluationMessage(diagnostics: readonly NxDiagnostic[]): string {
  const first = diagnostics[0];
  return first === undefined ? "NX evaluation failed." : first.message;
}

function normalizeDiagnostics(raw: unknown): readonly NxDiagnostic[] {
  if (!Array.isArray(raw)) {
    throw new NxWasmError("The NX wasm module returned diagnostics in an unexpected shape.");
  }

  return raw.map(normalizeDiagnostic);
}

function normalizeDiagnostic(raw: unknown): NxDiagnostic {
  const value = asRecord(raw, "diagnostic");
  return {
    severity: normalizeSeverity(value["severity"]),
    message: typeof value["message"] === "string" ? value["message"] : "",
    labels: Array.isArray(value["labels"]) ? value["labels"].map(normalizeDiagnosticLabel) : [],
    ...(typeof value["code"] === "string" ? { code: value["code"] } : {}),
    ...(typeof value["help"] === "string" ? { help: value["help"] } : {}),
    ...(typeof value["note"] === "string" ? { note: value["note"] } : {})
  };
}

function normalizeDiagnosticLabel(raw: unknown): NxDiagnosticLabel {
  const value = asRecord(raw, "diagnostic label");
  return {
    file: typeof value["file"] === "string" ? value["file"] : "",
    span: normalizeTextSpan(value["span"]),
    primary: value["primary"] === true,
    ...(typeof value["message"] === "string" ? { message: value["message"] } : {})
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

function normalizeSeverity(value: unknown): NxDiagnostic["severity"] {
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

  throw new NxWasmError(`The NX wasm module returned an invalid ${name}.`);
}
