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

const nxEnvironmentDeclarations = new Map();

const nxEnvironmentComponents = new Set();

let nxEnvironmentProvides = [];

let nxEnvironmentState = nxNewEnvironmentState();

/** A state with no layers, roots or kept results. A result kept for an object, such as the layer that holds a formula, is held weakly in `byLayer`, so it goes with the object. */

function nxNewEnvironmentState() {
  return { layers: [], base: 0, roots: new Map(), nextId: 0, derived: new Map(), byLayer: new WeakMap(), evaluating: [] };
}

/** The test-only count of evaluations of defaults, provides, formulas and top-level value thunks; `undefined` while off. */

let nxEvaluations;

/** Turns the test-only evaluation counter on, at zero. Only tests call it. */

export function nxCountEvaluations() {
  nxEvaluations = 0;
}

/** The evaluations counted since `nxCountEvaluations`, or `undefined` while the counter is off. */

export function nxEvaluationCount() {
  return nxEvaluations;
}

function nxCountEvaluation() {
  if (nxEvaluations !== undefined) {
    nxEvaluations += 1;
  }
}

/** Pushes `layer` with the next identity of the state. */

function nxPushLayer(state, layer) {
  layer.id = state.nextId;
  state.nextId += 1;
  state.layers.push(layer);
}

/** The identity of the topmost layer below `start` that sets or caps `key`, or `undefined`. */

function nxEnvironmentToken(state, key, start) {
  for (let position = Math.min(start, state.layers.length) - 1; position >= 0; position -= 1) {
    const layer = state.layers[position];
    if (layer.key === key) {
      return layer.id;
    }
  }
  return undefined;
}

