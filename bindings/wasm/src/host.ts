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
  NxBuildContextOptions,
  NxDiagnostic,
  NxDiagnosticLabel,
  NxGeneratedNxIr,
  NxIrEmitOptions,
  NxIrMetadata,
  NxLanguageDocumentInput,
  NxLanguageSnapshotOptions,
  NxLibraryInput,
  NxSourceBuildOptions,
  NxTextSpan,
  NxValueNode,
  NxValueRole,
  NxValueText,
  NxWorkspaceBuildOptions,
  NxWorkspaceValidateOptions
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
   * Builds a reusable program artifact from one NX source text: a workspace of one module.
   *
   * @throws NxEvaluationError when NX analysis reports diagnostics.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  buildProgramArtifact(source: string, options?: NxSourceBuildOptions): NxProgramArtifact;

  /**
   * Builds a reusable program artifact from a workspace of in-memory modules and an entry.
   *
   * @throws NxEvaluationError when NX analysis reports diagnostics, each against the identity of
   * the module it belongs to.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  buildWorkspaceArtifact(options: NxWorkspaceBuildOptions): NxProgramArtifact;

  /**
   * Analyzes a workspace of in-memory modules without building it, and answers with every
   * diagnostic — errors, warnings, info and hints — each against the identity of the module it
   * belongs to. NX diagnostics are the answer rather than an error, including a workspace the SDK
   * cannot form, such as one with two modules of the same identity.
   *
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  validateWorkspace(options: NxWorkspaceValidateOptions): readonly NxDiagnostic[];

  /**
   * Creates an empty library registry in this host. Libraries loaded into it are analyzed once and
   * reused by every build and validation in a build context created from it.
   *
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  createLibraryRegistry(): NxLibraryRegistry;

  /**
   * Analyzes in-memory documents into an immutable snapshot that answers editor queries.
   *
   * @throws NxEvaluationError when a URI is unparseable or two documents share an identity.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  createLanguageSnapshot(
    documents: Iterable<NxLanguageDocumentInput>,
    options?: NxLanguageSnapshotOptions
  ): NxLanguageSnapshot;

  /**
   * Abandons the instance and every resource in it.
   *
   * Calling `dispose` more than once is allowed. Operations afterwards throw
   * `NxDisposedResourceError`.
   */
  dispose(): void;

  /**
   * Renders an NX IR image as text with every table index resolved: the same text
   * `nxlang ir explain` prints.
   *
   * @throws NxEvaluationError when the bytes are not an NX IR image this build reads, with a
   * diagnostic saying why.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  explainNxIr(image: Uint8Array): string;
}

/**
 * Libraries loaded from memory into one host, analyzed once and shared by every build context
 * created from the registry.
 *
 * <para>A library root is immutable for the registry's life: loading it again with the same
 * modules and version does nothing, and loading it with different ones fails. A host that needs to
 * reclaim the memory its libraries hold is replaced, not emptied.</para>
 */
export interface NxLibraryRegistry {
  /**
   * Loads one library. Every library it imports must already be loaded.
   *
   * @returns The library's own warnings, info and hints. Workspace validation and builds against
   * the library leave them out, since they are about text a workspace's author cannot edit, so
   * this is where a host reads them.
   * @throws NxEvaluationError when the library's analysis reports errors, a library it imports is
   * not loaded, or its root is already loaded with other modules or another version. A library that
   * fails to load is not retained.
   * @throws NxDisposedResourceError when this registry has been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  loadLibrary(library: NxLibraryInput): readonly NxDiagnostic[];

  /**
   * Loads several libraries in dependency order, whatever order they are given in. A library a
   * library of the list imports must be in the list or already loaded.
   *
   * @returns Each listed library's own warnings, info and hints, in the order the libraries were
   * given; see {@link loadLibrary}.
   * @throws NxEvaluationError as {@link loadLibrary} does, or when the libraries import each other
   * in a cycle. Libraries of the list loaded before the failing one stay loaded.
   * @throws NxDisposedResourceError when this registry has been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  loadLibraries(libraries: Iterable<NxLibraryInput>): readonly NxDiagnostic[];

  /**
   * Creates a build context that sees this registry's loaded libraries, all of them unless
   * `options.visibleRoots` names some.
   *
   * <para>The context holds the libraries it sees, so it stays usable after the registry is
   * disposed.</para>
   *
   * @throws NxEvaluationError when a visible root is not a loaded library.
   * @throws NxDisposedResourceError when this registry has been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  createBuildContext(options?: NxBuildContextOptions): NxProgramBuildContext;

  /**
   * Releases the registry inside the module. Calling `dispose` more than once is allowed.
   */
  dispose(): void;

