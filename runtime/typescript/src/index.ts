export const NX_IR_FORMAT_ID = "nx-ir-json";
export const NX_IR_SCHEMA_VERSION = 2;
export const NX_IR_RUNTIME_ABI = "nx-ir-runtime-v1";

export type NxDiagnosticSeverity = "error" | "warning" | "info" | "hint";

export interface NxIrDiagnostic {
  readonly severity: NxDiagnosticSeverity;
  readonly code: string;
  readonly message: string;
  readonly path?: string;
  readonly source?: NxIrSourceSpan;
}

export type NxResult<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly diagnostics: readonly NxIrDiagnostic[] };

export type NxCanonicalValue =
  | null
  | boolean
  | number
  | string
  | readonly NxCanonicalValue[]
  | { readonly [key: string]: NxCanonicalValue };

export interface NxIrProgram {
  readonly format: string;
  readonly schemaVersion: number;
  readonly runtimeAbi: string;
  readonly programFingerprint: string;
  readonly requiredFeatures: readonly string[];
  readonly functionEntrypoints: readonly NxIrEntrypoint[];
  readonly componentEntrypoints: readonly NxIrEntrypoint[];
  readonly modules: readonly NxIrModule[];
  readonly sources: readonly NxIrSourceEntry[];
}

export interface NxIrEntrypoint {
  readonly name: string;
  readonly reference: NxIrReference;
}

export interface NxIrModule {
  readonly id: string;
  readonly runtimeId: number;
  readonly provenance: { readonly kind: string; readonly [key: string]: unknown };
  readonly imports: readonly NxIrReference[];
  readonly declarations: readonly NxIrDeclaration[];
}

export interface NxIrReference {
  readonly module: string;
  readonly declaration: string;
  readonly name: string;
  readonly kind: string;
}

export interface NxIrDeclaration {
  readonly id: string;
  readonly reference: NxIrReference;
  readonly span: NxIrSourceSpan;
  readonly kind: NxIrDeclarationKind;
}

export type NxIrDeclarationKind =
  | NxIrFunctionDeclaration
  | NxIrValueDeclaration
  | NxIrRecordDeclaration
  | NxIrComponentDeclaration
  | NxIrUnionDeclaration
  | NxIrTypeAliasDeclaration;

export interface NxIrFunctionDeclaration {
  readonly tag: "function";
  readonly params: readonly NxIrParam[];
  readonly body: NxIrExpression;
  readonly returnType?: NxIrSemanticType;
}

export interface NxIrValueDeclaration {
  readonly tag: "value";
  readonly value: NxIrExpression;
  readonly ty?: NxIrSemanticType;
}

export interface NxIrRecordDeclaration {
  readonly tag: "record";
  readonly fields: readonly NxIrRecordField[];
  /**
   * The record's abstract bases, nearest first.
   *
   * Fields arrive already flattened, so this answers only what flattening cannot: a value stamped
   * with this record's name is acceptable wherever any of these is expected.
   */
  readonly bases?: readonly NxIrReference[];
  /**
   * Whether the record was declared `abstract`, and so has no values of its own.
   *
   * A base-typed site accepts a value of a record that extends this one, never one of this one.
   */
  readonly isAbstract?: boolean;
}

export interface NxIrComponentDeclaration {
  readonly tag: "component";
  readonly isAbstract: boolean;
  readonly isExternal: boolean;
  readonly props: readonly NxIrComponentField[];
  readonly state: readonly NxIrComponentField[];
  readonly body?: NxIrExpression | null;
}

export interface NxIrUnionDeclaration {
  readonly tag: "union";
  readonly cases: readonly NxIrUnionCase[];
  /** The union's abstract bases, nearest first, inherited by every case. */
  readonly bases?: readonly NxIrReference[];
}

export interface NxIrTypeAliasDeclaration {
  readonly tag: "typeAlias";
}

export interface NxIrParam {
  readonly name: string;
  readonly slot: string;
  readonly ty: NxIrTypeRef;
  readonly isContent: boolean;
  readonly span: NxIrSourceSpan;
}

export interface NxIrRecordField {
  readonly name: string;
  readonly slot: string;
  readonly ty: NxIrTypeRef;
  readonly isContent: boolean;
  readonly isRequired: boolean;
  readonly default?: NxIrExpression | null;
  readonly span: NxIrSourceSpan;
}

export interface NxIrComponentField extends NxIrRecordField {
  readonly ownerModule: string;
}

export interface NxIrUnionCase {
  readonly name: string;
  readonly fields: readonly NxIrRecordField[];
  /**
   * Whether this case declares no fields in a union that declares no base.
   *
   * A constant case carries nothing beyond its own name, so its wire form is that bare string
   * rather than a `$type` object.
   */
  readonly isConstant: boolean;
  readonly span: NxIrSourceSpan;
}

export interface NxIrExpression {
  readonly id: string;
  readonly span: NxIrSourceSpan;
  readonly ty?: NxIrSemanticType;
  readonly op: { readonly tag: string; readonly [key: string]: unknown };
}

export interface NxIrTypeRef {
  readonly kind: string;
  readonly name?: string;
  readonly reference?: NxIrReference;
  readonly display?: string;
  readonly element?: NxIrTypeRef;
  readonly inner?: NxIrTypeRef;
  readonly params?: readonly NxIrTypeRef[];
  readonly returnType?: NxIrTypeRef;
}

export interface NxIrSemanticType {
  readonly display: string;
  readonly shape: { readonly kind: string; readonly [key: string]: unknown };
}

export interface NxIrSourceSpan {
  readonly source?: string;
  readonly start: number;
  readonly end: number;
}

export interface NxIrSourceEntry {
  readonly identity: string;
  readonly source: string;
}

export interface NxPreparedProgram {
  readonly ir: NxIrProgram;
  readonly modulesById: ReadonlyMap<string, NxIrModule>;
  readonly declarationsById: ReadonlyMap<string, PreparedDeclaration>;
  readonly functionEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
  readonly componentEntrypoints: ReadonlyMap<string, PreparedDeclaration>;
  readonly sourcesByIdentity: ReadonlyMap<string, string>;
  /**
   * Every constructible nominal shape, keyed by the `$type` a value of it carries.
   *
   * A value arriving at a base-typed boundary names its own type and nothing more, so this is how
   * that name is turned back into the schema to normalize it with. One key can hold several shapes:
   * two modules may each declare a record of the same name.
   */
  readonly nominalShapesByDiscriminator: ReadonlyMap<string, readonly NominalShape[]>;
}

