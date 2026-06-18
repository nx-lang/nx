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
  | "number"
  | "string"
  | { readonly array: NxSchema }
  | { readonly nullable: NxSchema }
  | { readonly enum: readonly string[] }
  | NxRecordSchema
  | { readonly union: readonly NxRecordSchema[] };

export type NxFieldSchema = {
  readonly name: string;
  readonly schema: NxSchema;
  readonly required: boolean;
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
export const nxNumberSchema: NxSchema = "number";
export const nxStringSchema: NxSchema = "string";

export function nxArraySchema(element: NxSchema): NxSchema {
  return { array: element };
}

export function nxNullableSchema(inner: NxSchema): NxSchema {
  return { nullable: inner };
}

export function nxEnumSchema(members: readonly string[]): NxSchema {
  return { enum: members };
}

export function nxUnionSchema(cases: readonly NxRecordSchema[]): NxSchema {
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
  if (typeof schema === "object" && "nullable" in schema) {
    if (value === null) {
      return null;
    }
    return nxNormalizeValue(value, schema.nullable, path);
  }
  if (typeof schema === "object" && "array" in schema) {
    if (!Array.isArray(value)) {
      throw new NxRuntimeError([
        {
          code: "invalid-array",
          message: `${path} expected an array`,
        },
      ]);
    }
    return value.map((element, index) => nxNormalizeValue(element, schema.array, `${path}[${index}]`));
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
    const caseSchema = schema.union.find((candidate) => candidate.record === typeName);
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
      output[field.name] = field.defaultValue ?? null;
    } else if (field.required) {
      nxMissingField(`${path}.${field.name}`, path);
    }
  }
  return output;
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
export const nxNumberSchema = "number";
export const nxStringSchema = "string";

export function nxArraySchema(element) {
  return { array: element };
}

export function nxNullableSchema(inner) {
  return { nullable: inner };
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
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "nullable")) {
    if (value === null) {
      return null;
    }
    return nxNormalizeValue(value, schema.nullable, path);
  }
  if (typeof schema === "object" && Object.prototype.hasOwnProperty.call(schema, "array")) {
    if (!Array.isArray(value)) {
      throw new NxRuntimeError([
        {
          code: "invalid-array",
          message: `${path} expected an array`,
        },
      ]);
    }
    return value.map((element, index) => nxNormalizeValue(element, schema.array, `${path}[${index}]`));
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
    const caseSchema = schema.union.find((candidate) => candidate.record === typeName);
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
      output[field.name] = field.defaultValue ?? null;
    } else if (field.required) {
      nxMissingField(`${path}.${field.name}`, path);
    }
  }
  return output;
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
"#
    .to_string()
}
