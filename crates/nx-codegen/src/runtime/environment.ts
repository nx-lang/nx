// Environment values: the lookup generated code shares, with the same order and results as the
// IR runtimes. Evaluation is synchronous, so one module-level state is safe: `nxWithEnvironment`
// sets it for a call, and a provide adds layers to it around its body.
//
// Within one state, derived results are reused by the rule the IR runtimes follow: each layer has
// an identity, and a top-level value thunk, a default, an entry-module provide (a bundle for each
// value read) and a formula a layer holds keep their last result with the token of every value
// their evaluation read, reused while every token matches. A call made outside `nxWithEnvironment`
// runs in the module's own state, whose roots never change and whose layers are numbered for its
// whole life, so what it keeps stays valid from one such call to the next.

export type NxEnvironmentDeclaration<T> = {
  readonly key: string;
  readonly defaultValue: () => T;
  readonly schema: NxSchema;
};

/**
 * A formula: an override's value computed where the environment value is read. It is a function
 * of no arguments that closes over the locals it reads, and holds the module it was written in and
 * those locals as data. It is meaningful only to the generated program that made it.
 */
export type NxFormula = (() => any) & { readonly module: string; readonly locals: Readonly<Record<string, unknown>> };

/**
 * One override a provide or a bundle sets: the declaration, the field path and the value, which is
 * an {@link NxFormula} for an override computed at each read. For a bundle the declaration is
 * `null`, the path is empty and the value is the bundle.
 */
export type NxEnvOverride = readonly [NxEnvironmentDeclaration<any> | null, readonly string[], unknown];

/**
 * One entry of a component descriptor's `$env`, outermost first: an override's value, a formula,
 * or the entries of a bundle applied by `provide env with`.
 */
export type NxEnvironmentEntry =
  | { readonly value: string; readonly path: readonly string[]; readonly to: any }
  | { readonly value: string; readonly path: readonly string[]; readonly formula: NxFormula }
  | { readonly bundle: readonly NxEnvironmentEntry[] };

/** What an override or a bundle entry sets: a value, or a formula. */
type NxEnvironmentSource = { readonly value: any } | { readonly formula: () => any };

/** Where a read continues while a source of the value is evaluated: the sources after it. */
type NxEnvironmentBelow = { readonly layer: number } | { readonly provide: number } | { readonly exhausted: true };

type NxEnvironmentLayer = NxEnvironmentLayerContent & { id?: number };
type NxEnvironmentLayerContent =
  | {
      readonly set: true;
      readonly key: string;
      readonly path: readonly string[];
      readonly source: NxEnvironmentSource;
      /** An entry of a bundle, whose update-record value patches the value rather than replacing it. */
      readonly bundle: boolean;
    }
  | { readonly set: false; readonly key: string; readonly below: NxEnvironmentBelow };

/** The last result of a derived result, with the token of each value its evaluation read. */
type NxKeptResult = { readonly value: any; readonly reads: Map<string, number | undefined> };
type NxEnvironmentState = {
  readonly layers: NxEnvironmentLayer[];
  base: number;
  readonly roots: Map<string, any>;
  nextId: number;
  /** Kept results by name: `p<index>` for a targeted provide, `b<index>:<key>` for a bundle read for a value. */
  readonly derived: Map<string, NxKeptResult>;
  /** Kept results by object: a default's declaration, a value thunk, and the layer that holds a formula. */
  readonly byLayer: WeakMap<object, NxKeptResult>;
  readonly evaluating: { readonly start: number; readonly reads: Map<string, number | undefined> }[];
};

type NxPendingOverride = { readonly path: readonly string[]; readonly value: any; readonly patch: boolean };

/** An evaluated override: the declaration's key, the field path and what it sets. */
type NxEvaluatedOverride = { readonly key: string; readonly path: readonly string[]; readonly source: NxEnvironmentSource };

/** What a provide's or bundle's override gives once evaluated where it is written. */
type NxEvaluatedEntry =
  | { readonly set: NxEvaluatedOverride }
  | { readonly bundle: readonly NxEvaluatedOverride[]; readonly items: readonly any[] };

const nxEnvironmentDeclarations = new Map<string, NxEnvironmentDeclaration<any>>();
const nxEnvironmentComponents = new Set<string>();
let nxEnvironmentProvides: readonly (readonly [NxEnvironmentDeclaration<any> | null, readonly string[], () => any])[] = [];
let nxEnvironmentState: NxEnvironmentState = nxNewEnvironmentState();

