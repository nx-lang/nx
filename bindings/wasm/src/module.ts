import { NxWasmError } from "./errors.js";

/**
 * What `compileNxModule` accepts: the module's bytes, a fetch `Response` carrying them, or an
 * already-compiled module.
 */
export type NxModuleSource = BufferSource | Response | PromiseLike<Response> | WebAssembly.Module;

/**
 * Compiles the NX WebAssembly module once, so hosts can be created from it without fetching or
 * recompiling.
 *
 * <para>A `Response` is compiled as it streams where the host supports it, which is what makes the
 * module cheap to serve as an ordinary immutable asset. An already-compiled module is returned as
 * it is, so callers can pass one through without a special case.</para>
 *
 * @throws NxWasmError when the source is not something the loader can compile.
 */
export async function compileNxModule(source: NxModuleSource): Promise<WebAssembly.Module> {
  if (source instanceof WebAssembly.Module) {
    return source;
  }

  if (isBufferSource(source)) {
    return WebAssembly.compile(source);
  }

  if (isResponseLike(source)) {
    const response = await source;
    if (typeof WebAssembly.compileStreaming === "function") {
      return WebAssembly.compileStreaming(Promise.resolve(response));
    }
    return WebAssembly.compile(await response.arrayBuffer());
  }

  throw new NxWasmError(
    "compileNxModule needs the module's bytes, a Response carrying them, or a WebAssembly.Module."
  );
}

function isBufferSource(source: unknown): source is BufferSource {
  return source instanceof ArrayBuffer || ArrayBuffer.isView(source);
}

function isResponseLike(source: unknown): source is Response | PromiseLike<Response> {
  return (
    typeof source === "object" &&
    source !== null &&
    (typeof (source as PromiseLike<Response>).then === "function" ||
      typeof (source as Response).arrayBuffer === "function")
  );
}
