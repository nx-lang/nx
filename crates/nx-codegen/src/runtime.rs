use crate::options::{CodegenTarget, NX_JS_RUNTIME_ABI};

pub fn javascript_runtime_abi() -> &'static str {
    NX_JS_RUNTIME_ABI
}

pub fn javascript_runtime_helper_source() -> String {
    javascript_runtime()
}

pub fn runtime_helper_source(target: CodegenTarget) -> String {
    match target {
        CodegenTarget::TypeScript => typescript_runtime(),
        CodegenTarget::JavaScript => javascript_runtime(),
    }
}

fn typescript_runtime() -> String {
    r#"export type NxValue =
  | null
  | boolean
  | number
  | string
  | readonly NxValue[]
  | { readonly [key: string]: NxValue };

/**
 * The empty value: the empty sequence, which a host may also write as `null`. A `?` type is
 * `T | NxEmpty`, and generated code reads it through `nxExists`, `nxStep`, `nxCoalesce` and
 * `nxItems`, which treat both spellings as the one empty.
 */
export type NxEmpty = null | readonly never[];

/**
 * The empty value as generated code writes it: an empty array, which is both the empty `T?` and
 * the empty `T*`.
 */
export const nxEmpty: readonly never[] = [];

export type NxDiagnostic = {
  readonly code?: string;
  readonly message: string;
};

export type NxResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly diagnostics: readonly NxDiagnostic[] };

export type NxSchema =
  | "any"
  | "boolean"
  | "float32"
  | "number"
  | "string"
  | { readonly array: NxSchema; readonly nonEmpty?: boolean }
  | { readonly optional: NxSchema }
  | { readonly enum: readonly string[] }
  | NxRecordSchema
  | { readonly union: readonly NxSchema[] };

export type NxFieldSchema = {
  readonly name: string;
  readonly schema: NxSchema;
  readonly required: boolean;
  /** Whether the field carries the `?` mark, so that an empty value is an omitted key. */
  readonly optional?: boolean;
  readonly hasDefault?: boolean;
  readonly defaultValue?: NxValue;
  readonly defaultFactory?: NxFieldDefault;
};

export type NxRecordSchema = {
  readonly record?: string;
  readonly fields: readonly NxFieldSchema[];
};

type NxFieldDefault = (record: Record<string, NxValue>) => NxValue;

type NxFieldInput = Omit<NxFieldSchema, "name">;

export type NxExternalComponentSchema<TProps, TElement> = {
  readonly name: string;
  readonly props: NxRecordSchema;
  readonly element: NxRecordSchema;
  fromJson(input?: NxValue): TElement;
  tryFromJson(input?: NxValue): NxResult<TElement>;
};

export type NxComponentSchema<TProps, TState, TRendered> = {
  readonly name: string;
  readonly props: NxRecordSchema;
  readonly element: NxRecordSchema;
  readonly state?: NxRecordSchema;
  initializeJson(props?: NxValue): { readonly rendered: TRendered; readonly state: TState };
  evaluateJson(props?: NxValue, state?: NxValue): TRendered;
  tryInitializeJson(props?: NxValue): NxResult<{ readonly rendered: TRendered; readonly state: TState }>;
  tryEvaluateJson(props?: NxValue, state?: NxValue): NxResult<TRendered>;
};

export const nxAnySchema: NxSchema = "any";
export const nxBooleanSchema: NxSchema = "boolean";
export const nxFloat32Schema: NxSchema = "float32";
export const nxNumberSchema: NxSchema = "number";
export const nxStringSchema: NxSchema = "string";

export function nxArraySchema(element: NxSchema): NxSchema {
  return { array: element };
}

export function nxNonEmptyArraySchema(element: NxSchema): NxSchema {
  return { array: element, nonEmpty: true };
}

export function nxOptionalSchema(item: NxSchema): NxSchema {
  return { optional: item };
}

/**
 * The empty value is the empty sequence. A host may also send `null`, and a record carries no key
 * for an empty optional field, so a missing key reads as `undefined`; all three are the one empty.
 */
export function nxIsEmpty(value: unknown): boolean {
  return value == null || (Array.isArray(value) && value.length === 0);
}

/** `x?`: true when the value holds at least one item. */
export function nxExists(value: unknown): boolean {
  return !nxIsEmpty(value);
}

/** The item type of a value that may be empty or a sequence: `R` for `R | NxEmpty` or `readonly R[]`. */
export type NxItem<T> = Exclude<T, NxEmpty | undefined> extends infer R
  ? R extends readonly (infer I)[]
    ? I
    : R
  : never;

/** The type of member `K` of the item a value holds, or `NxValue` where the item type is not a record. */
export type NxMember<T, K extends string> = NxItem<T> extends infer R
  ? R extends { readonly [P in K]?: infer V }
    ? V
    : NxValue
  : never;

/** `x?.m`: the empty value when the receiver is empty, its member otherwise. */
export function nxStep<T, K extends string>(value: T, member: K): NxMember<T, K> | readonly never[] {
  if (nxIsEmpty(value)) {
    return [];
  }
  const receiver = (Array.isArray(value) ? value[0] : value) as Record<string, NxMember<T, K>>;
  return receiver[member] ?? [];
}