/** A state with no layers, roots or kept results. A result kept for an object, such as the layer that holds a formula, is held weakly, so it goes with the object. */
function nxNewEnvironmentState(): NxEnvironmentState {
  return { layers: [], base: 0, roots: new Map(), nextId: 0, derived: new Map(), byLayer: new WeakMap(), evaluating: [] };
}

/** The test-only count of evaluations of defaults, provides, formulas and top-level value thunks; `undefined` while off. */
let nxEvaluations: number | undefined;

/** Turns the test-only evaluation counter on, at zero. Only tests call it. */
export function nxCountEvaluations(): void {
  nxEvaluations = 0;
}

/** The evaluations counted since `nxCountEvaluations`, or `undefined` while the counter is off. */
export function nxEvaluationCount(): number | undefined {
  return nxEvaluations;
}

function nxCountEvaluation(): void {
  if (nxEvaluations !== undefined) {
    nxEvaluations += 1;
  }
}

/** Pushes `layer` with the next identity of the state. */
function nxPushLayer(state: NxEnvironmentState, layer: NxEnvironmentLayer): void {
  layer.id = state.nextId;
  state.nextId += 1;
  state.layers.push(layer);
}

/** The identity of the topmost layer below `start` that sets or caps `key`, or `undefined`. */
function nxEnvironmentToken(state: NxEnvironmentState, key: string, start: number): number | undefined {
  for (let position = Math.min(start, state.layers.length) - 1; position >= 0; position -= 1) {
    const layer = state.layers[position]!;
    if (layer.key === key) {
      return layer.id;
    }
  }
  return undefined;
}

function nxNoteEnvironmentRead(state: NxEnvironmentState, key: string): void {
  for (const recording of state.evaluating) {
    if (!recording.reads.has(key)) {
      recording.reads.set(key, nxEnvironmentToken(state, key, recording.start));
    }
  }
}

/**
 * The kept result of `key` when every value it read has the same token here, or else `work`
 * evaluated, kept and returned. A reused result's reads count as reads of every derived result
 * being evaluated around it, with tokens over the layers below where each began.
 */
function nxDerived(state: NxEnvironmentState, key: string | object, work: () => any): any {
  const last = typeof key === "string" ? state.derived.get(key) : state.byLayer.get(key);
  const start = state.layers.length;
  if (last !== undefined) {
    let holds = true;
    for (const [read, token] of last.reads) {
      if (nxEnvironmentToken(state, read, start) !== token) {
        holds = false;
        break;
      }
    }
    if (holds) {
      for (const read of last.reads.keys()) {
        nxNoteEnvironmentRead(state, read);
      }
      return last.value;
    }
  }
  const recording = { start, reads: new Map<string, number | undefined>() };
  state.evaluating.push(recording);
  let value: any;
  try {
    nxCountEvaluation();
    value = work();
  } finally {
    state.evaluating.pop();
  }
  const kept: NxKeptResult = { value, reads: recording.reads };
  if (typeof key === "string") {
    state.derived.set(key, kept);
  } else {
    state.byLayer.set(key, kept);
  }
  return value;
}

/** A top-level value that reads the environment: computed where it is read, and reused within a call while what it read is unchanged. */
export function nxValueThunk<T>(evaluate: () => T): () => T {
  const thunk = (): T => nxDerived(nxEnvironmentState, thunk, evaluate);
  return thunk;
}

function nxEnvironmentError(code: string, message: string): NxRuntimeError {
  return new NxRuntimeError([{ code, message }]);
}

/** Declares the environment value `key`, `identity::name`, with its default and its schema. */
export function nxEnvironmentDeclaration<T>(key: string, defaultValue: () => T, schema: NxSchema): NxEnvironmentDeclaration<T> {
  const declaration: NxEnvironmentDeclaration<T> = { key, defaultValue, schema };
  nxEnvironmentDeclarations.set(key, declaration);
  return declaration;
}

/** Names the components a provide adds `$env` to the descriptors of. */
export function nxDeclareComponents(names: readonly string[]): void {
  for (const name of names) {
    nxEnvironmentComponents.add(name);
  }
}