function nxNoteEnvironmentRead(state, key) {
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

function nxDerived(state, key, work) {
  const kept = typeof key === "string" ? state.derived : state.byLayer;
  const start = state.layers.length;
  const last = kept.get(key);
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
  const recording = { start, reads: new Map() };
  state.evaluating.push(recording);
  let value;
  try {
    nxCountEvaluation();
    value = work();
  }
  finally {
    state.evaluating.pop();
  }
  kept.set(key, { value, reads: recording.reads });
  return value;
}

/** A top-level value that reads the environment: computed where it is read, and reused within a call while what it read is unchanged. */

export function nxValueThunk(evaluate) {
  const thunk = () => nxDerived(nxEnvironmentState, thunk, evaluate);
  return thunk;
}

function nxEnvironmentError(code, message) {
  return new NxRuntimeError([{ code, message }]);
}

/** Declares the environment value `key`, `identity::name`, with its default and its schema. */

export function nxEnvironmentDeclaration(key, defaultValue, schema) {
  const declaration = { key, defaultValue, schema };
  nxEnvironmentDeclarations.set(key, declaration);
  return declaration;
}

/** Names the components a provide adds `$env` to the descriptors of. */

export function nxDeclareComponents(names) {
  for (const name of names) {
    nxEnvironmentComponents.add(name);
  }
}

/** The entry module's top-level provides, in source order: `[declaration or null, path, value]`. */

export function nxEntryProvides(provides) {
  nxEnvironmentProvides = provides;
}

/**
 * The environment a call runs in: root values, then the overrides of each component descriptor
 * down to an instance. `enter` answers a new one.
 */

export class NxEnvironment {
  roots;
  entries;
  constructor(roots, entries) {
    this.roots = Object.freeze(roots.slice());
    this.entries = Object.freeze(entries.slice());
    Object.freeze(this);
  }
  enter(descriptor) {
    if (typeof descriptor !== "object" || descriptor === null || Array.isArray(descriptor)) {
      return this;
    }
    const entries = nxEnvironmentEntries(descriptor.$env);
    return entries.length === 0 ? this : new NxEnvironment(this.roots, [...this.entries, ...entries]);
  }
}

/** An environment of root values, named as `colorMode` or `identity::colorMode`. */

export function nxEnvironment(values = {}) {
  return new NxEnvironment(Object.entries(values).filter((entry) => entry[1] !== undefined), []);
}

function nxMalformedEnvironment(message) {
  throw nxEnvironmentError("nx-ir-boundary-type", `A component descriptor's '$env' is malformed: ${message}.`);
}

function nxEnvironmentEntries(carried) {
  if (carried === undefined || carried === null) {
    return [];
  }
  if (!Array.isArray(carried)) {
    nxMalformedEnvironment("it is not a list");
  }
  return carried.map(nxEnvironmentEntry);
}

function nxEnvironmentEntry(item) {
  const entry = item;
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
  let path;
  if (entry.path === undefined || entry.path === null) {
    path = [];
  }
  else if (typeof entry.path === "string") {
    path = [entry.path];
  }
  else if (Array.isArray(entry.path)) {
    path = entry.path.map((field) => {
      if (typeof field !== "string") {
        nxMalformedEnvironment("a 'path' entry is not a string");
      }
      return field;
    });
  }
  else {
    nxMalformedEnvironment("an entry's 'path' is not a list");
  }
  if (entry.formula !== undefined) {
    if (!nxIsFormula(entry.formula)) {
      // A formula another program wrote down is meaningful only to that program.
      throw nxEnvironmentError("nx-ir-boundary-type", `A '$env' entry for '${entry.value}' holds a formula that is not one of this generated program's.`);
    }
    return nxFormulaEntry(entry.value, path, entry.formula);
  }
  if (entry.to === undefined) {
    nxMalformedEnvironment("an entry has no 'to' and no 'formula'");
  }
  return { value: entry.value, path, to: entry.to };
}

/** Props without the reserved `$env` field, which is not a prop. */

export function nxWithoutEnvironment(props) {
  if (typeof props !== "object" || props === null || !Object.prototype.hasOwnProperty.call(props, "$env")) {
    return props;
  }
  const { $env: _env, ...rest } = props;
  return rest;
}

function nxEnvironmentByHostName(name) {
  const qualified = nxEnvironmentDeclarations.get(name);
  if (qualified !== undefined) {
    return qualified;
  }
  const candidates = [...nxEnvironmentDeclarations.values()].filter((declaration) => declaration.key.endsWith(`::${name}`));
  if (candidates.length === 1) {
    return candidates[0];
  }
  if (candidates.length > 1) {
    const modules = candidates.map((candidate) => candidate.key).sort();
    throw nxEnvironmentError("nx-ir-environment-name", `Environment value '${name}' is declared more than once in the program (${modules.join(", ")}); name it as 'identity::${name}'.`);
  }
  throw nxEnvironmentError("nx-ir-environment-name", `The program declares no environment value named '${name}'.`);
}

/** The schema of the field `path` names in a value of `schema`, as a `$env` entry's path names it. */

function nxEnvironmentSchemaAt(schema, path) {
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

const nxFormulas = new WeakSet();

function nxIsFormula(value) {
  return typeof value === "function" && nxFormulas.has(value);
}

/**
 * `value` as a formula of `module` that closes over `locals`: evaluated where the environment
 * value is read. Serializing one throws rather than drop it.
 */

export function nxFormula(evaluate, module, locals) {
  const formula = Object.assign(() => evaluate(), {
    module,
    locals: Object.freeze({ ...locals }),
    toJSON() {
      throw nxEnvironmentError("nx-ir-boundary-type", "An environment formula of generated code cannot be serialized.");
    },
  });
  nxFormulas.add(formula);
  return Object.freeze(formula);
}

/** A `$env` entry holding a formula, which throws naming the value when it is serialized. */

function nxFormulaEntry(key, path, formula) {
  const entry = { value: key, path: [...path], formula };
  Object.defineProperty(entry, "toJSON", {
    enumerable: false,
    value() {
      throw nxEnvironmentError("nx-ir-boundary-type", `The '$env' entry for environment value '${key}' holds a formula of generated code, which cannot be serialized.`);
    },
  });
  return entry;
}

/** The layers one host `$env` entry gives: one, or one for each entry of a bundle. */

function nxHostEnvironmentLayers(entry, bundle, layers) {
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
  let source;
  if ("formula" in entry) {
    source = { formula: entry.formula };
  }
  else if (bundle) {
    // A bundle entry may hold an update record that patches the value, which the value's own type
    // does not describe.
    source = { value: entry.to };
  }
  else {
    source = {
      value: nxNormalizeValue(entry.to, nxEnvironmentSchemaAt(declaration.schema, entry.path), `environment value '${[entry.value, ...entry.path].join(".")}'`),
    };
  }
  layers.push({ set: true, key: declaration.key, path: entry.path, source, bundle });
}

/**
 * Runs `run` in `environment`, or with no root values when it is `undefined`, and restores the
 * environment around it afterwards. Root values and overrides are checked against their declared
 * types first.
 */

export function nxWithEnvironment(environment, run) {
  const state = nxNewEnvironmentState();
  if (environment !== undefined) {
    if (!(environment instanceof NxEnvironment)) {
      throw nxEnvironmentError("nx-ir-options", "The environment option must be an environment made with nxEnvironment(values).");
    }
    for (const [name, value] of environment.roots) {
      const declaration = nxEnvironmentByHostName(name);
      state.roots.set(declaration.key, nxNormalizeValue(value, declaration.schema, `environment value '${name}'`));
    }
    const layers = [];
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
  }
  finally {
    nxEnvironmentState = saved;
  }
}

function nxIsUpdateRecord(value) {
  return nxIsRecordValue(value) && typeof value.$type === "string" && value.$type.endsWith(".Update");
}

function nxShortEnvironmentName(key) {
  const at = key.lastIndexOf("::");
  return at < 0 ? key : key.slice(at + 2);
}

function nxEnvironmentCycle(state, key) {
  let start = 0;
  for (let position = state.layers.length - 1; position >= 0; position -= 1) {
    const layer = state.layers[position];
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
  throw nxEnvironmentError("nx-ir-environment-cycle", `Environment value '${nxShortEnvironmentName(key)}' is read while its own default is being evaluated: ${named}.`);
}

/**
 * What a source gives: its value, or a formula evaluated with reads of `key` capped at `below`. A
 * formula `layer` holds is reused while what it read is unchanged; one of an entry-module bundle is
 * evaluated at each read.
 */

function nxSourceValue(state, key, below, source, layer) {
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

export function nxEnvRead(declaration) {
  const state = nxEnvironmentState;
  nxNoteEnvironmentRead(state, declaration.key);
  const own = [];
  const beneath = [];
  let found = false;
  let base;
  let providesBelow;
  let position = state.layers.length;
  while (position > 0) {
    position -= 1;
    const layer = state.layers[position];
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

function nxApplyEnvironmentOverrides(base, pending) {
  let value = base;
  for (let index = pending.length - 1; index >= 0; index -= 1) {
    const entry = pending[index];
    value = nxSetEnvironmentPath(value, entry.path, 0, entry.value, entry.patch);
  }
  return value;
}

function nxIsRecordValue(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nxSetEnvironmentPath(value, path, at, to, patch) {
  if (at === path.length) {
    if (!patch) {
      return to;
    }
    if (nxIsRecordValue(value) && nxIsRecordValue(to)) {
      return nxApplyUpdate(value, to);
    }
    throw nxEnvironmentError("nx-ir-environment", "An environment update record applies only to a record value.");
  }
  const field = path[at];
  if (!nxIsRecordValue(value)) {
    throw nxEnvironmentError("nx-ir-environment", `Environment override of field '${field}' needs a record value.`);
  }
  const current = Object.prototype.hasOwnProperty.call(value, field) ? value[field] : undefined;
  let next;
  if (current !== undefined) {
    next = nxSetEnvironmentPath(current, path, at + 1, to, patch);
  }
  else if (at + 1 === path.length && !patch) {
    next = to;
  }
  else {
    throw nxEnvironmentError("nx-ir-environment", `Environment override of field '${field}' found the field empty.`);
  }
  return { ...value, [field]: next };
}

/** Runs `work` with a read of `key` seeing only the sources `below` gives. */

function nxCappedEnvironment(state, key, below, work) {
  const depth = state.layers.length;
  nxPushLayer(state, { set: false, key, below });
  try {
    return work();
  }
  finally {
    state.layers.length = depth;
  }
}

/** The `$entries` of a bundle value. */

function nxBundleItems(bundle) {
  if (!nxIsRecordValue(bundle) || bundle.$type !== "env.Update" || !Array.isArray(bundle.$entries)) {
    throw nxEnvironmentError("nx-ir-environment", "A bundle needs an 'env.Update' value.");
  }
  return bundle.$entries;
}

/** The entries of a bundle's `$entries`, in order, a bundle entry's own entries in its place. */

function nxBundleEntries(items, out = []) {
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
      throw nxEnvironmentError("nx-ir-boundary-type", `An 'env.Update' entry for '${entry.value}' holds a formula that is not one of this generated program's.`);
    }
    const path = Array.isArray(entry.path) ? entry.path : [];
    out.push({
      key: entry.value,
      path,
      source: entry.formula !== undefined ? { formula: entry.formula } : { value: entry.to },
    });
  }
  return out;
}

function nxProvidedOrDefault(state, declaration, below) {
  const pending = [];
  const provides = nxEnvironmentProvides;
  for (let position = Math.min(below, provides.length) - 1; position >= 0; position -= 1) {
    const [target, path, evaluate] = provides[position];
    const cap = { provide: position };
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
      const entry = entries[index];
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

function nxEvaluatedEntries(overrides) {
  return overrides.map(([declaration, path, value]) => {
    if (declaration === null) {
      const items = nxBundleItems(value);
      return { bundle: nxBundleEntries(items), items };
    }
    const source = nxIsFormula(value) ? { formula: value } : { value };
    return { set: { key: declaration.key, path, source } };
  });
}

/**
 * The `$env` or `$entries` entries for evaluated overrides. In `$env` a bundle is one `{bundle}`
 * entry; in a bundle's `$entries` its entries take its place.
 */

function nxEnvironmentEntryValues(evaluated, nest) {
  const out = [];
  for (const item of evaluated) {
    if ("set" in item) {
      const { key, path, source } = item.set;
      out.push("value" in source ? { value: key, path: [...path], to: source.value } : nxFormulaEntry(key, path, source.formula));
    }
    else if (nest) {
      out.push({ bundle: [...item.items] });
    }
    else {
      out.push(...item.items);
    }
  }
  return out;
}

/** `provide overrides { body }`: the overrides, evaluated by the caller, in effect while the body runs, then added to the `$env` of every component descriptor the result reaches. */

export function nxProvide(overrides, body) {
  const evaluated = nxEvaluatedEntries(overrides);
  const state = nxEnvironmentState;
  const depth = state.layers.length;
  for (const item of evaluated) {
    if ("set" in item) {
      nxPushLayer(state, { set: true, key: item.set.key, path: item.set.path, source: item.set.source, bundle: false });
    }
    else {
      for (const { key, path, source } of item.bundle) {
        nxPushLayer(state, { set: true, key, path, source, bundle: true });
      }
    }
  }
  let result;
  try {
    result = body();
  }
  finally {
    state.layers.length = depth;
  }
  return nxStampEnvironment(result, nxEnvironmentEntryValues(evaluated, true));
}

/** `<env.Update … />` or `base with { … }`: the base's entries, then the overrides, as one bundle value. */

export function nxEnvUpdate(overrides, base) {
  const entries = base === undefined ? [] : [...nxBundleItems(base)];
  entries.push(...nxEnvironmentEntryValues(nxEvaluatedEntries(overrides), false));
  return { $type: "env.Update", $entries: entries };
}

function nxStampEnvironment(value, entries) {
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
  const output = {};
  for (const [key, field] of Object.entries(value)) {
    output[key] = nxStampEnvironment(field, entries);
  }
  return output;
}
