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

/**
 * Whether `value` is a record the copy below enters: an object whose prototype is
 * `Object.prototype`, of this realm or another, or nothing. This is the test the IR runtime's
 * input measure applies, so the copy enters what the measure enters and nothing else. A typed
 * array, a `Date`, a `Map` and an instance of a class are not records: the measure counts each as
 * one value, and the copy keeps each as it is.
 */
function isPlainRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype === null || Object.getPrototypeOf(prototype) === null;
}

/**
 * A copy of `value` with every member of a record that is `undefined` left out, at any depth:
 * what JSON keeps of it. TypeScript lets an optional field be set to `undefined`, so a host that
 * builds a record from a generated type can hold one, and the IR runtime, which reads JSON, would
 * refuse the member where it means the field is absent. Items of a list are kept as they are, and
 * so is anything that is neither a list nor a plain record: it is not entered, not copied and
 * counts as one value.
 *
 * <para>It answers nothing when `value` holds more than `maxValues` values, counting each value it
 * would keep, and reads no further. That bounds the work, and it is what ends on a value that
 * holds itself. It keeps its own stack, so a value nested very deeply is copied, or refused, and
 * never overflows the engine's.</para>
 *
 * <para>With `valueIsRecord`, `value` itself is read as the record its own members make whatever
 * built it, an instance of a class included. A caller that has established `value` is a record
 * says so; nothing below it is read that way.</para>
 */
export function withoutUndefinedMembers(value: unknown, maxValues: number, valueIsRecord = false): unknown {
  let values = 1;
  const root: { held?: unknown } = {};
  // What is still to copy: a value, and where its copy is put.
  const pending: { readonly source: unknown; readonly into: object; readonly key: string | number; readonly isRecord?: boolean }[] = [
    { source: value, into: root, key: "held", isRecord: valueIsRecord && typeof value === "object" && value !== null && !Array.isArray(value) },
  ];
  // Defined, not assigned: a key named `__proto__` is a property of its own.
  const put = (into: object, key: string | number, held: unknown): void => {
    Object.defineProperty(into, key, { value: held, enumerable: true, writable: true, configurable: true });
  };
  for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
    const { source, into, key } = next;
    if (Array.isArray(source)) {
      if ((values += source.length) > maxValues) {
        return undefined;
      }
      const copy: unknown[] = new Array<unknown>(source.length);
      put(into, key, copy);
      for (let index = source.length - 1; index >= 0; index -= 1) {
        pending.push({ source: source[index], into: copy, key: index });
      }
    } else if (next.isRecord === true || isPlainRecord(source)) {
      const record = source as Readonly<Record<string, unknown>>;
      const copy = {};
      put(into, key, copy);
      const kept = Object.keys(record).filter((member) => record[member] !== undefined);
      if ((values += kept.length) > maxValues) {
        return undefined;
      }
      for (const member of kept) {
        // Each key is defined now, so the copy's keys are in the record's order whatever order
        // their values are copied in.
        put(copy, member, undefined);
        pending.push({ source: record[member], into: copy, key: member });
      }
    } else {
      put(into, key, source);
    }
  }
  return root.held;
}