/** The entry module's top-level provides, in source order: `[declaration or null, path, value]`. */
export function nxEntryProvides(provides: readonly (readonly [NxEnvironmentDeclaration<any> | null, readonly string[], () => any])[]): void {
  nxEnvironmentProvides = provides;
}

/**
 * The environment a call runs in: root values, then the overrides of each component descriptor
 * down to an instance. `enter` answers a new one.
 */
export class NxEnvironment {
  readonly roots: readonly (readonly [string, unknown])[];
  readonly entries: readonly NxEnvironmentEntry[];

  constructor(roots: readonly (readonly [string, unknown])[], entries: readonly NxEnvironmentEntry[]) {
    this.roots = Object.freeze(roots.slice());
    this.entries = Object.freeze(entries.slice());
    Object.freeze(this);
  }

  enter(descriptor: unknown): NxEnvironment {
    if (typeof descriptor !== "object" || descriptor === null || Array.isArray(descriptor)) {
      return this;
    }
    const entries = nxEnvironmentEntries((descriptor as { readonly $env?: unknown }).$env);
    return entries.length === 0 ? this : new NxEnvironment(this.roots, [...this.entries, ...entries]);
  }
}

/** An environment of root values, named as `colorMode` or `identity::colorMode`. */
export function nxEnvironment(values: Readonly<Record<string, unknown>> = {}): NxEnvironment {
  return new NxEnvironment(Object.entries(values).filter((entry) => entry[1] !== undefined), []);
}

function nxMalformedEnvironment(message: string): never {
  throw nxEnvironmentError("nx-ir-boundary-type", `A component descriptor's '$env' is malformed: ${message}.`);
}

function nxEnvironmentEntries(carried: unknown): NxEnvironmentEntry[] {
  if (carried === undefined || carried === null) {
    return [];
  }
  if (!Array.isArray(carried)) {
    nxMalformedEnvironment("it is not a list");
  }
  return carried.map(nxEnvironmentEntry);
}

function nxEnvironmentEntry(item: unknown): NxEnvironmentEntry {
  const entry = item as any;
  if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
    nxMalformedEnvironment("an entry is not a record");
  }
  if (entry.bundle !== undefined) {
    if (!Array.isArray(entry.bundle)) {
      nxMalformedEnvironment("a 'bundle' entry is not a list");
    }
    return { bundle: entry.bundle.map(nxEnvironmentEntry) };
  }
  if (typeof entry.value !== "string") {
    nxMalformedEnvironment("an entry has no 'value' string");
  }
  let path: string[];
  if (entry.path === undefined || entry.path === null) {
    path = [];
  } else if (typeof entry.path === "string") {
    path = [entry.path];
  } else if (Array.isArray(entry.path)) {
    path = entry.path.map((field: unknown) => {
      if (typeof field !== "string") {
        nxMalformedEnvironment("a 'path' entry is not a string");
      }
      return field as string;
    });
  } else {
    nxMalformedEnvironment("an entry's 'path' is not a list");
  }
  if (entry.formula !== undefined) {
    if (!nxIsFormula(entry.formula)) {
      // A formula another program wrote down is meaningful only to that program.
      throw nxEnvironmentError(
        "nx-ir-boundary-type",
        `A '$env' entry for '${entry.value}' holds a formula that is not one of this generated program's.`,
      );
    }
    return nxFormulaEntry(entry.value, path, entry.formula);
  }
  if (entry.to === undefined) {
    nxMalformedEnvironment("an entry has no 'to' and no 'formula'");
  }
  return { value: entry.value, path, to: entry.to };
}

/** Props without the reserved `$env` field, which is not a prop. */
export function nxWithoutEnvironment(props: NxValue): NxValue {
  if (typeof props !== "object" || props === null || !Object.prototype.hasOwnProperty.call(props, "$env")) {
    return props;
  }
  const { $env: _env, ...rest } = props as Record<string, NxValue>;
  return rest;
}