/** `x ?? y`: the left operand when it holds an item, else the fallback, run only then. */
export function nxCoalesce<T, U>(value: T, fallback: () => U): Exclude<T, NxEmpty | undefined> | U {
  return nxIsEmpty(value) ? fallback() : (value as Exclude<T, NxEmpty | undefined>);
}

/**
 * A `for` over an optional (or exactly-one) value: the body's value for the one item, unchanged,
 * or the empty value. The product of `?` with the body's occurrence is the body's own, so an item
 * stays an item rather than becoming a one-element array.
 */
export function nxForOne<T, U>(
  value: T | NxEmpty | undefined,
  body: (item: T, index: number) => U,
): U | readonly never[] {
  if (nxIsEmpty(value)) {
    return [];
  }
  return body((Array.isArray(value) ? value[0] : value) as T, 0);
}

/** The items a `for` iterates: a sequence's items, an optional's one item, or none. */
export function nxItems<T>(value: T | readonly T[] | NxEmpty | undefined): readonly T[] {
  if (nxIsEmpty(value)) {
    return [];
  }
  return Array.isArray(value) ? (value as readonly T[]) : [value as T];
}

/**
 * The entry for an optional field: the key with its value when the value holds an item, and no
 * key when it is empty, so that an empty optional field is an omitted key.
 */
export function nxOptional<K extends string, T>(
  name: K,
  value: T | NxEmpty,
  many = false,
): { readonly [P in K]?: T } {
  if (nxIsEmpty(value)) {
    return {};
  }
  // A `?` field holds its item as itself; a `?:T+` field keeps its sequence.
  const item = !many && Array.isArray(value) && value.length === 1 ? value[0] : value;
  return { [name]: item } as { readonly [P in K]?: T };
}

/** The value of a present clearable update-record field: `null` for a cleared one. */
export function nxCleared<T>(value: T): Exclude<T, NxEmpty> | null {
  return nxIsEmpty(value) ? null : (value as Exclude<T, NxEmpty>);
}

export function nxEnumSchema(members: readonly string[]): NxSchema {
  return { enum: members };
}

export function nxUnionSchema(cases: readonly NxSchema[]): NxSchema {
  return { union: cases };
}

export function nxNamedRecordSchema(record: string, fields: readonly NxFieldSchema[]): NxRecordSchema {
  return { record, fields };
}

export function nxField(
  schema: NxSchema,
  options: { readonly required?: boolean; readonly defaultValue?: NxValue } = {},
): NxFieldInput {
  const hasDefault = Object.prototype.hasOwnProperty.call(options, "defaultValue");
  return {
    schema,
    required: options.required ?? !hasDefault,
    ...(hasDefault ? { hasDefault: true, defaultValue: options.defaultValue } : {}),
  };
}

export function nxRecordSchema(fields: Record<string, NxFieldInput>): NxRecordSchema {
  return {
    fields: Object.entries(fields).map(([name, field]) => ({ name, ...field })),
  };
}

export class NxRuntimeError extends Error {
  readonly diagnostics: readonly NxDiagnostic[];

  constructor(diagnostics: readonly NxDiagnostic[]) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
    this.name = "NxRuntimeError";
    this.diagnostics = diagnostics;
  }
}

export function nxElement(
  tag: string,
  properties: Record<string, NxValue>,
  content: readonly NxValue[] = [],
): NxValue {
  const output: Record<string, NxValue> = { ...properties };
  if (content.length === 1) {
    output.content = content[0];
  } else if (content.length > 1) {
    output.content = content;
  }
  return { $type: tag, ...output };
}

