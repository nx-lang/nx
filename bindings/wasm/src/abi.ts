/**
 * ABI version this loader is written against. It must equal the module's `nx_wasm_abi_version`.
 */
export const abiVersion = 1;

/**
 * The operation succeeded; the payload is its JSON result.
 */
export const statusOk = 0;
/**
 * NX reported diagnostics; the payload is the JSON diagnostics array.
 */
export const statusEvaluationError = 1;
/**
 * The module could not carry out the operation; the payload is a JSON string message.
 */
export const statusInternalError = 2;

/**
 * Byte offsets within the `{ status, ptr, len }` record every operation returns a pointer to.
 *
 * The layout is the crate's `#[repr(C)] NxWasmResult` on a 32-bit target: three little-endian
 * `u32`s, twelve bytes in all. Named here rather than spelled as numbers where the record is read,
 * so the loader and the crate have one place to agree.
 */
export const resultStatusOffset = 0;
export const resultPointerOffset = 4;
export const resultLengthOffset = 8;

/**
 * The exports the loader calls. Every operation returns a pointer to a result record.
 */
export interface NxWasmExports {
  readonly memory: WebAssembly.Memory;
  nx_wasm_abi_version(): number;
  nx_wasm_alloc(length: number): number;
  nx_wasm_free(pointer: number, length: number): void;
  nx_wasm_result_free(result: number): void;
  nx_wasm_program_build(pointer: number, length: number): number;
  nx_wasm_program_nx_ir(handle: number): number;
  nx_wasm_program_free(handle: number): void;
  nx_wasm_snapshot_new(pointer: number, length: number): number;
  nx_wasm_snapshot_hover(handle: number, pointer: number, length: number): number;
  nx_wasm_snapshot_completions(handle: number, pointer: number, length: number): number;
  nx_wasm_snapshot_diagnostics(handle: number): number;
  nx_wasm_snapshot_document_symbols(handle: number, pointer: number, length: number): number;
  nx_wasm_snapshot_free(handle: number): void;
  /**
   * Present only in a module built with the crate's `debug-trap` feature; it traps on purpose.
   */
  nx_wasm_trap?(): void;
}

/**
 * How a host supplies the WASI imports the module needs and starts its instance.
 *
 * <para>Each entry point of the package provides one of these: the browser entry over a WASI shim,
 * the Node entry over `node:wasi`. Nothing else in the loader knows which is in use.</para>
 */
export interface WasiProvider {
  /**
   * Import object to instantiate the module with, holding the `wasi_snapshot_preview1` namespace.
   */
  readonly imports: WebAssembly.Imports;

  /**
   * Runs the module's reactor initialization against the instance the imports were used for.
   */
  initialize(instance: WebAssembly.Instance): void;
}

/**
 * Creates the WASI provider for one instance. A provider is never shared between instances.
 */
export type CreateWasiProvider = () => WasiProvider;