function nxEnvironmentByHostName(name: string): NxEnvironmentDeclaration<any> {
  const qualified = nxEnvironmentDeclarations.get(name);
  if (qualified !== undefined) {
    return qualified;
  }
  const candidates = [...nxEnvironmentDeclarations.values()].filter((declaration) => declaration.key.endsWith(`::${name}`));
  if (candidates.length === 1) {
    return candidates[0]!;
  }
  if (candidates.length > 1) {
    const modules = candidates.map((candidate) => candidate.key).sort();
    throw nxEnvironmentError(
      "nx-ir-environment-name",
      `Environment value '${name}' is declared more than once in the program (${modules.join(", ")}); name it as 'identity::${name}'.`,
    );
  }
  throw nxEnvironmentError("nx-ir-environment-name", `The program declares no environment value named '${name}'.`);
}

/** The schema of the field `path` names in a value of `schema`, as a `$env` entry's path names it. */
function nxEnvironmentSchemaAt(schema: NxSchema, path: readonly string[]): NxSchema {
  let current = schema;
  for (const field of path) {
    while (typeof current === "object" && "optional" in current) {
      current = current.optional;
    }
    const found = typeof current === "object" && "fields" in current ? current.fields.find((entry) => entry.name === field) : undefined;
    if (found === undefined) {
      throw nxEnvironmentError("nx-ir-environment", `Environment path field '${field}' names no record field.`);
    }
    current = found.schema;
  }
  return current;
}

/** The formulas this generated program made, which a `$env` entry may hold. */
const nxFormulas = new WeakSet<object>();

function nxIsFormula(value: unknown): value is NxFormula {
  return typeof value === "function" && nxFormulas.has(value);
}

/**
 * `value` as a formula of `module` that closes over `locals`: evaluated where the environment
 * value is read. Serializing one throws rather than drop it.
 */
export function nxFormula(evaluate: () => any, module: string, locals: Readonly<Record<string, unknown>>): NxFormula {
  const formula = Object.assign(() => evaluate(), {
    module,
    locals: Object.freeze({ ...locals }),
    toJSON(): never {
      throw nxEnvironmentError("nx-ir-boundary-type", "An environment formula of generated code cannot be serialized.");
    },
  });
  nxFormulas.add(formula);
  return Object.freeze(formula);
}

/** A `$env` entry holding a formula, which throws naming the value when it is serialized. */
function nxFormulaEntry(key: string, path: readonly string[], formula: NxFormula): NxEnvironmentEntry {
  const entry = { value: key, path: [...path], formula };
  Object.defineProperty(entry, "toJSON", {
    enumerable: false,
    value(): never {
      throw nxEnvironmentError(
        "nx-ir-boundary-type",
        `The '$env' entry for environment value '${key}' holds a formula of generated code, which cannot be serialized.`,
      );
    },
  });
  return entry;
}

/** The layers one host `$env` entry gives: one, or one for each entry of a bundle. */
function nxHostEnvironmentLayers(entry: NxEnvironmentEntry, bundle: boolean, layers: NxEnvironmentLayerContent[]): void {
  if ("bundle" in entry) {
    for (const inner of entry.bundle) {
      nxHostEnvironmentLayers(inner, true, layers);
    }
    return;
  }
  const declaration = nxEnvironmentDeclarations.get(entry.value);
  if (declaration === undefined) {
    throw nxEnvironmentError("nx-ir-environment-name", `A '$env' entry names '${entry.value}', which the program does not declare.`);
  }
  let source: NxEnvironmentSource;
  if ("formula" in entry) {
    source = { formula: entry.formula };
  } else if (bundle) {
    // A bundle entry may hold an update record that patches the value, which the value's own type
    // does not describe.
    source = { value: entry.to };
  } else {
    source = {
      value: nxNormalizeValue(
        entry.to,
        nxEnvironmentSchemaAt(declaration.schema, entry.path),
        `environment value '${[entry.value, ...entry.path].join(".")}'`,
      ),
    };
  }
  layers.push({ set: true, key: declaration.key, path: entry.path, source, bundle });
}

/**
 * Runs `run` in `environment`, or with no root values when it is `undefined`, and restores the
 * environment around it afterwards. Root values and overrides are checked against their declared
 * types first.
 */