export function nxExternalComponentSchema<TProps, TElement>(config: {
  readonly name: string;
  readonly props: NxRecordSchema;
  readonly create: (props: TProps) => TElement;
}): NxExternalComponentSchema<TProps, TElement> {
  return {
    name: config.name,
    props: config.props,
    element: nxElementRecordSchema(config.name, config.props),
    fromJson(input: NxValue = {}) {
      return config.create(nxNormalizeBoundaryRecord<TProps>(input, config.props, `${config.name} props`));
    },
    tryFromJson(input: NxValue = {}) {
      try {
        return { ok: true, value: this.fromJson(input) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
  };
}

function nxElementRecordSchema(record: string, props: NxRecordSchema): NxRecordSchema {
  return nxNamedRecordSchema(
    record,
    props.fields.map((field) => ({
      name: field.name,
      schema: field.schema,
      required: true,
    })),
  );
}

export function nxComponentSchema<TProps, TState, TRendered>(config: {
  readonly name: string;
  readonly props: NxRecordSchema;
  readonly state?: NxRecordSchema;
  readonly initialize: (props: TProps) => { readonly rendered: TRendered; readonly state: TState };
  readonly evaluate: (props: TProps, state?: TState) => TRendered;
}): NxComponentSchema<TProps, TState, TRendered> {
  return {
    name: config.name,
    props: config.props,
    element: nxElementRecordSchema(config.name, config.props),
    state: config.state,
    initializeJson(props: NxValue = {}) {
      return config.initialize(nxNormalizeBoundaryRecord<TProps>(props, config.props, `${config.name} props`));
    },
    evaluateJson(props: NxValue = {}, state?: NxValue) {
      const normalizedProps = nxNormalizeBoundaryRecord<TProps>(props, config.props, `${config.name} props`);
      const normalizedState = state === undefined
        ? undefined
        : nxNormalizeBoundaryRecord<TState>(state, config.state ?? nxRecordSchema({}), `${config.name} state`);
      return config.evaluate(normalizedProps, normalizedState);
    },
    tryInitializeJson(props: NxValue = {}) {
      try {
        return { ok: true, value: this.initializeJson(props) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
    tryEvaluateJson(props: NxValue = {}, state?: NxValue) {
      try {
        return { ok: true, value: this.evaluateJson(props, state) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
  };
}

function nxNormalizeBoundaryRecord<T>(input: NxValue | undefined, schema: NxRecordSchema, operation: string): T {
  return nxNormalizeRecordInput(nxAssertRecord(input, operation), schema, operation) as T;
}

export function nxAssertRecord(input: NxValue | undefined, operation: string): Record<string, NxValue> {
  if (input == null) {
    return {};
  }
  if (typeof input === "object" && !Array.isArray(input)) {
    return input as Record<string, NxValue>;
  }
  throw new NxRuntimeError([
    {
      code: "invalid-record",
      message: `${operation} expected an object`,
    },
  ]);
}

export function nxRejectUnknownFields(
  input: Record<string, NxValue>,
  allowed: readonly string[],
  operation: string,
): void {
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(input)) {
    if (key !== "$type" && !allowedSet.has(key)) {
      throw new NxRuntimeError([
        {
          code: "unknown-field",
          message: `${operation} has unknown field '${key}'`,
        },
      ]);
    }
  }
}

export function nxMissingField(field: string, operation: string): never {
  throw new NxRuntimeError([
    {
      code: "missing-field",
      message: `${operation} is missing required field '${field}'`,
    },
  ]);
}

export function nxNormalizeValue(value: NxValue, schema: NxSchema, path: string): any {
  // A `?` site: a missing key, `null` and an empty array are the empty value, a one-element
  // array is its element, and a longer array is refused.
  if (typeof schema === "object" && "optional" in schema) {
    if (nxIsEmpty(value)) {
      return [];
    }
    if (Array.isArray(value)) {
      if (value.length !== 1) {
        throw new NxRuntimeError([
          {
            code: "invalid-field",
            message: `${path} expected at most one value, got ${value.length}`,
          },
        ]);
      }
      return nxNormalizeValue(value[0]!, schema.optional, path);
    }
    return nxNormalizeValue(value, schema.optional, path);
  }
  // A `+` or `*` site: an item is a sequence of one, `null` is the empty sequence, and a `+`
  // site refuses the empty sequence.
  if (typeof schema === "object" && "array" in schema) {
    const items = value === null ? [] : Array.isArray(value) ? value : [value];
    if (schema.nonEmpty === true && items.length === 0) {
      throw new NxRuntimeError([
        {
          code: "invalid-array",
          message: `${path} expected at least one value`,
        },
      ]);
    }
    return items.map((element, index) => nxNormalizeValue(element, schema.array, `${path}[${index}]`));
  }
  if (typeof schema === "object" && "enum" in schema) {
    if (typeof value === "string" && schema.enum.includes(value)) {
      return value;
    }
    throw new NxRuntimeError([
      {
        code: "invalid-enum",
        message: `${path} has invalid enum member ${JSON.stringify(value)}`,
      },
    ]);
  }
  if (typeof schema === "object" && "record" in schema) {
    return nxNormalizeRecordValue(value, schema, path);
  }
  if (typeof schema === "object" && "union" in schema) {
    const input = nxAssertRecord(value, path);
    const typeName = nxRequireRecordType(input, path);
    // A mixed union also lists its constant cases as enum schemas; only a record case can
    // match a `$type`.
    const caseSchema = schema.union.find(
      (candidate): candidate is NxRecordSchema =>
        typeof candidate === "object" && "record" in candidate && candidate.record === typeName,
    );
    if (caseSchema == null) {
      throw new NxRuntimeError([
        {
          code: "invalid-union",
          message: `${path} has invalid union case ${JSON.stringify(typeName)}`,
        },
      ]);
    }
    return nxNormalizeRecordInput(input, caseSchema, path);
  }
  switch (schema) {
    case "any":
      return value;
    case "boolean":
      if (typeof value === "boolean") {
        return value;
      }
      break;
    case "number":
      if (typeof value === "number" && Number.isFinite(value)) {
        return value;
      }
      break;
    // JSON cannot spell a float32, so a number takes the width of its site as a literal written
    // there does: an integer only when a float32 holds it exactly, a real rounded to the nearest.
    case "float32":
      if (
        typeof value === "number" &&
        Number.isFinite(value) &&
        (!Number.isInteger(value) || Math.fround(value) === value)
      ) {
        return Math.fround(value);
      }
      break;
    case "string":
      if (typeof value === "string") {
        return value;
      }
      break;
  }
  throw new NxRuntimeError([
    {
      code: "invalid-field",
      message: `${path} has invalid value for ${JSON.stringify(schema)}`,
    },
  ]);
}

function nxNormalizeRecordValue(value: NxValue, schema: NxRecordSchema, path: string): Record<string, NxValue> {
  const input = nxAssertRecord(value, path);
  if (schema.record != null) {
    const typeName = nxRequireRecordType(input, path);
    if (typeName !== schema.record) {
      throw new NxRuntimeError([
        {
          code: "invalid-record-type",
          message: `${path} expected ${schema.record}, got ${JSON.stringify(typeName)}`,
        },
      ]);
    }
  }
  return nxNormalizeRecordInput(input, schema, path);
}

function nxNormalizeRecordInput(
  input: Record<string, NxValue>,
  schema: NxRecordSchema,
  path: string,
): Record<string, NxValue> {
  const allowed = schema.fields.map((field) => field.name);
  nxRejectUnknownFields(input, allowed, path);

  const output: Record<string, NxValue> = {};
  if (schema.record != null) {
    output.$type = schema.record;
  }
  for (const field of schema.fields) {
    const hasField = Object.prototype.hasOwnProperty.call(input, field.name);
    if (hasField) {
      output[field.name] = nxNormalizeValue(input[field.name], field.schema, `${path}.${field.name}`);
    } else if (field.defaultFactory != null) {
      output[field.name] = nxNormalizeValue(field.defaultFactory(output), field.schema, `${path}.${field.name}`);
    } else if (field.hasDefault === true) {
      output[field.name] = field.defaultValue ?? [];
    } else if (field.required) {
      nxMissingField(`${path}.${field.name}`, path);
    }
    // An empty optional field is an omitted key, a `p?:T+` one included, whose read type is a
    // sequence.
    if (
      Object.prototype.hasOwnProperty.call(output, field.name) &&
      nxIsEmpty(output[field.name]) &&
      (field.optional === true || !isSequenceSchema(field.schema))
    ) {
      delete output[field.name];
    }
  }
  return output;
}

/** Whether a schema is a `+` or `*` site, where the empty value is an empty array, not an omitted key. */
function isSequenceSchema(schema: NxSchema): boolean {
  return typeof schema === "object" && "array" in schema;
}

function nxRequireRecordType(input: Record<string, NxValue>, path: string): string {
  const typeName = input.$type;
  if (typeof typeName === "string") {
    return typeName;
  }
  throw new NxRuntimeError([
    {
      code: "missing-record-type",
      message: `${path} is missing required string field '$type'`,
    },
  ]);
}

export function nxDiagnosticsFromError(error: unknown): readonly NxDiagnostic[] {
  if (error instanceof NxRuntimeError) {
    return error.diagnostics;
  }
  if (error instanceof Error) {
    return [{ code: "runtime-error", message: error.message }];
  }
  return [{ code: "runtime-error", message: String(error) }];
}

export function nxRuntimeError(message: string): never {
  throw new NxRuntimeError([{ code: "runtime-error", message }]);
}

/** `apply`: a present field replaces the record's, and a present empty one clears it, leaving no key. */
export function nxApplyUpdate<T>(record: T, update: unknown): T {
  const { $type: _update, ...fields } = update as Record<string, NxValue>;
  const output: Record<string, NxValue> = { ...(record as Record<string, NxValue>) };
  for (const [key, value] of Object.entries(fields)) {
    if (nxIsEmpty(value)) {
      delete output[key];
    } else {
      output[key] = value;
    }
  }
  return output as T;
}

export function nxMergeUpdates<T>(first: T, second: T): T {
  const { $type: _second, ...later } = second as Record<string, NxValue>;
  return { ...(first as Record<string, NxValue>), ...later } as T;
}

/** `diff`: the fields that differ, each with its value from `after`, a cleared one as `null`. */
export function nxDiffRecords(before: unknown, after: unknown): any {
  const beforeRecord = before as Record<string, NxValue>;
  const afterRecord = after as Record<string, NxValue>;
  const output: Record<string, NxValue> = { $type: `${String(beforeRecord.$type)}.Update` };
  // A field either record leaves out reads as the empty value, so a field only one of them
  // carries still compares.
  for (const key of new Set([...Object.keys(beforeRecord), ...Object.keys(afterRecord)])) {
    if (key === "$type") {
      continue;
    }
    const next = afterRecord[key] ?? [];
    if (!nxValuesEqual(beforeRecord[key] ?? [], next)) {
      output[key] = nxCleared(next);
    }
  }
  return output;
}

/**
 * Runs `body` once per integer in a range record, in order, and collects the results.
 *
 * The count is computed once, so a closed range ending at the largest exact integer terminates, and
 * the integers themselves are never collected into a list. An empty or reversed range runs the body
 * no times.
 *
 * The range is not checked against the prelude's `$type`, where the IR runtime checks it, because
 * the emitter writes this call only at a site the checker proved to be a range of an integer type.
 * The IR runtime evaluates images it did not emit, so it guards; generated code is its own output.
 */
export function nxRangeMap<T>(
  range: {
    readonly $type?: string;
    readonly start: number;
    readonly end: number;
    readonly endInclusive: boolean;
  },
  body: (item: number, index: number) => T,
): T[] {
  const count =
    range.end > range.start
      ? range.end - range.start + (range.endInclusive ? 1 : 0)
      : range.end === range.start && range.endInclusive
        ? 1
        : 0;
  const results: T[] = [];
  for (let offset = 0; offset < count; offset += 1) {
    results.push(body(range.start + offset, offset));
  }
  return results;
}

/**
 * Integer division, which truncates toward zero as the interpreter and the IR runtime's `idiv` do.
 */
export function nxIntDiv(dividend: number, divisor: number): number {
  return Math.trunc(nxDiv(dividend, divisor)) + 0;
}

/**
 * Division, which fails on a zero divisor as the interpreter and the IR runtime's `div` do, rather
 * than producing JavaScript's `Infinity` or `NaN`.
 */
export function nxDiv(dividend: number, divisor: number): number {
  if (divisor === 0) {
    nxRuntimeError("Division by zero");
  }
  return dividend / divisor;
}

/**
 * A remainder, which fails on a zero divisor as the interpreter and the IR runtime's `mod` do,
 * rather than producing JavaScript's `NaN`.
 */
export function nxMod(dividend: number, divisor: number): number {
  if (divisor === 0) {
    nxRuntimeError("Division by zero");
  }
  return (dividend % divisor) + 0;
}

/**
 * The canonical text form of a `float32`, which generated code carries as the `number` it widens
 * to. `String(value)` would print that widening's digits (`0.10000000149011612`); this prints the
 * shortest digits that round-trip to the same `float32` (`0.1`), in the same ECMAScript layout.
 */
export function nxFloat32Text(value: number): string {
  const target = Math.fround(value);
  if (!Number.isFinite(target) || target === 0) {
    return String(target);
  }
  for (let precision = 1; precision <= 9; precision += 1) {
    const candidate = Number(target.toPrecision(precision));
    if (Math.fround(candidate) === target) {
      return String(candidate);
    }
  }
  return String(target);
}

export function nxChangedFields(update: unknown, order: readonly string[]): any[] {
  if (!Array.isArray(order)) {
    nxRuntimeError("nxChangedFields needs the update record's declared field order");
  }
  const keys = Object.keys(update as Record<string, NxValue>).filter((key) => key !== "$type");
  const position = (key: string): number => {
    const index = order.indexOf(key);
    return index < 0 ? order.length : index;
  };
  return keys.sort((left, right) => position(left) - position(right));
}

/**
 * `==`: numbers, strings and booleans by value, sequences by their items in order, records by
 * their fields. Every value compares as a sequence, so an item equals a sequence of one holding
 * an equal item, and every spelling of the empty value equals only the empty value.
 */
export function nxValuesEqual(left: unknown, right: unknown): boolean {
  if (nxIsEmpty(left) || nxIsEmpty(right)) {
    return nxIsEmpty(left) && nxIsEmpty(right);
  }
  if (Array.isArray(left) && Array.isArray(right)) {
    return (
      left.length === right.length &&
      left.every((item, index) => nxValuesEqual(item, right[index] ?? null))
    );
  }
  if (Array.isArray(left)) {
    return left.length === 1 && nxValuesEqual(left[0], right);
  }
  if (Array.isArray(right)) {
    return right.length === 1 && nxValuesEqual(left, right[0]);
  }
  if (left !== null && typeof left === "object") {
    if (right === null || typeof right !== "object") {
      return false;
    }
    const leftRecord = left as Record<string, NxValue>;
    const rightRecord = right as Record<string, NxValue>;
    const leftKeys = Object.keys(leftRecord);
    const rightKeys = Object.keys(rightRecord);
    return (
      leftKeys.length === rightKeys.length &&
      leftKeys.every((key) => nxValuesEqual(leftRecord[key] ?? null, rightRecord[key] ?? null))
    );
  }
  return left === right;
}
"#
    .to_string()
}

fn javascript_runtime() -> String {
    r#"export class NxRuntimeError extends Error {
  constructor(diagnostics) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
    this.name = "NxRuntimeError";
    this.diagnostics = diagnostics;
  }
}

export const nxAnySchema = "any";
export const nxBooleanSchema = "boolean";
export const nxFloat32Schema = "float32";
export const nxNumberSchema = "number";
export const nxStringSchema = "string";

export function nxArraySchema(element) {
  return { array: element };
}

export function nxNonEmptyArraySchema(element) {
  return { array: element, nonEmpty: true };
}

export function nxOptionalSchema(item) {
  return { optional: item };
}

/**
 * The empty value is the empty sequence. A host may also send `null`, and a record carries no key
 * for an empty optional field, so a missing key reads as `undefined`; all three are the one empty.
 */
export function nxIsEmpty(value) {
  return value == null || (Array.isArray(value) && value.length === 0);
}

/** The empty value as generated code writes it. */
export const nxEmpty = [];

/** `x?`: true when the value holds at least one item. */
export function nxExists(value) {
  return !nxIsEmpty(value);
}

/** `x?.m`: the empty value when the receiver is empty, its member otherwise. */
export function nxStep(value, member) {
  if (nxIsEmpty(value)) {
    return [];
  }
  const receiver = Array.isArray(value) ? value[0] : value;
  return receiver[member] ?? [];
}

/** `x ?? y`: the left operand when it holds an item, else the fallback, run only then. */
export function nxCoalesce(value, fallback) {
  return nxIsEmpty(value) ? fallback() : value;
}

/**
 * A `for` over an optional (or exactly-one) value: the body's value for the one item, unchanged,
 * or the empty value.
 */
export function nxForOne(value, body) {
  if (nxIsEmpty(value)) {
    return [];
  }
  return body(Array.isArray(value) ? value[0] : value, 0);
}

/** The items a `for` iterates: a sequence's items, an optional's one item, or none. */
export function nxItems(value) {
  if (nxIsEmpty(value)) {
    return [];
  }
  return Array.isArray(value) ? value : [value];
}

/**
 * The entry for an optional field: the key with its value when the value holds an item, and no
 * key when it is empty, so that an empty optional field is an omitted key.
 */
export function nxOptional(name, value, many = false) {
  if (nxIsEmpty(value)) {
    return {};
  }
  // A `?` field holds its item as itself; a `?:T+` field keeps its sequence.
  return { [name]: !many && Array.isArray(value) && value.length === 1 ? value[0] : value };
}

/** The value of a present update-record field: `null` for a cleared one. */
export function nxCleared(value) {
  return nxIsEmpty(value) ? null : value;
}

export function nxEnumSchema(members) {
  return { enum: members };
}

export function nxUnionSchema(cases) {
  return { union: cases };
}

export function nxNamedRecordSchema(record, fields) {
  return { record, fields };
}

export function nxField(schema, options = {}) {
  const hasDefault = Object.prototype.hasOwnProperty.call(options, "defaultValue");
  return {
    schema,
    required: options.required ?? !hasDefault,
    ...(hasDefault ? { hasDefault: true, defaultValue: options.defaultValue } : {}),
  };
}

export function nxRecordSchema(fields) {
  return {
    fields: Object.entries(fields).map(([name, field]) => ({ name, ...field })),
  };
}

export function nxElement(tag, properties, content = []) {
  const output = { ...properties };
  if (content.length === 1) {
    output.content = content[0];
  } else if (content.length > 1) {
    output.content = content;
  }
  return { $type: tag, ...output };
}

export function nxExternalComponentSchema(config) {
  return {
    name: config.name,
    props: config.props,
    element: nxElementRecordSchema(config.name, config.props),
    fromJson(input = {}) {
      return config.create(nxNormalizeBoundaryRecord(input, config.props, `${config.name} props`));
    },
    tryFromJson(input = {}) {
      try {
        return { ok: true, value: this.fromJson(input) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
  };
}

function nxElementRecordSchema(record, props) {
  return nxNamedRecordSchema(
    record,
    props.fields.map((field) => ({
      name: field.name,
      schema: field.schema,
      required: true,
    })),
  );
}

export function nxComponentSchema(config) {
  return {
    name: config.name,
    props: config.props,
    element: nxElementRecordSchema(config.name, config.props),
    state: config.state,
    initializeJson(props = {}) {
      return config.initialize(nxNormalizeBoundaryRecord(props, config.props, `${config.name} props`));
    },
    evaluateJson(props = {}, state = undefined) {
      const normalizedProps = nxNormalizeBoundaryRecord(props, config.props, `${config.name} props`);
      const normalizedState = state === undefined
        ? undefined
        : nxNormalizeBoundaryRecord(state, config.state ?? nxRecordSchema({}), `${config.name} state`);
      return config.evaluate(normalizedProps, normalizedState);
    },
    tryInitializeJson(props = {}) {
      try {
        return { ok: true, value: this.initializeJson(props) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
    tryEvaluateJson(props = {}, state = undefined) {
      try {
        return { ok: true, value: this.evaluateJson(props, state) };
      } catch (error) {
        return { ok: false, diagnostics: nxDiagnosticsFromError(error) };
      }
    },
  };
}

function nxNormalizeBoundaryRecord(input, schema, operation) {
  return nxNormalizeRecordInput(nxAssertRecord(input, operation), schema, operation);
}

export function nxAssertRecord(input, operation) {
  if (input == null) {
    return {};
  }
  if (typeof input === "object" && !Array.isArray(input)) {
    return input;
  }
  throw new NxRuntimeError([
    {
      code: "invalid-record",
      message: `${operation} expected an object`,
    },
  ]);
}

export function nxRejectUnknownFields(input, allowed, operation) {
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(input)) {
    if (key !== "$type" && !allowedSet.has(key)) {
      throw new NxRuntimeError([
        {
          code: "unknown-field",
          message: `${operation} has unknown field '${key}'`,
        },
      ]);
    }
  }
}

export function nxMissingField(field, operation) {
  throw new NxRuntimeError([
    {
      code: "missing-field",
      message: `${operation} is missing required field '${field}'`,
    },
  ]);
}

export function nxNormalizeValue(value, schema, path) {
  // A `?` site: a missing key, `null` and an empty array are the empty value, a one-element
  // array is its element, and a longer array is refused.
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "optional")) {
    if (nxIsEmpty(value)) {
      return [];
    }
    if (Array.isArray(value)) {
      if (value.length !== 1) {
        throw new NxRuntimeError([
          {
            code: "invalid-field",
            message: `${path} expected at most one value, got ${value.length}`,
          },
        ]);
      }
      return nxNormalizeValue(value[0], schema.optional, path);
    }
    return nxNormalizeValue(value, schema.optional, path);
  }
  // A `+` or `*` site: an item is a sequence of one, `null` is the empty sequence, and a `+`
  // site refuses the empty sequence.
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "array")) {
    const items = value === null ? [] : Array.isArray(value) ? value : [value];
    if (schema.nonEmpty === true && items.length === 0) {
      throw new NxRuntimeError([
        {
          code: "invalid-array",
          message: `${path} expected at least one value`,
        },
      ]);
    }
    return items.map((element, index) => nxNormalizeValue(element, schema.array, `${path}[${index}]`));
  }
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "enum")) {
    if (typeof value === "string" && schema.enum.includes(value)) {
      return value;
    }
    throw new NxRuntimeError([
      {
        code: "invalid-enum",
        message: `${path} has invalid enum member ${JSON.stringify(value)}`,
      },
    ]);
  }
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "record")) {
    return nxNormalizeRecordValue(value, schema, path);
  }
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "union")) {
    const input = nxAssertRecord(value, path);
    const typeName = nxRequireRecordType(input, path);
    // A mixed union also lists its constant cases as enum schemas; only a record case can
    // match a `$type`.
    const caseSchema = schema.union.find(
      (candidate) =>
        typeof candidate === "object" && "record" in candidate && candidate.record === typeName,
    );
    if (caseSchema == null) {
      throw new NxRuntimeError([
        {
          code: "invalid-union",
          message: `${path} has invalid union case ${JSON.stringify(typeName)}`,
        },
      ]);
    }
    return nxNormalizeRecordInput(input, caseSchema, path);
  }
  switch (schema) {
    case "any":
      return value;
    case "boolean":
      if (typeof value === "boolean") {
        return value;
      }
      break;
    case "number":
      if (typeof value === "number" && Number.isFinite(value)) {
        return value;
      }
      break;
    // JSON cannot spell a float32, so a number takes the width of its site as a literal written
    // there does: an integer only when a float32 holds it exactly, a real rounded to the nearest.
    case "float32":
      if (
        typeof value === "number" &&
        Number.isFinite(value) &&
        (!Number.isInteger(value) || Math.fround(value) === value)
      ) {
        return Math.fround(value);
      }
      break;
    case "string":
      if (typeof value === "string") {
        return value;
      }
      break;
  }
  throw new NxRuntimeError([
    {
      code: "invalid-field",
      message: `${path} has invalid value for ${JSON.stringify(schema)}`,
    },
  ]);
}

function nxNormalizeRecordValue(value, schema, path) {
  const input = nxAssertRecord(value, path);
  if (schema.record != null) {
    const typeName = nxRequireRecordType(input, path);
    if (typeName !== schema.record) {
      throw new NxRuntimeError([
        {
          code: "invalid-record-type",
          message: `${path} expected ${schema.record}, got ${JSON.stringify(typeName)}`,
        },
      ]);
    }
  }
  return nxNormalizeRecordInput(input, schema, path);
}

function nxNormalizeRecordInput(input, schema, path) {
  const allowed = schema.fields.map((field) => field.name);
  nxRejectUnknownFields(input, allowed, path);

  const output = {};
  if (schema.record != null) {
    output.$type = schema.record;
  }
  for (const field of schema.fields) {
    const hasField = Object.prototype.hasOwnProperty.call(input, field.name);
    if (hasField) {
      output[field.name] = nxNormalizeValue(input[field.name], field.schema, `${path}.${field.name}`);
    } else if (field.defaultFactory != null) {
      output[field.name] = nxNormalizeValue(field.defaultFactory(output), field.schema, `${path}.${field.name}`);
    } else if (field.hasDefault === true) {
      output[field.name] = field.defaultValue ?? [];
    } else if (field.required) {
      nxMissingField(`${path}.${field.name}`, path);
    }
    // An empty optional field is an omitted key, a `p?:T+` one included, whose read type is a
    // sequence.
    if (
      Object.prototype.hasOwnProperty.call(output, field.name) &&
      nxIsEmpty(output[field.name]) &&
      (field.optional === true || !isSequenceSchema(field.schema))
    ) {
      delete output[field.name];
    }
  }
  return output;
}

/** Whether a schema is a `+` or `*` site, where the empty value is an empty array, not an omitted key. */
function isSequenceSchema(schema) {
  return typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "array");
}

