import { httpToolAnnotations } from "../src/annotations.js";
import type { HttpToolMethod, NormalizedHttpTool } from "../src/index.js";

/** An `http` tool definition as normalization would store it, for the builder's tests. */
export function httpTool(
  method: HttpToolMethod,
  path: string,
  baseUrl = "https://api.example.com",
  pathPlaceholders: readonly string[] = [...path.matchAll(/\{([^{}]*)\}/g)].map((match) => match[1]!),
): NormalizedHttpTool {
  return {
    name: "lookup_order",
    description: "Looks an order up.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
    annotations: httpToolAnnotations(method),
    kind: "http",
    config: {
      arguments: { module: "main.nx", name: "lookupOrder" },
      contextParameters: [],
      connection: { name: "shop", baseUrl },
      method,
      path,
      pathPlaceholders: [...new Set(pathPlaceholders)],
    },
  };
}

/** An `HttpArguments` value in canonical form, as an arguments function returns it. */
export function httpArguments(
  pathParams: Readonly<Record<string, string>> | readonly (readonly [string, string])[] = {},
  query: readonly (readonly [string, string])[] = [],
  body?: unknown,
): Record<string, unknown> {
  const pairs = Array.isArray(pathParams) ? pathParams : Object.entries(pathParams);
  const value: Record<string, unknown> = { $type: "HttpArguments" };
  if (pairs.length > 0) {
    value["pathParams"] = pairs.map(([name, held]) => ({ $type: "HttpParam", name, value: held }));
  }
  if (query.length > 0) {
    value["query"] = query.map(([name, held]) => ({ $type: "HttpParam", name, value: held }));
  }
  if (body !== undefined) {
    value["body"] = body;
  }
  return value;
}