/**
 * One record or union case as it appears on the wire.
 *
 * <para>`bases` holds declaration ids rather than names because that is the only identity that
 * survives separate modules: two records named `Card` are two types, and only the id says which
 * one a base-typed site meant.</para>
 */
export interface NominalShape {
  /** The `$type` a value of this shape carries: a record's name, or `Union.case`. */
  readonly discriminator: string;
  readonly declaration: string;
  readonly fields: readonly NxIrRecordField[];
  readonly bases: readonly string[];
  /** Whether this shape is an abstract record, which no value may be an instance of. */
  readonly isAbstract: boolean;
}

export interface PreparedDeclaration {
  readonly module: NxIrModule;
  readonly declaration: NxIrDeclaration;
}

export interface NxRuntimeOptions {
  readonly maxCallDepth?: number;
}

export interface ComponentInitResult {
  readonly rendered: NxCanonicalValue;
  readonly state: Record<string, NxCanonicalValue>;
}

export interface ComponentEvaluateResult {
  readonly rendered: NxCanonicalValue;
}

export class NxIrRuntimeError extends Error {
  public readonly diagnostics: readonly NxIrDiagnostic[];

  public constructor(diagnostics: readonly NxIrDiagnostic[]) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join("; "));
    this.name = "NxIrRuntimeError";
    this.diagnostics = diagnostics;
  }
}

const knownFeatures = new Set(["eager-v1"]);
const knownExpressionTags = new Set([
  "literal",
  "slot",
  "reference",
  "binary",
  "unary",
  "call",
  "if",
  "ifIs",
  "let",
  "block",
  "array",
  "for",
  "index",
  "member",
  "record",
  "unionCase",
  "intrinsicElement",
  "componentDescriptor",
]);

export function prepareNxIrProgram(input: string | NxIrProgram): NxPreparedProgram {
  const result = tryPrepareNxIrProgram(input);
  if (!result.ok) {
    throw new NxIrRuntimeError(result.diagnostics);
  }

  return result.value;
}

export function tryPrepareNxIrProgram(input: string | NxIrProgram): NxResult<NxPreparedProgram> {
  const diagnostics: NxIrDiagnostic[] = [];
  const ir = typeof input === "string" ? parseIrJson(input, diagnostics) : input;
  if (ir === undefined) {
    return { ok: false, diagnostics };
  }

  if (ir.format !== NX_IR_FORMAT_ID) {
    diagnostics.push(diagnostic("nx-ir-format", `Unsupported NX IR format '${ir.format}'.`));
  }
  if (ir.schemaVersion !== NX_IR_SCHEMA_VERSION) {
    diagnostics.push(
      diagnostic(
        "nx-ir-schema-version",
        `Unsupported NX IR schema version '${ir.schemaVersion}'.`,
      ),
    );
  }
  if (ir.runtimeAbi !== NX_IR_RUNTIME_ABI) {
    diagnostics.push(
      diagnostic(
        "nx-ir-runtime-abi",
        `Unsupported NX IR runtime ABI '${ir.runtimeAbi}'.`,
      ),
    );
  }
  for (const feature of ir.requiredFeatures ?? []) {
    if (!knownFeatures.has(feature)) {
      diagnostics.push(
        diagnostic("nx-ir-required-feature", `Unsupported NX IR required feature '${feature}'.`),
      );
    }
  }

  const modulesById = new Map<string, NxIrModule>();
  const declarationsById = new Map<string, PreparedDeclaration>();
  const functionEntrypoints = new Map<string, PreparedDeclaration>();
  const componentEntrypoints = new Map<string, PreparedDeclaration>();
  const sourcesByIdentity = new Map<string, string>();

  for (const source of ir.sources ?? []) {
    sourcesByIdentity.set(source.identity, source.source);
  }
  for (const module of ir.modules ?? []) {
    if (modulesById.has(module.id)) {
      diagnostics.push(diagnostic("nx-ir-duplicate-module", `Duplicate module '${module.id}'.`));
    }
    modulesById.set(module.id, module);
  }
  for (const module of ir.modules ?? []) {
    for (const declaration of module.declarations ?? []) {
      const prepared = { module, declaration };
      if (declarationsById.has(declaration.id)) {
        diagnostics.push(
          diagnostic("nx-ir-duplicate-declaration", `Duplicate declaration '${declaration.id}'.`),
        );
      }
      declarationsById.set(declaration.id, prepared);
    }
  }
  for (const module of ir.modules ?? []) {
    for (const declaration of module.declarations ?? []) {
      validateDeclaration(module, declaration, declarationsById, diagnostics);
    }
  }

  const nominalShapesByDiscriminator = indexNominalShapes(ir);
  for (const entrypoint of ir.functionEntrypoints ?? []) {
    const declaration = declarationsById.get(entrypoint.reference.declaration);
    if (declaration === undefined || declaration.declaration.kind.tag !== "function") {
      diagnostics.push(
        diagnostic("nx-ir-entrypoint", `Function entrypoint '${entrypoint.name}' is invalid.`),
      );
    } else {
      functionEntrypoints.set(entrypoint.name, declaration);
    }
  }
  for (const entrypoint of ir.componentEntrypoints ?? []) {
    const declaration = declarationsById.get(entrypoint.reference.declaration);
    if (declaration === undefined || declaration.declaration.kind.tag !== "component") {
      diagnostics.push(
        diagnostic("nx-ir-entrypoint", `Component entrypoint '${entrypoint.name}' is invalid.`),
      );
    } else {
      componentEntrypoints.set(entrypoint.name, declaration);
    }
  }

  if (diagnostics.length > 0) {
    return { ok: false, diagnostics };
  }

  return {
    ok: true,
    value: {
      ir,
      modulesById,
      declarationsById,
      functionEntrypoints,
      componentEntrypoints,
      sourcesByIdentity,
      nominalShapesByDiscriminator,
    },
  };
}

/**
 * Indexes every nominal shape by the `$type` its values carry.
 *
 * An abstract record is indexed too, even though nothing may be an instance of one: a value that
 * names one is a value to reject, and rejecting it by name reads better than reporting it as a type
 * the program does not have.
 *
 * A record contributes its own name; a union contributes one entry per non-constant case, under
 * `Union.case`, because that is what `evalUnionCase` stamps. A constant case is a bare string with
 * no schema to normalize, so it contributes nothing.
 */