export function nxWithEnvironment<T>(environment: NxEnvironment | undefined, run: () => T): T {
  const state: NxEnvironmentState = nxNewEnvironmentState();
  if (environment !== undefined) {
    if (!(environment instanceof NxEnvironment)) {
      throw nxEnvironmentError("nx-ir-options", "The environment option must be an environment made with nxEnvironment(values).");
    }
    for (const [name, value] of environment.roots) {
      const declaration = nxEnvironmentByHostName(name);
      state.roots.set(declaration.key, nxNormalizeValue(value as NxValue, declaration.schema, `environment value '${name}'`));
    }
    const layers: NxEnvironmentLayerContent[] = [];
    for (const entry of environment.entries) {
      nxHostEnvironmentLayers(entry, false, layers);
    }
    for (const layer of layers) {
      nxPushLayer(state, layer);
    }
    state.base = state.layers.length;
  }
  const saved = nxEnvironmentState;
  nxEnvironmentState = state;
  try {
    return run();
  } finally {
    nxEnvironmentState = saved;
  }
}

function nxIsUpdateRecord(value: unknown): boolean {
  return nxIsRecordValue(value) && typeof value.$type === "string" && value.$type.endsWith(".Update");
}

function nxShortEnvironmentName(key: string): string {
  const at = key.lastIndexOf("::");
  return at < 0 ? key : key.slice(at + 2);
}

function nxEnvironmentCycle(state: NxEnvironmentState, key: string): never {
  let start = 0;
  for (let position = state.layers.length - 1; position >= 0; position -= 1) {
    const layer = state.layers[position]!;
    if (!layer.set && layer.key === key && "exhausted" in layer.below) {
      start = position;
      break;
    }
  }
  const path = [key];
  for (const layer of state.layers.slice(start)) {
    if (!layer.set && path[path.length - 1] !== layer.key) {
      path.push(layer.key);
    }
  }
  path.push(key);
  const named = path.map((name) => `'${nxShortEnvironmentName(name)}'`).join(" → ");
  throw nxEnvironmentError(
    "nx-ir-environment-cycle",
    `Environment value '${nxShortEnvironmentName(key)}' is read while its own default is being evaluated: ${named}.`,
  );
}

/**
 * What a source gives: its value, or a formula evaluated with reads of `key` capped at `below`. A
 * formula `layer` holds is reused while what it read is unchanged; one of an entry-module bundle is
 * evaluated at each read.
 */
function nxSourceValue(
  state: NxEnvironmentState,
  key: string,
  below: NxEnvironmentBelow,
  source: NxEnvironmentSource,
  layer?: NxEnvironmentLayer,
): any {
  if ("value" in source) {
    return source.value;
  }
  if (layer === undefined) {
    nxCountEvaluation();
    return nxCappedEnvironment(state, key, below, source.formula);
  }
  return nxDerived(state, layer, () => nxCappedEnvironment(state, key, below, source.formula));
}

/**
 * `env.name`: the layers around the read, nearest first, then the root value, the entry module's
 * provides, last first, and the default. A formula, a provide and the default are evaluated at the
 * read, each with a cap that makes a read of the value inside it see only the sources after it.
 */
export function nxEnvRead<T>(declaration: NxEnvironmentDeclaration<T>): T {
  const state = nxEnvironmentState;
  nxNoteEnvironmentRead(state, declaration.key);
  const own: NxPendingOverride[] = [];
  const beneath: NxPendingOverride[] = [];
  let found = false;
  let base: any;
  let providesBelow: number | undefined;
  let position = state.layers.length;
  while (position > 0) {
    position -= 1;
    const layer = state.layers[position]!;
    if (layer.key !== declaration.key) {
      continue;
    }
    if (!layer.set) {
      const below = layer.below;
      if ("layer" in below) {
        position = below.layer;
        continue;
      }
      if ("provide" in below) {
        providesBelow = below.provide;
        break;
      }
      nxEnvironmentCycle(state, declaration.key);
    }
    const isOwn = position >= state.base;
    const value = nxSourceValue(state, declaration.key, { layer: position }, layer.source, layer);
    const patch = layer.bundle && nxIsUpdateRecord(value);
    if (layer.path.length === 0 && !patch) {
      if (isOwn) {
        return nxApplyEnvironmentOverrides(value, own);
      }
      found = true;
      base = value;
      break;
    }
    (isOwn ? own : beneath).push({ path: layer.path, value, patch });
  }
  if (!found && providesBelow === undefined && state.roots.has(declaration.key)) {
    found = true;
    base = state.roots.get(declaration.key);
  }
  const value = found ? base : nxProvidedOrDefault(state, declaration, providesBelow ?? Infinity);
  return nxApplyEnvironmentOverrides(nxApplyEnvironmentOverrides(value, beneath), own);
}