function nxRequireRecordType(input, path) {
  const typeName = input.$type;
  if (typeof typeName === "string") {
    return typeName;
  }
  throw new NxRuntimeError([
    {
      code: "missing-record-type",
      message: `${path} is missing required string field '$type'`,
    },
  ]);
}

export function nxDiagnosticsFromError(error) {
  if (error instanceof NxRuntimeError) {
    return error.diagnostics;
  }
  if (error instanceof Error) {
    return [{ code: "runtime-error", message: error.message }];
  }
  return [{ code: "runtime-error", message: String(error) }];
}

export function nxRuntimeError(message) {
  throw new NxRuntimeError([{ code: "runtime-error", message }]);
}

/** `apply`: a present field replaces the record's, and a present empty one clears it, leaving no key. */
export function nxApplyUpdate(record, update) {
  const { $type: _update, ...fields } = update;
  const output = { ...record };
  for (const [key, value] of Object.entries(fields)) {
    if (nxIsEmpty(value)) {
      delete output[key];
    } else {
      output[key] = value;
    }
  }
  return output;
}

export function nxMergeUpdates(first, second) {
  const { $type: _second, ...later } = second;
  return { ...first, ...later };
}

/** `diff`: the fields that differ, each with its value from `after`, a cleared one as `null`. */
export function nxDiffRecords(before, after) {
  const output = { $type: `${String(before.$type)}.Update` };
  // A field either record leaves out reads as the empty value, so a field only one of them
  // carries still compares.
  for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
    if (key === "$type") {
      continue;
    }
    const next = after[key] ?? [];
    if (!nxValuesEqual(before[key] ?? [], next)) {
      output[key] = nxCleared(next);
    }
  }
  return output;
}