function indexNominalShapes(ir: NxIrProgram): ReadonlyMap<string, readonly NominalShape[]> {
  const index = new Map<string, NominalShape[]>();
  const add = (shape: NominalShape): void => {
    const existing = index.get(shape.discriminator);
    if (existing === undefined) {
      index.set(shape.discriminator, [shape]);
      return;
    }
    existing.push(shape);
  };

  for (const module of ir.modules ?? []) {
    for (const declaration of module.declarations ?? []) {
      const kind = declaration.kind;
      if (kind.tag === "record") {
        add({
          discriminator: declaration.reference.name,
          declaration: declaration.id,
          fields: kind.fields ?? [],
          bases: (kind.bases ?? []).map((base) => base.declaration),
          isAbstract: kind.isAbstract === true,
        });
        continue;
      }
      if (kind.tag === "union") {
        const bases = (kind.bases ?? []).map((base) => base.declaration);
        for (const unionCase of kind.cases ?? []) {
          if (unionCase.isConstant) {
            continue;
          }
          add({
            discriminator: `${declaration.reference.name}.${unionCase.name}`,
            declaration: declaration.id,
            fields: unionCase.fields ?? [],
            bases,
            isAbstract: false,
          });
        }
      }
    }
  }

  return index;
}

export function evaluateFunction(
  program: NxPreparedProgram,
  name: string,
  args: readonly NxCanonicalValue[] = [],
  options: NxRuntimeOptions = {},
): NxCanonicalValue {
  const prepared = program.functionEntrypoints.get(name);
  if (prepared === undefined || prepared.declaration.kind.tag !== "function") {
    fail("nx-ir-missing-entrypoint", `Function entrypoint '${name}' was not found.`);
  }

  return invokeFunction(program, prepared, args, options, 0);
}

export function constructComponentDescriptor(
  program: NxPreparedProgram,
  name: string,
  props: Record<string, NxCanonicalValue> = {},
  content: readonly NxCanonicalValue[] = [],
): NxCanonicalValue {
  const prepared = componentDeclaration(program, name);
  const component = prepared.declaration.kind as NxIrComponentDeclaration;
  const input = { ...props };
  const contentField = component.props.find((field) => field.isContent);
  applyContentBinding(input, contentField?.name, component.props, content, name);
  const normalizedProps = normalizeFields(
    program,
    component.props,
    input,
    new Map(),
    `${name} props`,
    false,
  );

  return { $type: prepared.declaration.reference.name, ...normalizedProps };
}

export function initializeComponent(
  program: NxPreparedProgram,
  name: string,
  props: Record<string, NxCanonicalValue> = {},
  options: NxRuntimeOptions = {},
): ComponentInitResult {
  const prepared = componentDeclaration(program, name);
  const component = prepared.declaration.kind as NxIrComponentDeclaration;
  if (component.isAbstract || component.body === undefined || component.body === null) {
    fail("nx-ir-component", `Component '${name}' cannot be initialized because it has no body.`);
  }

  const env = new Map<string, NxCanonicalValue>();
  const normalizedProps = normalizeFields(program, component.props, props, env, `${name} props`, false);
  const state = normalizeFields(program, component.state, {}, env, `${name} state`, false);
  const rendered = evalExpression(component.body, {
    program,
    env,
    options,
    depth: 0,
  });

  return { rendered, state };
}

export function evaluateComponent(
  program: NxPreparedProgram,
  name: string,
  props: Record<string, NxCanonicalValue>,
  state: Record<string, NxCanonicalValue>,
  options: NxRuntimeOptions = {},
): ComponentEvaluateResult {
  const prepared = componentDeclaration(program, name);
  const component = prepared.declaration.kind as NxIrComponentDeclaration;
  if (component.isAbstract || component.body === undefined || component.body === null) {
    fail("nx-ir-component", `Component '${name}' cannot be evaluated because it has no body.`);
  }

  const env = new Map<string, NxCanonicalValue>();
  const normalizedProps = normalizeFields(program, component.props, props, env, `${name} props`, false);
  for (const field of component.props) {
    env.set(field.slot, normalizedProps[field.name] ?? null);
  }
  const normalizedState = normalizeFields(program, component.state, state, env, `${name} state`, true);
  for (const field of component.state) {
    env.set(field.slot, normalizedState[field.name] ?? null);
  }

  return {
    rendered: evalExpression(component.body, {
      program,
      env,
      options,
      depth: 0,
    }),
  };
}

export function normalizeComponentState(
  program: NxPreparedProgram,
  name: string,
  state: Record<string, NxCanonicalValue>,
): Record<string, NxCanonicalValue> {
  const prepared = componentDeclaration(program, name);
  const component = prepared.declaration.kind as NxIrComponentDeclaration;
  return normalizeFields(program, component.state, state, new Map(), `${name} state`, true);
}

export function applyComponentStatePatch(
  program: NxPreparedProgram,
  name: string,
  currentState: Record<string, NxCanonicalValue>,
  patch: Record<string, NxCanonicalValue>,
): Record<string, NxCanonicalValue> {
  const prepared = componentDeclaration(program, name);
  const component = prepared.declaration.kind as NxIrComponentDeclaration;
  const known = new Set(component.state.map((field) => field.name));
  for (const key of Object.keys(patch)) {
    if (!known.has(key)) {
      fail("nx-ir-state-field", `Unknown ${name} state field '${key}'.`);
    }
  }

  return normalizeFields(
    program,
    component.state,
    { ...currentState, ...patch },
    new Map(),
    `${name} state`,
    true,
  );
}

interface EvalContext {
  readonly program: NxPreparedProgram;
  readonly env: Map<string, NxCanonicalValue>;
  readonly options: NxRuntimeOptions;
  readonly depth: number;
}

interface FunctionReferenceValue {
  readonly $nxKind: "functionReference";
  readonly reference: NxIrReference;
}

function invokeFunction(
  program: NxPreparedProgram,
  prepared: PreparedDeclaration,
  args: readonly NxCanonicalValue[],
  options: NxRuntimeOptions,
  depth: number,
): NxCanonicalValue {
  const maxCallDepth = options.maxCallDepth ?? 100;
  if (depth > maxCallDepth) {
    fail("nx-ir-resource-limit", `Maximum NX IR call depth ${maxCallDepth} was exceeded.`);
  }

  const kind = prepared.declaration.kind;
  if (kind.tag !== "function") {
    fail("nx-ir-call", `'${prepared.declaration.reference.name}' is not a function.`);
  }
  if (args.length !== kind.params.length) {
    fail(
      "nx-ir-arguments",
      `Function '${prepared.declaration.reference.name}' expected ${kind.params.length} arguments, got ${args.length}.`,
    );
  }

  const env = new Map<string, NxCanonicalValue>();
  for (let index = 0; index < kind.params.length; index += 1) {
    const param = kind.params[index]!;
    env.set(param.slot, normalizeValue(program, param.ty, args[index]!, `${param.name}`));
  }

  return evalExpression(kind.body, { program, env, options, depth });
}