function nxApplyEnvironmentOverrides(base: any, pending: readonly NxPendingOverride[]): any {
  let value = base;
  for (let index = pending.length - 1; index >= 0; index -= 1) {
    const entry = pending[index]!;
    value = nxSetEnvironmentPath(value, entry.path, 0, entry.value, entry.patch);
  }
  return value;
}

function nxIsRecordValue(value: unknown): value is Record<string, any> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nxSetEnvironmentPath(value: any, path: readonly string[], at: number, to: any, patch: boolean): any {
  if (at === path.length) {
    if (!patch) {
      return to;
    }
    if (nxIsRecordValue(value) && nxIsRecordValue(to)) {
      return nxApplyUpdate(value, to);
    }
    throw nxEnvironmentError("nx-ir-environment", "An environment update record applies only to a record value.");
  }
  const field = path[at]!;
  if (!nxIsRecordValue(value)) {
    throw nxEnvironmentError("nx-ir-environment", `Environment override of field '${field}' needs a record value.`);
  }
  const current = Object.prototype.hasOwnProperty.call(value, field) ? value[field] : undefined;
  let next: any;
  if (current !== undefined) {
    next = nxSetEnvironmentPath(current, path, at + 1, to, patch);
  } else if (at + 1 === path.length && !patch) {
    next = to;
  } else {
    throw nxEnvironmentError("nx-ir-environment", `Environment override of field '${field}' found the field empty.`);
  }
  return { ...value, [field]: next };
}

/** Runs `work` with a read of `key` seeing only the sources `below` gives. */
function nxCappedEnvironment(state: NxEnvironmentState, key: string, below: NxEnvironmentBelow, work: () => any): any {
  const depth = state.layers.length;
  nxPushLayer(state, { set: false, key, below });
  try {
    return work();
  } finally {
    state.layers.length = depth;
  }
}

/** The `$entries` of a bundle value. */
function nxBundleItems(bundle: unknown): readonly any[] {
  if (!nxIsRecordValue(bundle) || bundle.$type !== "env.Update" || !Array.isArray(bundle.$entries)) {
    throw nxEnvironmentError("nx-ir-environment", "A bundle needs an 'env.Update' value.");
  }
  return bundle.$entries;
}

/** The entries of a bundle's `$entries`, in order, a bundle entry's own entries in its place. */
function nxBundleEntries(items: readonly any[], out: NxEvaluatedOverride[] = []): NxEvaluatedOverride[] {
  for (const entry of items) {
    if (nxIsRecordValue(entry) && entry.bundle !== undefined) {
      if (!Array.isArray(entry.bundle)) {
        throw nxEnvironmentError("nx-ir-environment", "An 'env.Update' entry is malformed.");
      }
      nxBundleEntries(entry.bundle, out);
      continue;
    }
    if (!nxIsRecordValue(entry) || typeof entry.value !== "string" || (entry.to === undefined) === (entry.formula === undefined)) {
      throw nxEnvironmentError("nx-ir-environment", "An 'env.Update' entry is malformed.");
    }
    if (entry.formula !== undefined && !nxIsFormula(entry.formula)) {
      throw nxEnvironmentError(
        "nx-ir-boundary-type",
        `An 'env.Update' entry for '${entry.value}' holds a formula that is not one of this generated program's.`,
      );
    }
    const path: readonly string[] = Array.isArray(entry.path) ? entry.path : [];
    out.push({
      key: entry.value,
      path,
      source: entry.formula !== undefined ? { formula: entry.formula } : { value: entry.to },
    });
  }
  return out;
}

