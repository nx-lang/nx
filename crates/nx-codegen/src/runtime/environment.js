// Environment values: the lookup generated code shares, with the same order and results as the
// IR runtimes. Evaluation is synchronous, so one module-level state is safe: `nxWithEnvironment`
// sets it for a call, and a provide adds layers to it around its body.

const nxEnvironmentDeclarations = new Map();
const nxEnvironmentComponents = new Set();
let nxEnvironmentProvides = [];
let nxEnvironmentState = { layers: [], base: 0, roots: new Map() };

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
  return carried.map((entry) => {
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
      nxMalformedEnvironment("an entry is not a record");
    }
    if (typeof entry.value !== "string") {
      nxMalformedEnvironment("an entry has no 'value' string");
    }
    let path;
    if (entry.path === undefined || entry.path === null) {
      path = [];
    } else if (typeof entry.path === "string") {
      path = [entry.path];
    } else if (Array.isArray(entry.path)) {
      path = entry.path.map((field) => {
        if (typeof field !== "string") {
          nxMalformedEnvironment("a 'path' entry is not a string");
        }
        return field;
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
export function nxWithEnvironment(environment, run) {
  const state = { layers: [], base: 0, roots: new Map() };
  if (environment !== undefined) {
    if (!(environment instanceof NxEnvironment)) {
      throw nxEnvironmentError("nx-ir-options", "The environment option must be an environment made with nxEnvironment(values).");
    }
    for (const [name, value] of environment.roots) {
      const declaration = nxEnvironmentByHostName(name);
      state.roots.set(declaration.key, nxNormalizeValue(value, declaration.schema, `environment value '${name}'`));
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
export function nxEnvRead(declaration) {
  const state = nxEnvironmentState;
  const pending = [];
  let local = 0;
  let found = false;
  let base;
  let cap;
  for (let position = state.layers.length - 1; position >= 0; position -= 1) {
    const layer = state.layers[position];
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
  } else if (at + 1 === path.length && !patch) {
    next = to;
  } else {
    throw nxEnvironmentError("nx-ir-environment", `Environment override of field '${field}' found the field empty.`);
  }
  return { ...value, [field]: next };
}

function nxCappedEnvironment(state, key, index, work) {
  const depth = state.layers.length;
  state.layers.push({ set: false, key, index });
  try {
    return work();
  } finally {
    state.layers.length = depth;
  }
}

function nxProvidedOrDefault(state, declaration, below) {
  const pending = [];
  const provides = nxEnvironmentProvides;
  for (let position = Math.min(below, provides.length) - 1; position >= 0; position -= 1) {
    const [target, path, evaluate] = provides[position];
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
      const fields = Array.isArray(entry.path) ? entry.path : [];
      const patch = nxIsRecordValue(entry.to) && typeof entry.to.$type === "string" && entry.to.$type.endsWith(".Update");
      if (fields.length === 0 && !patch) {
        return nxApplyEnvironmentOverrides(entry.to, pending);
      }
      pending.push({ path: fields, value: entry.to, patch });
    }
  }
  return nxApplyEnvironmentOverrides(declaration.defaultValue(), pending);
}

function nxEnvironmentEntryValues(overrides) {
  return overrides.map(([declaration, path, value]) => ({ value: declaration.key, path: [...path], to: value }));
}

/** `provide overrides { body }`: the overrides, evaluated by the caller, in effect while the body runs, then added to the `$env` of every component descriptor the result reaches. */
export function nxProvide(overrides, body) {
  const state = nxEnvironmentState;
  const depth = state.layers.length;
  for (const [declaration, path, value] of overrides) {
    state.layers.push({ set: true, key: declaration.key, path, value });
  }
  let result;
  try {
    result = body();
  } finally {
    state.layers.length = depth;
  }
  return nxStampEnvironment(result, nxEnvironmentEntryValues(overrides));
}

/** `<env.Update … />`: the overrides as one bundle value. */
export function nxEnvUpdate(overrides) {
  return { $type: "env.Update", $entries: nxEnvironmentEntryValues(overrides) };
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