function evalExpression(expression: NxIrExpression, context: EvalContext): NxCanonicalValue {
  const op = expression.op as Record<string, unknown>;
  switch (op.tag) {
    case "literal":
      return evalLiteral(op.value);
    case "slot":
      return readSlot(context, String(op.slot), String(op.name));
    case "reference":
      return evalReference(context, op.reference as NxIrReference);
    case "binary":
      return evalBinary(
        expression,
        String(op.operator),
        evalExpression(op.lhs as NxIrExpression, context),
        evalExpression(op.rhs as NxIrExpression, context),
      );
    case "unary":
      return evalUnary(String(op.operator), evalExpression(op.expr as NxIrExpression, context));
    case "call":
      return evalCall(expression, context);
    case "if":
      return truthy(evalExpression(op.condition as NxIrExpression, context))
        ? evalExpression(op.thenBranch as NxIrExpression, context)
        : op.elseBranch === undefined || op.elseBranch === null
          ? null
          : evalExpression(op.elseBranch as NxIrExpression, context);
    case "ifIs":
      return evalIfIs(op, context);
    case "let":
      return evalLet(op, context);
    case "block":
      return evalBlock(op, context);
    case "array":
      return ((op.elements as readonly NxIrExpression[]) ?? []).map((item) =>
        evalExpression(item, context),
      );
    case "for":
      return evalFor(op, context);
    case "index":
      return evalIndex(
        evalExpression(op.base as NxIrExpression, context),
        evalExpression(op.index as NxIrExpression, context),
        expression.span,
      );
    case "member":
      return evalMember(evalExpression(op.base as NxIrExpression, context), String(op.member));
    case "record":
      return evalRecord(op, context);
    case "unionCase":
      return evalUnionCase(op, context);
    case "intrinsicElement":
      return evalIntrinsicElement(op, context);
    case "componentDescriptor":
      return evalComponentDescriptor(op, context);
    default:
      fail("nx-ir-expression", `Unknown NX IR expression tag '${String(op.tag)}'.`, expression.span);
  }
}

function evalLiteral(value: unknown): NxCanonicalValue {
  const literal = value as Record<string, unknown>;
  switch (literal.kind) {
    case "string":
      return String(literal.value);
    case "int":
      if (typeof literal.number === "number") {
        return literal.number;
      }
      return { $type: "nx.int", value: String(literal.value) };
    case "float":
      return Number(literal.value);
    case "boolean":
      return Boolean(literal.value);
    case "null":
      return null;
    default:
      fail("nx-ir-literal", `Unknown literal kind '${String(literal.kind)}'.`);
  }
}

function evalReference(context: EvalContext, reference: NxIrReference): NxCanonicalValue {
  if (reference.kind === "function") {
    return { $nxKind: "functionReference", reference } as unknown as NxCanonicalValue;
  }
  const prepared = context.program.declarationsById.get(reference.declaration);
  if (prepared === undefined) {
    fail("nx-ir-reference", `Missing declaration '${reference.declaration}'.`);
  }
  if (prepared.declaration.kind.tag === "value") {
    return evalExpression(prepared.declaration.kind.value, context);
  }

  fail("nx-ir-reference", `Declaration '${reference.name}' cannot be used as a value.`);
}

function evalCall(expression: NxIrExpression, context: EvalContext): NxCanonicalValue {
  const op = expression.op as Record<string, unknown>;
  const callee = evalExpression(op.callee as NxIrExpression, context) as unknown;
  if (!isFunctionReference(callee)) {
    fail("nx-ir-call", "NX IR call callee did not evaluate to a function reference.", expression.span);
  }
  const prepared = context.program.declarationsById.get(callee.reference.declaration);
  if (prepared === undefined) {
    fail("nx-ir-call", `Missing function declaration '${callee.reference.declaration}'.`);
  }
  const args = ((op.args as readonly NxIrExpression[]) ?? []).map((arg) =>
    evalExpression(arg, context),
  );

  return invokeFunction(context.program, prepared, args, context.options, context.depth + 1);
}

function evalIfIs(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const scrutinee = evalExpression(op.scrutinee as NxIrExpression, context);
  for (const arm of (op.arms as readonly Record<string, unknown>[]) ?? []) {
    const patterns = (arm.patterns as readonly NxIrExpression[]) ?? [];
    if (patterns.some((pattern) => patternMatches(scrutinee, evalExpression(pattern, context)))) {
      return evalExpression(arm.body as NxIrExpression, context);
    }
  }

  return op.elseBranch === undefined || op.elseBranch === null
    ? null
    : evalExpression(op.elseBranch as NxIrExpression, context);
}

function evalLet(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const nextEnv = new Map(context.env);
  nextEnv.set(String(op.slot), evalExpression(op.value as NxIrExpression, context));
  return evalExpression(op.body as NxIrExpression, { ...context, env: nextEnv });
}

function evalBlock(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const env = new Map(context.env);
  const blockContext = { ...context, env };
  for (const statement of (op.statements as readonly Record<string, unknown>[]) ?? []) {
    if (statement.tag === "let") {
      env.set(String(statement.slot), evalExpression(statement.init as NxIrExpression, blockContext));
    } else if (statement.tag === "expr") {
      evalExpression(statement.expr as NxIrExpression, blockContext);
    }
  }

  return op.expression === undefined || op.expression === null
    ? null
    : evalExpression(op.expression as NxIrExpression, blockContext);
}

function evalFor(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const iterable = evalExpression(op.iterable as NxIrExpression, context);
  if (!Array.isArray(iterable)) {
    fail("nx-ir-for", "For expression iterable must evaluate to an array.");
  }

  return iterable.map((item, index) => {
    const env = new Map(context.env);
    env.set(String(op.itemSlot), item);
    if (typeof op.indexSlot === "string") {
      env.set(op.indexSlot, index);
    }

    return evalExpression(op.body as NxIrExpression, { ...context, env });
  });
}

function evalRecord(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const properties = propertiesObject(op.properties as readonly Record<string, unknown>[], context);
  const content = ((op.content as readonly NxIrExpression[]) ?? []).map((item) =>
    evalExpression(item, context),
  );
  const fields = (op.fields as readonly NxIrRecordField[]) ?? [];
  applyContentBinding(properties, op.contentField, fields, content, String(op.name));
  const normalized = normalizeFields(context.program, fields, properties, new Map(context.env), String(op.name), false);
  return { $type: String(op.name), ...normalized };
}

