/**
 * A JSON value. Arrays and objects are read-only, as the canonical values of `@nx-lang/ir-runtime`
 * are, so a value the runtime returns is one of these without a copy.
 */
export type JsonValue =
  | null
  | boolean
  | number
  | string
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

export type JsonObject = { readonly [key: string]: JsonValue };

/** A JSON Schema document, draft 2020-12. The package stores and forwards it and never reads it. */
export type JsonSchema = JsonObject;

/** Whether `value` is a JSON object: not `null` and not an array. */
export function isJsonObject(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** What kind of thing `value` is, for a message about a value that is not what was expected. */
export function describeValue(value: unknown): string {
  if (value === null || value === undefined) {
    return String(value);
  }
  if (Array.isArray(value)) {
    return "a list";
  }
  if (typeof value === "object") {
    const type = (value as { $type?: unknown }).$type;
    return typeof type === "string" ? `a '${type}'` : "a record with no '$type'";
  }
  return typeof value === "string" ? `the string ${JSON.stringify(value)}` : `the ${typeof value} ${String(value)}`;
}
