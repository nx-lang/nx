// Environment values: the lookup generated code shares, with the same order and results as the
// IR runtimes. Evaluation is synchronous, so one module-level state is safe: `nxWithEnvironment`
// sets it for a call, and a provide adds layers to it around its body.

export type NxEnvironmentDeclaration<T> = {
  readonly key: string;
  readonly defaultValue: () => T;
  readonly schema: NxSchema;
};

/** One override a provide sets: the declaration, the field path and the value. */
export type NxEnvOverride = readonly [NxEnvironmentDeclaration<any>, readonly string[], unknown];

/** One entry of a component descriptor's `$env`. */
export type NxEnvironmentEntry = { readonly value: string; readonly path: readonly string[]; readonly to: any };

type NxEnvironmentLayer =
  | { readonly set: true; readonly key: string; readonly path: readonly string[]; readonly value: any }
  | { readonly set: false; readonly key: string; readonly index: number };

type NxEnvironmentState = { readonly layers: NxEnvironmentLayer[]; base: number; readonly roots: Map<string, any> };

type NxPendingOverride = { readonly path: readonly string[]; readonly value: any; readonly patch: boolean };

const nxEnvironmentDeclarations = new Map<string, NxEnvironmentDeclaration<any>>();
const nxEnvironmentComponents = new Set<string>();
let nxEnvironmentProvides: readonly (readonly [NxEnvironmentDeclaration<any> | null, readonly string[], () => any])[] = [];
let nxEnvironmentState: NxEnvironmentState = { layers: [], base: 0, roots: new Map() };

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
  return carried.map((item: unknown): NxEnvironmentEntry => {
    const entry = item as any;
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
      nxMalformedEnvironment("an entry is not a record");
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
    if (entry.to === undefined) {
      nxMalformedEnvironment("an entry has no 'to'");
    }
    return { value: entry.value, path, to: entry.to };
  });
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

/**
 * Runs `run` in `environment`, or with no root values when it is `undefined`, and restores the
 * environment around it afterwards. Root values and whole-value overrides are checked against
 * their declared types first.
 */