/**
 * Runs `body` once per integer in a range record, in order, and collects the results.
 *
 * The count is computed once, so a closed range ending at the largest exact integer terminates, and
 * the integers themselves are never collected into a list. An empty or reversed range runs the body
 * no times.
 *
 * The range is not checked against the prelude's `$type`, where the IR runtime checks it, because
 * the emitter writes this call only at a site the checker proved to be a range of an integer type.
 * The IR runtime evaluates images it did not emit, so it guards; generated code is its own output.
 */
export function nxRangeMap(range, body) {
  const count =
    range.end > range.start
      ? range.end - range.start + (range.endInclusive ? 1 : 0)
      : range.end === range.start && range.endInclusive
        ? 1
        : 0;
  const results = [];
  for (let offset = 0; offset < count; offset += 1) {
    results.push(body(range.start + offset, offset));
  }
  return results;
}

/**
 * Integer division, which truncates toward zero as the interpreter and the IR runtime's `idiv` do.
 */
export function nxIntDiv(dividend, divisor) {
  return Math.trunc(nxDiv(dividend, divisor)) + 0;
}

/**
 * Division, which fails on a zero divisor as the interpreter and the IR runtime's `div` do, rather
 * than producing JavaScript's `Infinity` or `NaN`.
 */
export function nxDiv(dividend, divisor) {
  if (divisor === 0) {
    nxRuntimeError("Division by zero");
  }
  return dividend / divisor;
}

