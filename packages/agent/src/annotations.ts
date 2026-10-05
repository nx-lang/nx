import type { HttpToolMethod, ToolAnnotations } from "./definition.js";

/** A `FunctionTool` evaluates pure NX: it reads nothing outside the program and changes nothing. */
export const FUNCTION_TOOL_ANNOTATIONS: ToolAnnotations = Object.freeze({
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: false,
});

/** A `WebSearchTool` reads, from the open web. */
export const WEB_SEARCH_TOOL_ANNOTATIONS: ToolAnnotations = Object.freeze({
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: true,
});

/**
 * An `HttpTool`'s hints by method. Every method but `get` is a write, and a write is destructive:
 * an author cannot be trusted to declare one harmless. `put` and `delete` are idempotent as HTTP
 * defines them; the package never retries on the strength of it.
 */
const HTTP_TOOL_ANNOTATIONS: Readonly<Record<HttpToolMethod, ToolAnnotations>> = Object.freeze({
  get: Object.freeze({ readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: true }),
  post: Object.freeze({ readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: true }),
  put: Object.freeze({ readOnlyHint: false, destructiveHint: true, idempotentHint: true, openWorldHint: true }),
  patch: Object.freeze({ readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: true }),
  delete: Object.freeze({ readOnlyHint: false, destructiveHint: true, idempotentHint: true, openWorldHint: true }),
});

export const HTTP_TOOL_METHODS: readonly HttpToolMethod[] = ["get", "post", "put", "patch", "delete"];

export function isHttpToolMethod(value: unknown): value is HttpToolMethod {
  return typeof value === "string" && (HTTP_TOOL_METHODS as readonly string[]).includes(value);
}

export function httpToolAnnotations(method: HttpToolMethod): ToolAnnotations {
  return HTTP_TOOL_ANNOTATIONS[method];
}