function nxProvidedOrDefault(state: NxEnvironmentState, declaration: NxEnvironmentDeclaration<any>, below: number): any {
  const pending: NxPendingOverride[] = [];
  const provides = nxEnvironmentProvides;
  for (let position = Math.min(below, provides.length) - 1; position >= 0; position -= 1) {
    const [target, path, evaluate] = provides[position]!;
    const cap: NxEnvironmentBelow = { provide: position };
    if (target !== null) {
      if (target.key !== declaration.key) {
        continue;
      }
      const value = nxDerived(state, `p${position}`, () => nxCappedEnvironment(state, declaration.key, cap, evaluate));
      if (path.length === 0) {
        return nxApplyEnvironmentOverrides(value, pending);
      }
      pending.push({ path, value, patch: false });
      continue;
    }
    const bundle = nxDerived(state, `b${position}:${declaration.key}`, () => nxCappedEnvironment(state, declaration.key, cap, evaluate));
    const entries = nxBundleEntries(nxBundleItems(bundle));
    for (let index = entries.length - 1; index >= 0; index -= 1) {
      const entry = entries[index]!;
      if (entry.key !== declaration.key) {
        continue;
      }
      const value = nxSourceValue(state, declaration.key, cap, entry.source);
      const patch = nxIsUpdateRecord(value);
      if (entry.path.length === 0 && !patch) {
        return nxApplyEnvironmentOverrides(value, pending);
      }
      pending.push({ path: entry.path, value, patch });
    }
  }
  const value = nxDerived(state, declaration, () => nxCappedEnvironment(state, declaration.key, { exhausted: true }, declaration.defaultValue));
  return nxApplyEnvironmentOverrides(value, pending);
}

/** The overrides of a provide or bundle, as the caller evaluated them. */
function nxEvaluatedEntries(overrides: readonly NxEnvOverride[]): NxEvaluatedEntry[] {
  return overrides.map(([declaration, path, value]): NxEvaluatedEntry => {
    if (declaration === null) {
      const items = nxBundleItems(value);
      return { bundle: nxBundleEntries(items), items };
    }
    const source: NxEnvironmentSource = nxIsFormula(value) ? { formula: value } : { value };
    return { set: { key: declaration.key, path, source } };
  });
}

/**
 * The `$env` or `$entries` entries for evaluated overrides. In `$env` a bundle is one `{bundle}`
 * entry; in a bundle's `$entries` its entries take its place.
 */
function nxEnvironmentEntryValues(evaluated: readonly NxEvaluatedEntry[], nest: boolean): any[] {
  const out: any[] = [];
  for (const item of evaluated) {
    if ("set" in item) {
      const { key, path, source } = item.set;
      out.push("value" in source ? { value: key, path: [...path], to: source.value } : nxFormulaEntry(key, path, source.formula as NxFormula));
    } else if (nest) {
      out.push({ bundle: [...item.items] });
    } else {
      out.push(...item.items);
    }
  }
  return out;
}

/** `provide overrides { body }`: the overrides, evaluated by the caller, in effect while the body runs, then added to the `$env` of every component descriptor the result reaches. */
export function nxProvide<T>(overrides: readonly NxEnvOverride[], body: () => T): T {
  const evaluated = nxEvaluatedEntries(overrides);
  const state = nxEnvironmentState;
  const depth = state.layers.length;
  for (const item of evaluated) {
    if ("set" in item) {
      nxPushLayer(state, { set: true, key: item.set.key, path: item.set.path, source: item.set.source, bundle: false });
    } else {
      for (const { key, path, source } of item.bundle) {
        nxPushLayer(state, { set: true, key, path, source, bundle: true });
      }
    }
  }
  let result: T;
  try {
    result = body();
  } finally {
    state.layers.length = depth;
  }
  return nxStampEnvironment(result, nxEnvironmentEntryValues(evaluated, true)) as T;
}

/** `<env.Update … />` or `base with { … }`: the base's entries, then the overrides, as one bundle value. */
export function nxEnvUpdate(overrides: readonly NxEnvOverride[], base?: unknown): any {
  const entries = base === undefined ? [] : [...nxBundleItems(base)];
  entries.push(...nxEnvironmentEntryValues(nxEvaluatedEntries(overrides), false));
  return { $type: "env.Update", $entries: entries };
}

function nxStampEnvironment(value: unknown, entries: readonly unknown[]): unknown {
  if (entries.length === 0) {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map((item) => nxStampEnvironment(item, entries));
  }
  if (!nxIsRecordValue(value) || Object.getPrototypeOf(value) !== Object.prototype) {
    return value;
  }
  if (typeof value.$type === "string" && nxEnvironmentComponents.has(value.$type)) {
    const existing = Array.isArray(value.$env) ? value.$env : [];
    return { ...value, $env: [...entries, ...existing] };
  }
  const output: Record<string, unknown> = {};
  for (const [key, field] of Object.entries(value)) {
    output[key] = nxStampEnvironment(field, entries);
  }
  return output;
}