  [Symbol.dispose](): void;
}

/**
 * The libraries, and the implicit imports, a workspace is built or validated against. Pass it as
 * `buildContext` to {@link NxHost.buildWorkspaceArtifact} or {@link NxHost.validateWorkspace} on
 * the host that created it.
 */
export interface NxProgramBuildContext {
  /**
   * Releases the build context inside the module. Calling `dispose` more than once is allowed.
   */
  dispose(): void;

  [Symbol.dispose](): void;
}

/**
 * A reusable analyzed NX program that can emit deterministic NX IR.
 */
export interface NxProgramArtifact {
  /**
   * Emits NX IR for the modules `options` names, the entry module alone by default: one image per
   * module with its metadata, without a debug section unless asked.
   *
   * @throws NxEvaluationError when IR generation reports NX diagnostics.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  generateNxIr(options?: NxIrEmitOptions): readonly NxGeneratedNxIr[];

  /**
   * Evaluates the entry module's `root` and spells the value as NX text, the text `nxlang run`
   * prints, with nodes that say what each part of it is.
   *
   * <para>The artifact stays usable afterwards, whether the evaluation succeeds or not.</para>
   *
   * @throws NxEvaluationError when there is no `root`, evaluating it fails at run time (with the
   * failing expression's span when it is in the entry module), or the value has no NX spelling
   * (code `nx-text-unspellable`), as a value holding an action handler has none.
   * @throws NxDisposedResourceError when this artifact has already been disposed.
   * @throws NxHostCrashedError when the module traps, or has already trapped.
   */
  evaluateNx(): NxValueText;

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