export function nxWithEnvironment<T>(environment: NxEnvironment | undefined, run: () => T): T {
  const state: NxEnvironmentState = { layers: [], base: 0, roots: new Map() };
  if (environment !== undefined) {
    if (!(environment instanceof NxEnvironment)) {
      throw nxEnvironmentError("nx-ir-options", "The environment option must be an environment made with nxEnvironment(values).");
    }
    for (const [name, value] of environment.roots) {
      const declaration = nxEnvironmentByHostName(name);
      state.roots.set(declaration.key, nxNormalizeValue(value as NxValue, declaration.schema, `environment value '${name}'`));
    }
    for (const entry of environment.entries) {
      const declaration = nxEnvironmentDeclarations.get(entry.value);
      if (declaration === undefined) {
        throw nxEnvironmentError("nx-ir-environment-name", `A '$env' entry names '${entry.value}', which the program does not declare.`);
      }
      const value = entry.path.length === 0 ? nxNormalizeValue(entry.to, declaration.schema, `environment value '${entry.value}'`) : entry.to;
      state.layers.push({ set: true, key: declaration.key, path: entry.path, value });
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

/** `env.name`: the overrides around the read, nearest first, then the root value, the entry module's provides and the default. */
export function nxEnvRead<T>(declaration: NxEnvironmentDeclaration<T>): T {
  const state = nxEnvironmentState;
  const pending: NxPendingOverride[] = [];
  let local = 0;
  let found = false;
  let base: any;
  let cap: number | undefined;
  for (let position = state.layers.length - 1; position >= 0; position -= 1) {
    const layer = state.layers[position]!;
    if (layer.key !== declaration.key) {
      continue;
    }
    if (!layer.set) {
      cap = layer.index;
      break;
    }
    if (layer.path.length === 0) {
      if (position >= state.base) {
        return nxApplyEnvironmentOverrides(layer.value, pending);
      }
      found = true;
      base = layer.value;
      break;
    }
    if (position >= state.base) {
      local += 1;
    }
    pending.push({ path: layer.path, value: layer.value, patch: false });
  }
  if (!found && cap === undefined && state.roots.has(declaration.key)) {
    found = true;
    base = state.roots.get(declaration.key);
  }
  const value = found ? base : nxProvidedOrDefault(state, declaration, cap ?? Infinity);
  const own = pending.splice(0, Math.min(local, pending.length));
  return nxApplyEnvironmentOverrides(nxApplyEnvironmentOverrides(value, pending), own);
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

function nxCappedEnvironment(state: NxEnvironmentState, key: string, index: number, work: () => any): any {
  const depth = state.layers.length;
  state.layers.push({ set: false, key, index });
  try {
    return work();
  } finally {
    state.layers.length = depth;
  }
}

function nxProvidedOrDefault(state: NxEnvironmentState, declaration: NxEnvironmentDeclaration<any>, below: number): any {
  const pending: NxPendingOverride[] = [];
  const provides = nxEnvironmentProvides;
  for (let position = Math.min(below, provides.length) - 1; position >= 0; position -= 1) {
    const [target, path, evaluate] = provides[position]!;
    if (target !== null) {
      if (target.key !== declaration.key) {
        continue;
      }
      const value = nxCappedEnvironment(state, declaration.key, position, evaluate);
      if (path.length === 0) {
        return nxApplyEnvironmentOverrides(value, pending);
      }
      pending.push({ path, value, patch: false });
      continue;
    }
    const bundle = nxCappedEnvironment(state, declaration.key, position, evaluate);
    if (!nxIsRecordValue(bundle) || bundle.$type !== "env.Update" || !Array.isArray(bundle.$entries)) {
      throw nxEnvironmentError("nx-ir-environment", "A bundle provide needs an 'env.Update' value.");
    }
    for (let index = bundle.$entries.length - 1; index >= 0; index -= 1) {
      const entry = bundle.$entries[index];
      if (entry.value !== declaration.key) {
        continue;
      }
      const fields: readonly string[] = Array.isArray(entry.path) ? entry.path : [];
      const patch = nxIsRecordValue(entry.to) && typeof entry.to.$type === "string" && entry.to.$type.endsWith(".Update");
      if (fields.length === 0 && !patch) {
        return nxApplyEnvironmentOverrides(entry.to, pending);
      }
      pending.push({ path: fields, value: entry.to, patch });
    }
  }
  return nxApplyEnvironmentOverrides(declaration.defaultValue(), pending);
}

function nxEnvironmentEntryValues(overrides: readonly NxEnvOverride[]): NxEnvironmentEntry[] {
  return overrides.map(([declaration, path, value]) => ({ value: declaration.key, path: [...path], to: value }));
}

/** `provide overrides { body }`: the overrides, evaluated by the caller, in effect while the body runs, then added to the `$env` of every component descriptor the result reaches. */
export function nxProvide<T>(overrides: readonly NxEnvOverride[], body: () => T): T {
  const state = nxEnvironmentState;
  const depth = state.layers.length;
  for (const [declaration, path, value] of overrides) {
    state.layers.push({ set: true, key: declaration.key, path, value });
  }
  let result: T;
  try {
    result = body();
  } finally {
    state.layers.length = depth;
  }
  return nxStampEnvironment(result, nxEnvironmentEntryValues(overrides)) as T;
}

/** `<env.Update … />`: the overrides as one bundle value. */
export function nxEnvUpdate(overrides: readonly NxEnvOverride[]): any {
  return { $type: "env.Update", $entries: nxEnvironmentEntryValues(overrides) };
}

function nxStampEnvironment(value: unknown, entries: readonly NxEnvironmentEntry[]): unknown {
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