function evalUnionCase(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const union = op.union as NxIrReference;
  const caseName = String(op.caseName);
  const properties = propertiesObject(op.properties as readonly Record<string, unknown>[], context);
  const content = ((op.content as readonly NxIrExpression[]) ?? []).map((item) =>
    evalExpression(item, context),
  );
  const fields = (op.fields as readonly NxIrRecordField[]) ?? [];
  applyContentBinding(properties, op.contentField, fields, content, `${union.name}.${caseName}`);
  const normalized = normalizeFields(
    context.program,
    fields,
    properties,
    new Map(context.env),
    `${union.name}.${caseName}`,
    false,
  );

  // A constant case carries nothing beyond its own name.
  if (op.isConstant === true) {
    return caseName;
  }

  return { $type: `${union.name}.${caseName}`, ...normalized };
}

function evalIntrinsicElement(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const properties = propertiesObject(op.properties as readonly Record<string, unknown>[], context);
  const content = ((op.content as readonly NxIrExpression[]) ?? []).map((item) =>
    evalExpression(item, context),
  );
  if (content.length > 0) {
    properties.children = content;
  }

  return { $type: String(op.tagName), ...properties };
}

function evalComponentDescriptor(op: Record<string, unknown>, context: EvalContext): NxCanonicalValue {
  const reference = op.component as NxIrReference;
  const prepared = context.program.declarationsById.get(reference.declaration);
  if (prepared === undefined || prepared.declaration.kind.tag !== "component") {
    fail("nx-ir-component", `Missing component declaration '${reference.name}'.`);
  }
  const component = prepared.declaration.kind;
  const props = propertiesObject(op.properties as readonly Record<string, unknown>[], context);
  const content = ((op.content as readonly NxIrExpression[]) ?? []).map((item) =>
    evalExpression(item, context),
  );
  applyContentBinding(props, op.contentField, component.props, content, reference.name);
  const env = new Map(context.env);
  const normalized = normalizeFields(context.program, component.props, props, env, `${reference.name} props`, false);

  return { $type: reference.name, ...normalized };
}

function applyContentBinding(
  input: Record<string, NxCanonicalValue>,
  contentField: unknown,
  fields: readonly NxIrRecordField[],
  content: readonly NxCanonicalValue[],
  path: string,
): void {
  if (content.length === 0) {
    return;
  }
  if (typeof contentField !== "string") {
    fail("nx-ir-boundary-field", `${path} does not accept content.`);
  }
  if (Object.prototype.hasOwnProperty.call(input, contentField)) {
    fail("nx-ir-boundary-field", `${path} field '${contentField}' was supplied both as a property and as content.`);
  }

  const declared = fields.find((field) => field.name === contentField)?.ty;
  const bindsList = declared !== undefined && isListTypeRef(declared);
  input[contentField] = bindsList || content.length > 1 ? [...content] : content[0]!;
}

/**
 * Whether a content property's declared type holds a list, looking through nullability.
 *
 * A list-typed content property binds a list however many children were supplied, including exactly
 * one. Collapsing a single child to the child itself would then fail normalization, and it would
 * disagree with the interpreter, which lists the single child.
 */
function isListTypeRef(ty: NxIrTypeRef): boolean {
  if (ty.kind === "nullable") {
    return ty.inner !== undefined && isListTypeRef(ty.inner);
  }
  return ty.kind === "array";
}

function normalizeFields(
  program: NxPreparedProgram,
  fields: readonly NxIrRecordField[],
  input: Record<string, NxCanonicalValue>,
  env: Map<string, NxCanonicalValue>,
  path: string,
  requireExplicit: boolean,
): Record<string, NxCanonicalValue> {
  const known = new Set(fields.map((field) => field.name));
  for (const key of Object.keys(input)) {
    if (!known.has(key)) {
      fail("nx-ir-boundary-field", `Unknown ${path} field '${key}'.`);
    }
  }

  const output: Record<string, NxCanonicalValue> = {};
  for (const field of fields) {
    let value: NxCanonicalValue;
    if (Object.prototype.hasOwnProperty.call(input, field.name)) {
      value = normalizeValue(program, field.ty, input[field.name]!, `${path}.${field.name}`);
    } else if (!requireExplicit && field.default !== undefined && field.default !== null) {
      value = evalExpression(field.default, {
        program,
        env,
        options: {},
        depth: 0,
      });
      value = normalizeValue(program, field.ty, value, `${path}.${field.name}`);
    } else if (!field.isRequired && !requireExplicit) {
      value = null;
    } else {
      fail("nx-ir-boundary-field", `Missing required ${path} field '${field.name}'.`, field.span);
    }
    output[field.name] = value;
    env.set(field.slot, value);
  }

  return output;
}

function normalizeValue(
  program: NxPreparedProgram,
  ty: NxIrTypeRef,
  value: NxCanonicalValue,
  path: string,
): NxCanonicalValue {
  switch (ty.kind) {
    case "primitive":
      return normalizePrimitiveValue(String(ty.name), value, path);
    case "nominal":
      return normalizeNominalValue(
        program,
        requiredReference(ty.reference, "nominal type"),
        String(ty.display ?? ty.reference?.name ?? "nominal"),
        value,
        path,
      );
    case "array": {
      // A single value at a list-typed site is a list of one. That is the language's rule, not a
      // leniency: `Shadows={ <SkiaShadow /> }` and `xs={3.0}` both evaluate to one-element lists
      // under the interpreter, and the IR records the value at its own type rather than wrapping
      // it, leaving the coercion to normalization.
      const items = Array.isArray(value) ? value : [value];
      return items.map((item, index) =>
        normalizeValue(program, requiredTypeRef(ty.element, "array element"), item, `${path}[${index}]`),
      );
    }
    case "nullable":
      return value === null
        ? null
        : normalizeValue(program, requiredTypeRef(ty.inner, "nullable inner"), value, path);
    case "function":
      return value;
    default:
      fail("nx-ir-schema", `Unknown type reference kind '${ty.kind}'.`);
  }
}

function normalizePrimitiveValue(
  name: string,
  value: NxCanonicalValue,
  path: string,
): NxCanonicalValue {
  switch (name) {
    case "int":
    case "int32":
    case "int64":
    case "float32":
    case "float64":
      if (typeof value !== "number") {
        fail("nx-ir-boundary-type", `Expected ${path} to be a number.`);
      }
      return value;
    case "string":
      if (typeof value !== "string") {
        fail("nx-ir-boundary-type", `Expected ${path} to be a string.`);
      }
      return value;
    case "boolean":
      if (typeof value !== "boolean") {
        fail("nx-ir-boundary-type", `Expected ${path} to be a boolean.`);
      }
      return value;
    case "object":
      return value;
    default:
      fail("nx-ir-schema", `Unknown primitive type '${name}'.`);
  }
}