  buildWorkspaceArtifact(options: NxWorkspaceBuildOptions): NxProgramArtifact {
    const context = this.#contextHandle(options.buildContext);
    const handle = this.#handle("nx_wasm_workspace_build", (argument) =>
      this.#exports.nx_wasm_workspace_build(context, argument.pointer, argument.length),
      { ...workspacePayload(options), entry: options.entry }
    );
    return new WasmProgramArtifact(this, handle);
  }

  validateWorkspace(options: NxWorkspaceValidateOptions): readonly NxDiagnostic[] {
    const context = this.#contextHandle(options.buildContext);
    const raw = this.callWithArgument<unknown>(
      "nx_wasm_workspace_validate",
      (exports, pointer, length) => exports.nx_wasm_workspace_validate(context, pointer, length),
      workspacePayload(options)
    );
    return normalizeDiagnostics(raw);
  }

  createLibraryRegistry(): NxLibraryRegistry {
    const operation = "nx_wasm_registry_new";
    const handle = asHandle(
      operation,
      this.call<unknown>(operation, (exports) => exports.nx_wasm_registry_new())
    );
    return new WasmLibraryRegistry(this, handle);
  }

  /**
   * The module handle of a build context this host created, or `0` for none.
   */
  #contextHandle(context: NxProgramBuildContext | undefined): number {
    if (context === undefined) {
      return 0;
    }
    if (!(context instanceof WasmProgramBuildContext) || !context.belongsTo(this)) {
      throw new NxWasmError("The build context was not created by this host.");
    }
    return context.live();
  }

  createLanguageSnapshot(
    documents: Iterable<NxLanguageDocumentInput>,
    options: NxLanguageSnapshotOptions = {}
  ): NxLanguageSnapshot {
    const payload = {
      documents: Array.from(documents, (document) => ({
        uri: document.uri,
        source: document.source,
        ...(document.identity === undefined ? {} : { identity: document.identity }),
        ...(document.version === undefined ? {} : { version: document.version })
      })),
      implicitImports: Array.from(options.implicitImports ?? [])
    };
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
    return parseJson(operation, decoder.decode(this.#read(operation, result))) as T;
  }

  /**
   * Runs one module operation over a JSON argument and parses its payload.
   *
   * @internal
   */
  explainNxIr(image: Uint8Array): string {
    return this.callWithBytes<string>(
      "nx_wasm_ir_explain",
      (exports, pointer, length) => exports.nx_wasm_ir_explain(pointer, length),
      image
    );
  }

  callWithArgument<T>(
    operation: string,
    run: (exports: NxWasmExports, pointer: number, length: number) => number,
    argument: unknown
  ): T {
    return this.callWithBytes<T>(operation, run, encoder.encode(JSON.stringify(argument)));
  }

  /**
   * Makes one call whose argument is raw bytes and whose payload is JSON.
   */
  callWithBytes<T>(
    operation: string,
    run: (exports: NxWasmExports, pointer: number, length: number) => number,
    bytes: Uint8Array
  ): T {
    return parseJson(operation, decoder.decode(this.callForBytes(operation, run, bytes))) as T;
  }

  /**
   * Makes one call whose argument is raw bytes and whose payload is bytes the export documents,
   * copied out of the module's memory before the result is released.
   */
  callForBytes(
    operation: string,
    run: (exports: NxWasmExports, pointer: number, length: number) => number,
    bytes: Uint8Array
  ): Uint8Array {
    const pointer = this.#write(operation, bytes);
    try {
      const result = this.#enter(operation, () =>
        run(this.#exports, pointer, bytes.length)
      );
      return this.#read(operation, result);
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
    return asHandle(
      operation,
      this.callWithArgument<unknown>(
        operation,
        (_exports, pointer, length) => run({ pointer, length }),
        argument
      )
    );
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

  /**
   * Reads a result record's payload, copied out of the module's memory, and releases the record.
   * A success answers with the bytes; a failure throws the error the status names.
   */
  #read(operation: string, result: number): Uint8Array {
    if (result === 0) {
      throw new NxWasmError(`${operation} answered with a null result record.`);
    }

    const view = new DataView(this.#exports.memory.buffer);
    const status = view.getUint32(result + resultStatusOffset, true);
    const pointer = view.getUint32(result + resultPointerOffset, true);
    const length = view.getUint32(result + resultLengthOffset, true);
    const payload =
      pointer === 0 || length === 0
        ? new Uint8Array(0)
        : new Uint8Array(this.#exports.memory.buffer, pointer, length).slice();

    this.#enter(operation, () => {
      this.#exports.nx_wasm_result_free(result);
      return 0;
    });

    switch (status) {
      case statusOk:
        return payload;
      case statusEvaluationError: {
        const diagnostics = normalizeDiagnostics(parseJson(operation, decoder.decode(payload)));
        throw new NxEvaluationError(evaluationMessage(diagnostics), diagnostics);
      }
      case statusInternalError:
        throw new NxWasmError(String(parseJson(operation, decoder.decode(payload))));
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

  generateNxIr(options: NxIrEmitOptions = {}): readonly NxGeneratedNxIr[] {
    const handle = this.#live();
    const operation = "nx_wasm_program_nx_ir";
    const bundle = this.#host.callForBytes(
      operation,
      (exports, pointer, length) => exports.nx_wasm_program_nx_ir(handle, pointer, length),
      encoder.encode(
        JSON.stringify({
          ...(options.modules === undefined ? {} : { modules: Array.from(options.modules) }),
          debug: options.debug === true
        })
      )
    );
    return readNxIrBundle(operation, bundle);
  }

  evaluateNx(): NxValueText {
    const handle = this.#live();
    const raw = this.#host.call<unknown>("nx_wasm_program_evaluate_nx", (exports) =>
      exports.nx_wasm_program_evaluate_nx(handle)
    );
    return normalizeValueText(raw);
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

class WasmLibraryRegistry implements NxLibraryRegistry {
  readonly #host: WasmHost;
  #handle: number | undefined;

  constructor(host: WasmHost, handle: number) {
    this.#host = host;
    this.#handle = handle;
  }

  loadLibrary(library: NxLibraryInput): readonly NxDiagnostic[] {
    return this.loadLibraries([library]);
  }

  loadLibraries(libraries: Iterable<NxLibraryInput>): readonly NxDiagnostic[] {
    const handle = this.#live();
    const raw = this.#host.callWithArgument<unknown>(
      "nx_wasm_registry_load",
      (exports, pointer, length) => exports.nx_wasm_registry_load(handle, pointer, length),
      {
        libraries: Array.from(libraries, (library) => ({
          root: library.root,
          ...(library.version === undefined ? {} : { version: library.version }),
          modules: library.modules.map((module) => ({
            identity: module.identity,
            source: module.source
          }))
        }))
      }
    );
    return normalizeDiagnostics(raw);
  }

  createBuildContext(options: NxBuildContextOptions = {}): NxProgramBuildContext {
    const handle = this.#live();
    const operation = "nx_wasm_build_context_new";
    const context = asHandle(
      operation,
      this.#host.callWithArgument<unknown>(
        operation,
        (exports, pointer, length) => exports.nx_wasm_build_context_new(handle, pointer, length),
        {
          ...(options.visibleRoots === undefined
            ? {}
            : { visibleRoots: Array.from(options.visibleRoots) }),
          implicitImports: Array.from(options.implicitImports ?? [])
        }
      )
    );
    return new WasmProgramBuildContext(this.#host, context);
  }

  dispose(): void {
    const handle = this.#handle;
    if (handle === undefined) {
      return;
    }

    this.#handle = undefined;
    this.#host.free("nx_wasm_registry_free", (exports) => exports.nx_wasm_registry_free(handle));
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  #live(): number {
    if (this.#handle === undefined) {
      throw new NxDisposedResourceError("NxLibraryRegistry");
    }
    return this.#handle;
  }
}

class WasmProgramBuildContext implements NxProgramBuildContext {
  readonly #host: WasmHost;
  #handle: number | undefined;

  constructor(host: WasmHost, handle: number) {
    this.#host = host;
    this.#handle = handle;
  }

  /**
   * Whether `host` created this context: a handle means nothing in another instance's memory.
   *
   * @internal
   */
  belongsTo(host: WasmHost): boolean {
    return this.#host === host;
  }

  /**
   * @internal
   */
  live(): number {
    if (this.#handle === undefined) {
      throw new NxDisposedResourceError("NxProgramBuildContext");
    }
    return this.#handle;
  }

  dispose(): void {
    const handle = this.#handle;
    if (handle === undefined) {
      return;
    }

    this.#handle = undefined;
    this.#host.free("nx_wasm_build_context_free", (exports) =>
      exports.nx_wasm_build_context_free(handle)
    );
  }

  [Symbol.dispose](): void {
    this.dispose();
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

/**
 * Splits an NX IR bundle into its artifacts.
 *
 * The bundle is what the module answers `nx_wasm_program_nx_ir` with: a little-endian `u32` header
 * length, a JSON header `[{ identity, metadata, offset, length }]`, padding to four bytes, then the
 * images at the offsets the header gives, measured from the start of the bundle. Each image is
 * sliced out as its own copy so a caller holds nothing but its artifact.
 */
function readNxIrBundle(operation: string, bundle: Uint8Array): readonly NxGeneratedNxIr[] {
  if (bundle.byteLength < 4) {
    throw new NxWasmError(`${operation} answered with a bundle that has no header.`);
  }
  const headerLength = new DataView(bundle.buffer, bundle.byteOffset, bundle.byteLength).getUint32(0, true);
  if (4 + headerLength > bundle.byteLength) {
    throw new NxWasmError(`${operation} answered with a bundle whose header does not fit.`);
  }
  const header = parseJson(operation, decoder.decode(bundle.subarray(4, 4 + headerLength)));
  if (!Array.isArray(header)) {
    throw new NxWasmError(`${operation} answered with a bundle header that is not a list.`);
  }
  return header.map((raw) => {
    const entry = asRecord(raw, "bundle entry");
    const offset = entry["offset"];
    const length = entry["length"];
    if (typeof offset !== "number" || typeof length !== "number" || offset + length > bundle.byteLength) {
      throw new NxWasmError(`${operation} answered with an image outside its bundle.`);
    }
    return {
      identity: String(entry["identity"]),
      bytes: bundle.slice(offset, offset + length),
      metadata: entry["metadata"] as NxIrMetadata
    };
  });
}

/**
 * The JSON a workspace build or validation sends for its modules and implicit imports.
 */
function workspacePayload(options: NxWorkspaceValidateOptions): {
  modules: unknown[];
  implicitImports?: string[];
} {
  return {
    modules: options.modules.map((module) => ({
      identity: module.identity,
      source: module.source,
      ...(module.version === undefined ? {} : { version: module.version })
    })),
    // Sent only when given: a list, even an empty one, replaces the build context's own.
    ...(options.implicitImports === undefined
      ? {}
      : { implicitImports: Array.from(options.implicitImports) })
  };
}

function asHandle(operation: string, handle: unknown): number {
  if (typeof handle !== "number" || !Number.isInteger(handle) || handle <= 0) {
    throw new NxWasmError(`${operation} answered with something that is not a handle.`);
  }
  return handle;
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

const valueRoles: ReadonlySet<string> = new Set<NxValueRole>([
  "record",
  "property",
  "sequence",
  "case",
  "scalar",
  "function",
  "empty"
]);

/**
 * Reads `{ text, nodes }` from the module, turning each declaration's snake_case span into the
 * SDK's `NxTextSpan` and leaving out what the module left out.
 */
function normalizeValueText(raw: unknown): NxValueText {
  const value = asRecord(raw, "evaluated value");
  if (typeof value["text"] !== "string" || !Array.isArray(value["nodes"])) {
    throw new NxWasmError("The NX wasm module returned an evaluated value in an unexpected shape.");
  }
  return { text: value["text"], nodes: value["nodes"].map(normalizeValueNode) };
}

function normalizeValueNode(raw: unknown): NxValueNode {
  const value = asRecord(raw, "value node");
  const role = value["role"];
  if (typeof role !== "string" || !valueRoles.has(role)) {
    throw new NxWasmError(`The NX wasm module returned a value node with an unknown role.`);
  }
  return {
    start: typeof value["start"] === "number" ? value["start"] : 0,
    end: typeof value["end"] === "number" ? value["end"] : 0,
    role: role as NxValueRole,
    ...(typeof value["parent"] === "number" ? { parent: value["parent"] } : {}),
    ...(typeof value["type"] === "string" ? { type: value["type"] } : {}),
    ...(typeof value["name"] === "string" ? { name: value["name"] } : {}),
    ...(value["optional"] === true ? { optional: true } : {}),
    ...(typeof value["count"] === "number" ? { count: value["count"] } : {}),
    ...(value["declaration"] === undefined || value["declaration"] === null
      ? {}
      : { declaration: normalizeTextSpan(value["declaration"]) })
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
