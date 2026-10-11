/**
 * An action's form: its fields, read from the action's input schema, and the action a filled form
 * makes.
 *
 * <para>The schema is the one a host validates an action against (`typeSchema` with `direction:
 * "input"`), so what the form sends is what an embedding host would. A string, a number, a boolean
 * and a constant union each get an input of their own; anything else is written as JSON.</para>
 */
import type { NxHostRecord, NxHostValue } from "@nx-lang/ir-runtime";
import type { NxTypeSchema } from "@nx-lang/sdk-wasm";

import type { PreviewField } from "./protocol.ts";

type Schema = Readonly<Record<string, unknown>>;

/** The fields of the record type `schema` describes, in the schema's order. */
export function fieldsOf(schema: NxTypeSchema): PreviewField[] {
  const root = schema.schema;
  if (root === undefined) {
    return [];
  }
  const definitions = (root["$defs"] ?? {}) as Readonly<Record<string, Schema>>;
  const resolve = (value: Schema | undefined, depth = 0): Schema | undefined => {
    const reference = value?.["$ref"];
    if (typeof reference !== "string" || depth > 16) {
      return value;
    }
    const target = definitions[decodeURIComponent(reference.replace(/^#\/\$defs\//, ""))];
    // A reference's own keywords, `default` among them, sit beside it and win over the target's.
    const { $ref: _, ...own } = value!;
    return resolve(target === undefined ? undefined : { ...target, ...own }, depth + 1);
  };
  const record = resolve(root as Schema);
  const properties = (record?.["properties"] ?? {}) as Readonly<Record<string, Schema>>;
  const required = new Set((record?.["required"] ?? []) as readonly string[]);
  return Object.entries(properties)
    .filter(([name]) => name !== "$type")
    .map(([name, property]) => fieldOf(name, required.has(name), resolve(property) ?? {}));
}

function fieldOf(name: string, required: boolean, schema: Schema): PreviewField {
  const value = schema["default"];
  const cases = schema["enum"];
  if (Array.isArray(cases) && cases.every((entry) => typeof entry === "string")) {
    return { name, required, kind: "cases", cases, ...(typeof value === "string" ? { default: value } : {}) };
  }
  switch (schema["type"]) {
    case "string":
      return { name, required, kind: "string", ...(typeof value === "string" ? { default: value } : {}) };
    case "integer":
    case "number":
      return { name, required, kind: schema["type"], ...(typeof value === "number" ? { default: value } : {}) };
    case "boolean":
      return { name, required, kind: "boolean", ...(typeof value === "boolean" ? { default: value } : {}) };
    default:
      return { name, required, kind: "json", ...(value === undefined ? {} : { default: value }) };
  }
}

/** What a form holds for one field: its text, or a checkbox's state. */
export type FieldInput = string | boolean;

/** The input a field starts with: its default, or empty. */
export function initialInput(field: PreviewField): FieldInput {
  switch (field.kind) {
    case "boolean":
      return field.default ?? false;
    case "json":
      return field.default === undefined ? "" : JSON.stringify(field.default);
    case "cases":
      return field.default ?? (field.required ? field.cases[0] ?? "" : "");
    default:
      return field.default === undefined ? "" : String(field.default);
  }
}

/**
 * The action a filled form makes, or the first field it cannot read. An empty optional field is
 * left out, so the action takes its default.
 */
export function actionFrom(
  type: string,
  fields: readonly PreviewField[],
  inputs: Readonly<Record<string, FieldInput>>
): { readonly action: NxHostRecord } | { readonly field: string; readonly message: string } {
  const action: Record<string, NxHostValue> = { $type: type };
  for (const field of fields) {
    const input = inputs[field.name] ?? initialInput(field);
    if (field.kind === "boolean") {
      action[field.name] = input === true;
      continue;
    }
    const text = typeof input === "string" ? input : "";
    if (text === "" && field.kind !== "string") {
      if (field.required) {
        return { field: field.name, message: `${field.name} needs a value.` };
      }
      continue;
    }
    switch (field.kind) {
      case "string":
      case "cases":
        if (text === "" && !field.required) {
          continue;
        }
        action[field.name] = text;
        break;
      case "integer":
      case "number": {
        const number = Number(text);
        if (!Number.isFinite(number) || (field.kind === "integer" && !Number.isInteger(number))) {
          return { field: field.name, message: `${field.name} needs ${field.kind === "integer" ? "a whole number" : "a number"}.` };
        }
        action[field.name] = number;
        break;
      }
      case "json":
        try {
          action[field.name] = JSON.parse(text) as NxHostValue;
        } catch {
          return { field: field.name, message: `${field.name} needs a JSON value.` };
        }
        break;
    }
  }
  return { action };
}