function normalizeNominalValue(
  program: NxPreparedProgram,
  reference: NxIrReference,
  display: string,
  value: NxCanonicalValue,
  path: string,
): NxCanonicalValue {
  const prepared = program.declarationsById.get(reference.declaration);
  if (prepared === undefined) {
    fail("nx-ir-schema", `Missing nominal type declaration '${reference.declaration}'.`);
  }
  const kind = prepared.declaration.kind;
  if (kind.tag === "record") {
    const object = requireObject(value, path);
    // The declared type supplies the field list, so a discriminator carried by the value selects
    // nothing and is dropped rather than used. Record construction stamps one on every value it
    // produces, so keeping it would make the runtime reject its own output.
    //
    // It is still checked before it is dropped, because this path also takes host input: a
    // discriminator naming some other type is a value of the wrong type, and silently restamping it
    // with the declared name would report it as the type it is not. Absent is fine — a host writing
    // a plain object has no discriminator to give.
    const discriminator = object.$type;
    if (discriminator !== undefined && discriminator !== display) {
      // A derived value is not the wrong type: `User extends Base` is acceptable wherever `Base`
      // is. It keeps its own discriminator and is normalized against its own schema, because the
      // expected type's field list does not have the derived fields in it.
      const subtype = resolveSubtype(
        program,
        String(discriminator),
        reference.declaration,
        display,
        path,
      );
      const { $type: _derived, ...derived } = object;
      return {
        $type: subtype.discriminator,
        ...normalizeFields(program, subtype.fields, derived, new Map(), path, false),
      };
    }
    // Nothing is an instance of an abstract record. Reaching here means the value would be stamped
    // with this declaration's own name, and analysis rejects that spelling in NX source — so
    // accepting it from a host would hand back a value no NX program can produce and no consumer
    // branching on `$type` has a case for.
    if (kind.isAbstract === true) {
      fail(
        "nx-ir-boundary-type",
        discriminator === undefined
          ? `Expected ${path} to be a concrete type extending ${display}, got an object with no ` +
              `'$type' discriminator naming one.`
          : `Expected ${path} to be a concrete type extending ${display}, got abstract '${display}'.`,
      );
    }
    const { $type: _discard, ...rest } = object;
    return { $type: display, ...normalizeFields(program, kind.fields, rest, new Map(), path, false) };
  }
  if (kind.tag === "union") {
    // A constant case arrives as its bare name rather than as a `$type` object.
    if (typeof value === "string") {
      const constantCase = kind.cases.find((item) => item.name === value && item.isConstant);
      if (constantCase === undefined) {
        fail("nx-ir-boundary-type", `Invalid constant union case for ${path}: '${value}'.`);
      }
      return value;
    }

    const object = requireObject(value, path);
    const typeName = object.$type;
    if (typeof typeName !== "string") {
      fail("nx-ir-boundary-type", `Expected ${path} to include a '$type' discriminator.`);
    }
    const prefix = `${display}.`;
    if (!typeName.startsWith(prefix)) {
      fail("nx-ir-boundary-type", `Expected ${path} to be a ${display} union case.`);
    }
    const caseName = typeName.slice(prefix.length);
    const unionCase = kind.cases.find((item) => item.name === caseName);
    if (unionCase === undefined) {
      fail("nx-ir-boundary-type", `Invalid union case '${typeName}' for ${path}.`);
    }
    const { $type: _discard, ...rest } = object;
    return {
      $type: typeName,
      ...normalizeFields(program, unionCase.fields, rest, new Map(), path, false),
    };
  }

  return value;
}

/**
 * Finds the shape a value's `$type` names, given that a value of `expected` was asked for.
 *
 * <para>The discriminator is a name, not an identity, so this can find more than one shape — two
 * modules may each declare a `Card` extending the same base. That is reported rather than guessed
 * at: picking one would normalize against the wrong field list and quietly produce a wrong value.
 * Carrying identity in the value itself is what would remove the ambiguity, and it is not carried
 * because `$type` is output a host reads and the interpreter emits the same names.</para>
 */
function resolveSubtype(
  program: NxPreparedProgram,
  discriminator: string,
  expected: string,
  display: string,
  path: string,
): NominalShape {
  const candidates = (program.nominalShapesByDiscriminator.get(discriminator) ?? []).filter(
    (shape) => shape.bases.includes(expected),
  );
  if (candidates.length > 1) {
    fail(
      "nx-ir-boundary-type",
      `Ambiguous subtype at ${path}: ${candidates.length} declarations named '${discriminator}' ` +
        `extend ${display}, and a '$type' discriminator cannot tell them apart.`,
    );
  }
  const only = candidates[0];
  if (only === undefined) {
    fail("nx-ir-boundary-type", `Expected ${path} to be a ${display}, got '${discriminator}'.`);
  }
  if (only.isAbstract) {
    // An intermediate abstract record extends the expected one, so it passes the base check, but it
    // is still a type with no values.
    fail(
      "nx-ir-boundary-type",
      `Expected ${path} to be a concrete type extending ${display}, got abstract '${discriminator}'.`,
    );
  }
  return only;
}

function propertiesObject(
  properties: readonly Record<string, unknown>[],
  context: EvalContext,
): Record<string, NxCanonicalValue> {
  const output: Record<string, NxCanonicalValue> = {};
  for (const property of properties ?? []) {
    output[String(property.name)] = evalExpression(property.value as NxIrExpression, context);
  }
  return output;
}

function componentDeclaration(program: NxPreparedProgram, name: string): PreparedDeclaration {
  const prepared = program.componentEntrypoints.get(name);
  if (prepared === undefined || prepared.declaration.kind.tag !== "component") {
    fail("nx-ir-component", `Component '${name}' was not found.`);
  }

  return prepared;
}

function validateDeclaration(
  module: NxIrModule,
  declaration: NxIrDeclaration,
  declarationsById: ReadonlyMap<string, PreparedDeclaration>,
  diagnostics: NxIrDiagnostic[],
): void {
  const kind = declaration.kind;
  switch (kind.tag) {
    case "function":
      for (const param of kind.params) {
        validateTypeRef(param.ty, declarationsById, diagnostics, param.span);
      }
      validateExpression(kind.body, diagnostics);
      break;
    case "value":
      validateExpression(kind.value, diagnostics);
      break;
    case "record":
      validateFields(kind.fields, declarationsById, diagnostics);
      break;
    case "component":
      validateFields(kind.props, declarationsById, diagnostics);
      validateFields(kind.state, declarationsById, diagnostics);
      if (kind.body !== undefined && kind.body !== null) {
        validateExpression(kind.body, diagnostics);
      }
      break;
    case "union":
      for (const item of kind.cases) {
        validateFields(item.fields, declarationsById, diagnostics);
      }
      break;
    case "typeAlias":
      break;
    default:
      diagnostics.push(
        diagnostic(
          "nx-ir-declaration",
          `Unknown declaration tag '${String((kind as { tag?: unknown }).tag)}' in module '${module.id}'.`,
          declaration.span,
        ),
      );
  }
}