/**
 * A remainder, which fails on a zero divisor as the interpreter and the IR runtime's `mod` do,
 * rather than producing JavaScript's `NaN`.
 */
export function nxMod(dividend, divisor) {
  if (divisor === 0) {
    nxRuntimeError("Division by zero");
  }
  return (dividend % divisor) + 0;
}

/**
 * The canonical text form of a `float32`, which generated code carries as the `number` it widens
 * to. `String(value)` would print that widening's digits (`0.10000000149011612`); this prints the
 * shortest digits that round-trip to the same `float32` (`0.1`), in the same ECMAScript layout.
 */
export function nxFloat32Text(value) {
  const target = Math.fround(value);
  if (!Number.isFinite(target) || target === 0) {
    return String(target);
  }
  for (let precision = 1; precision <= 9; precision += 1) {
    const candidate = Number(target.toPrecision(precision));
    if (Math.fround(candidate) === target) {
      return String(candidate);
    }
  }
  return String(target);
}

export function nxChangedFields(update, order) {
  if (!Array.isArray(order)) {
    nxRuntimeError("nxChangedFields needs the update record's declared field order");
  }
  const keys = Object.keys(update).filter((key) => key !== "$type");
  const position = (key) => {
    const index = order.indexOf(key);
    return index < 0 ? order.length : index;
  };
  return keys.sort((left, right) => position(left) - position(right));
}

/**
 * `==`: numbers, strings and booleans by value, sequences by their items in order, records by
 * their fields. Every value compares as a sequence, so an item equals a sequence of one holding
 * an equal item, and every spelling of the empty value equals only the empty value.
 */
export function nxValuesEqual(left, right) {
  if (nxIsEmpty(left) || nxIsEmpty(right)) {
    return nxIsEmpty(left) && nxIsEmpty(right);
  }
  if (Array.isArray(left) && Array.isArray(right)) {
    return (
      left.length === right.length &&
      left.every((item, index) => nxValuesEqual(item, right[index] ?? null))
    );
  }
  if (Array.isArray(left)) {
    return left.length === 1 && nxValuesEqual(left[0], right);
  }
  if (Array.isArray(right)) {
    return right.length === 1 && nxValuesEqual(left, right[0]);
  }
  if (left !== null && typeof left === "object") {
    if (right === null || typeof right !== "object") {
      return false;
    }
    const leftKeys = Object.keys(left);
    const rightKeys = Object.keys(right);
    return (
      leftKeys.length === rightKeys.length &&
      leftKeys.every((key) => nxValuesEqual(left[key] ?? null, right[key] ?? null))
    );
  }
  return left === right;
}
"#
    .to_string()
}