function validateFields(
  fields: readonly {
    readonly ty: NxIrTypeRef;
    readonly default?: NxIrExpression | null;
    readonly span: NxIrSourceSpan;
  }[],
  declarationsById: ReadonlyMap<string, PreparedDeclaration>,
  diagnostics: NxIrDiagnostic[],
): void {
  for (const field of fields) {
    validateTypeRef(field.ty, declarationsById, diagnostics, field.span);
    if (field.default !== undefined && field.default !== null) {
      validateExpression(field.default, diagnostics);
    }
  }
}

function validateTypeRef(
  ty: NxIrTypeRef,
  declarationsById: ReadonlyMap<string, PreparedDeclaration>,
  diagnostics: NxIrDiagnostic[],
  source?: NxIrSourceSpan,
): void {
  switch (ty.kind) {
    case "primitive":
      if (typeof ty.name !== "string") {
        diagnostics.push(diagnostic("nx-ir-type-ref", "Primitive type reference is missing a name.", source));
      }
      break;
    case "nominal": {
      const reference = ty.reference;
      if (reference === undefined) {
        diagnostics.push(
          diagnostic("nx-ir-type-ref", "Nominal type reference is missing a declaration reference.", source),
        );
        break;
      }
      if (!declarationsById.has(reference.declaration)) {
        diagnostics.push(
          diagnostic(
            "nx-ir-type-ref",
            `Nominal type reference '${reference.name}' points at missing declaration '${reference.declaration}'.`,
            source,
          ),
        );
      }
      break;
    }
    case "array":
      if (ty.element === undefined) {
        diagnostics.push(diagnostic("nx-ir-type-ref", "Array type reference is missing an element type.", source));
      } else {
        validateTypeRef(ty.element, declarationsById, diagnostics, source);
      }
      break;
    case "nullable":
      if (ty.inner === undefined) {
        diagnostics.push(diagnostic("nx-ir-type-ref", "Nullable type reference is missing an inner type.", source));
      } else {
        validateTypeRef(ty.inner, declarationsById, diagnostics, source);
      }
      break;
    case "function":
      for (const param of ty.params ?? []) {
        validateTypeRef(param, declarationsById, diagnostics, source);
      }
      if (ty.returnType === undefined) {
        diagnostics.push(diagnostic("nx-ir-type-ref", "Function type reference is missing a return type.", source));
      } else {
        validateTypeRef(ty.returnType, declarationsById, diagnostics, source);
      }
      break;
    default:
      diagnostics.push(diagnostic("nx-ir-type-ref", `Unknown type reference kind '${ty.kind}'.`, source));
      break;
  }
}

function validateExpression(expression: NxIrExpression, diagnostics: NxIrDiagnostic[]): void {
  const op = expression.op as Record<string, unknown>;
  if (!knownExpressionTags.has(String(op.tag))) {
    diagnostics.push(
      diagnostic("nx-ir-expression", `Unknown expression tag '${String(op.tag)}'.`, expression.span),
    );
    return;
  }

  for (const child of childExpressions(op)) {
    validateExpression(child, diagnostics);
  }
}

function childExpressions(op: Record<string, unknown>): NxIrExpression[] {
  const output: NxIrExpression[] = [];
  const add = (value: unknown): void => {
    if (value !== undefined && value !== null) {
      output.push(value as NxIrExpression);
    }
  };
  const addMany = (value: unknown): void => {
    for (const item of (value as readonly NxIrExpression[] | undefined) ?? []) {
      add(item);
    }
  };
  switch (op.tag) {
    case "binary":
      add(op.lhs);
      add(op.rhs);
      break;
    case "unary":
      add(op.expr);
      break;
    case "call":
      add(op.callee);
      addMany(op.args);
      break;
    case "if":
      add(op.condition);
      add(op.thenBranch);
      add(op.elseBranch);
      break;
    case "ifIs":
      add(op.scrutinee);
      for (const arm of (op.arms as readonly Record<string, unknown>[] | undefined) ?? []) {
        addMany(arm.patterns);
        add(arm.body);
      }
      add(op.elseBranch);
      break;
    case "let":
      add(op.value);
      add(op.body);
      break;
    case "block":
      for (const statement of (op.statements as readonly Record<string, unknown>[] | undefined) ?? []) {
        add(statement.init);
        add(statement.expr);
      }
      add(op.expression);
      break;
    case "array":
      addMany(op.elements);
      break;
    case "for":
      add(op.iterable);
      add(op.body);
      break;
    case "index":
      add(op.base);
      add(op.index);
      break;
    case "member":
      add(op.base);
      break;
    case "record":
    case "unionCase":
    case "componentDescriptor":
    case "intrinsicElement":
      for (const property of (op.properties as readonly Record<string, unknown>[] | undefined) ?? []) {
        add(property.value);
      }
      addMany(op.content);
      for (const field of (op.fields as readonly { readonly default?: NxIrExpression | null }[] | undefined) ?? []) {
        add(field.default);
      }
      break;
  }
  return output;
}

function evalBinary(
  expression: NxIrExpression,
  operator: string,
  lhs: NxCanonicalValue,
  rhs: NxCanonicalValue,
): NxCanonicalValue {
  switch (operator) {
    case "add":
      return checkedNumber(lhs, operator, expression.span) + checkedNumber(rhs, operator, expression.span);
    case "sub":
      return checkedNumber(lhs, operator, expression.span) - checkedNumber(rhs, operator, expression.span);
    case "mul":
      return checkedNumber(lhs, operator, expression.span) * checkedNumber(rhs, operator, expression.span);
    case "div":
      return evalDivision(lhs, rhs, expression.ty, expression.span);
    case "mod":
      return evalModulo(lhs, rhs, expression.ty, expression.span);
    case "concat":
      return String(lhs) + String(rhs);
    case "eq":
      return deepEqual(lhs, rhs);
    case "ne":
      return !deepEqual(lhs, rhs);
    case "lt":
      return checkedNumber(lhs, operator, expression.span) < checkedNumber(rhs, operator, expression.span);
    case "le":
      return checkedNumber(lhs, operator, expression.span) <= checkedNumber(rhs, operator, expression.span);
    case "gt":
      return checkedNumber(lhs, operator, expression.span) > checkedNumber(rhs, operator, expression.span);
    case "ge":
      return checkedNumber(lhs, operator, expression.span) >= checkedNumber(rhs, operator, expression.span);
    case "and":
      return truthy(lhs) && truthy(rhs);
    case "or":
      return truthy(lhs) || truthy(rhs);
    default:
      fail("nx-ir-operator", `Unknown binary operator '${operator}'.`);
  }
}

function evalDivision(
  lhs: NxCanonicalValue,
  rhs: NxCanonicalValue,
  ty: NxIrSemanticType | undefined,
  source: NxIrSourceSpan,
): NxCanonicalValue {
  const lhsNumber = checkedNumber(lhs, "div", source);
  const rhsNumber = checkedNumber(rhs, "div", source);
  if (rhsNumber === 0) {
    fail("nx-ir-division-by-zero", "Division by zero.", source);
  }
  if (isIntegerSemanticType(ty)) {
    checkedInteger(lhsNumber, "div", source);
    checkedInteger(rhsNumber, "div", source);
    return normalizeSignedZero(Math.trunc(lhsNumber / rhsNumber));
  }
  return lhsNumber / rhsNumber;
}

function evalModulo(
  lhs: NxCanonicalValue,
  rhs: NxCanonicalValue,
  ty: NxIrSemanticType | undefined,
  source: NxIrSourceSpan,
): NxCanonicalValue {
  const lhsNumber = checkedNumber(lhs, "mod", source);
  const rhsNumber = checkedNumber(rhs, "mod", source);
  if (rhsNumber === 0) {
    fail("nx-ir-division-by-zero", "Division by zero.", source);
  }
  if (isIntegerSemanticType(ty)) {
    checkedInteger(lhsNumber, "mod", source);
    checkedInteger(rhsNumber, "mod", source);
  }
  return normalizeSignedZero(lhsNumber % rhsNumber);
}

function evalUnary(operator: string, value: NxCanonicalValue): NxCanonicalValue {
  switch (operator) {
    case "neg":
      return -checkedNumber(value, operator);
    case "not":
      return !truthy(value);
    default:
      fail("nx-ir-operator", `Unknown unary operator '${operator}'.`);
  }
}

function readSlot(context: EvalContext, slot: string, name: string): NxCanonicalValue {
  if (!context.env.has(slot)) {
    fail("nx-ir-slot", `Local slot '${name}' was not bound.`);
  }
  return context.env.get(slot)!;
}

function evalIndex(
  base: NxCanonicalValue,
  index: NxCanonicalValue,
  source?: NxIrSourceSpan,
): NxCanonicalValue {
  if (!Array.isArray(base)) {
    fail("nx-ir-index", "Index expression requires an array.", source);
  }
  if (typeof index !== "number" || !Number.isInteger(index)) {
    fail("nx-ir-index", "Index expression requires an integer index.", source);
  }
  if (index < 0 || index >= base.length) {
    fail(
      "nx-ir-index-bounds",
      `Array index ${index} is out of bounds for length ${base.length}.`,
      source,
    );
  }
  return base[index]!;
}

function evalMember(base: NxCanonicalValue, member: string): NxCanonicalValue {
  const object = requireObject(base, "member access");
  if (!Object.prototype.hasOwnProperty.call(object, member)) {
    fail("nx-ir-member", `Object does not contain member '${member}'.`);
  }
  return object[member]!;
}

function parseIrJson(source: string, diagnostics: NxIrDiagnostic[]): NxIrProgram | undefined {
  try {
    return JSON.parse(source) as NxIrProgram;
  } catch (error) {
    diagnostics.push(diagnostic("nx-ir-json", `Invalid NX IR JSON: ${String(error)}.`));
    return undefined;
  }
}

function requiredTypeRef(value: NxIrTypeRef | undefined, context: string): NxIrTypeRef {
  if (value === undefined) {
    fail("nx-ir-schema", `Missing ${context} type reference.`);
  }
  return value;
}

function requiredReference(value: NxIrReference | undefined, context: string): NxIrReference {
  if (value === undefined) {
    fail("nx-ir-schema", `Missing ${context} reference.`);
  }
  return value;
}

function isFunctionReference(value: unknown): value is FunctionReferenceValue {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { readonly $nxKind?: unknown }).$nxKind === "functionReference"
  );
}

function checkedNumber(value: NxCanonicalValue, operation: string, source?: NxIrSourceSpan): number {
  if (typeof value !== "number") {
    fail("nx-ir-number", `Operator '${operation}' requires JavaScript-safe numeric values.`, source);
  }
  return value;
}

function checkedInteger(value: number, operation: string, source?: NxIrSourceSpan): void {
  if (!Number.isInteger(value)) {
    fail("nx-ir-number", `Operator '${operation}' requires integer operands for integer results.`, source);
  }
}

function isIntegerSemanticType(ty: NxIrSemanticType | undefined): boolean {
  if (ty?.shape.kind !== "primitive") {
    return false;
  }
  const name = ty.shape.name;
  return name === "int" || name === "int32" || name === "int64";
}

function normalizeSignedZero(value: number): number {
  return Object.is(value, -0) ? 0 : value;
}

function truthy(value: NxCanonicalValue): boolean {
  return Boolean(value);
}

function patternMatches(value: NxCanonicalValue, pattern: NxCanonicalValue): boolean {
  if (isObject(value) && isObject(pattern) && typeof pattern.$type === "string") {
    return value.$type === pattern.$type;
  }
  return deepEqual(value, pattern);
}

function deepEqual(lhs: NxCanonicalValue, rhs: NxCanonicalValue): boolean {
  return JSON.stringify(lhs) === JSON.stringify(rhs);
}

function requireObject(value: NxCanonicalValue, path: string): Record<string, NxCanonicalValue> {
  if (!isObject(value) || Array.isArray(value)) {
    fail("nx-ir-boundary-type", `Expected ${path} to be an object.`);
  }
  return value as Record<string, NxCanonicalValue>;
}

function isObject(value: unknown): value is Record<string, NxCanonicalValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function fail(code: string, message: string, source?: NxIrSourceSpan): never {
  throw new NxIrRuntimeError([diagnostic(code, message, source)]);
}

function diagnostic(code: string, message: string, source?: NxIrSourceSpan): NxIrDiagnostic {
  return {
    severity: "error",
    code,
    message,
    ...(source === undefined ? {} : { source }),
  };
}
