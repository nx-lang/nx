/**
 * The instance tree behind a drawing: one node per use of an authored component, keyed by its
 * position in the drawn tree, holding the runtime instance the renderer dispatches against.
 *
 * <para>The runtime dispatches one instance at a time and composes nothing; composing is done
 * here. A node is created from a descriptor under the node that encloses it, so a handler the
 * parent bound reaches the child through the parent's instance, and is initialized again with the
 * state it holds whenever its parent redraws and hands it a new descriptor: props flow down and
 * state stays. A drawing pass visits every node it draws; the nodes it did not visit are dropped
 * when the pass ends, so an instance lives exactly as long as its place in the tree.</para>
 *
 * <para>A callback for a token drawn inside one node's output dispatches against the ancestor
 * whose body created the handler — the farthest ancestor whose table holds the same handler value,
 * which the runtime guarantees is the parent's own — so a `Page`-bound handler inside a `Stack`'s
 * content patches `Page`. Each effect the dispatch returns is routed to the handler the parent
 * bound for that emit, or collected as a host effect for the visitor to see. A chain of dispatches
 * is all or nothing: a failure part way through it restores every instance it had replaced and
 * throws the runtime's diagnostic.</para>
 */
import {
  dispatchComponentActions,
  initializeComponent,
  type ActionHandlerValue,
  type NxCanonicalValue,
  type NxComponentInstance,
} from "@nx-lang/ir-runtime";
import type { Program } from "./evaluate";
import type { NxObject, NxValue } from "./values";

/**
 * The record the runtime renders a handler as: the name of the action it accepts, and a token when
 * an instance rendered it. Pure evaluation, which is what the root function is, renders no token.
 */
export interface HandlerRecord extends NxObject {
  readonly $type: "ActionHandler";
  readonly action: string;
  readonly token?: string;
}

export function isHandlerRecord(value: NxValue | undefined): value is HandlerRecord {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    value.$type === "ActionHandler" &&
    typeof value.action === "string"
  );
}

/** One use of an authored component, at one position in the drawn tree. */
export interface InstanceNode {
  readonly key: string;
  readonly component: string;
  readonly parent: InstanceNode | null;
  /** The descriptor the node was last initialized from; a different one means the parent redrew. */
  descriptor: NxObject;
  instance: NxComponentInstance;
  rendered: NxValue;
}

/** An action that left the tree: nothing above the instance that produced it handled it. */
export interface HostEffect {
  readonly instance: string;
  readonly action: NxObject;
}

/** The props a descriptor supplies: everything but its discriminator. */
function fieldsOf(descriptor: NxObject): Record<string, NxCanonicalValue> {
  const fields: Record<string, NxCanonicalValue> = {};
  for (const [name, value] of Object.entries(descriptor)) {
    if (name !== "$type" && value !== undefined) {
      fields[name] = value as NxCanonicalValue;
    }
  }
  return fields;
}

/**
 * Removes every handler record without a token from a value, at any depth, naming each as
 * `<Type>.<property>` after the value that carried it.
 *
 * A record without a token comes from pure evaluation, which is what the root function is: it
 * names no handler any instance holds, so a control drawn with it gets no callback and a component
 * initialized with it would be refused. The drawing reports each as inert instead.
 */
export function stripInertHandlers(value: NxValue): { value: NxValue; inert: string[] } {
  const inert: string[] = [];
  const walk = (item: NxValue): NxValue => {
    if (Array.isArray(item)) {
      return item.map(walk);
    }
    if (typeof item !== "object" || item === null) {
      return item;
    }
    const owner = typeof item.$type === "string" ? item.$type : "(record)";
    const output: Record<string, NxValue | undefined> = {};
    for (const [name, entry] of Object.entries(item)) {
      if (isHandlerRecord(entry) && entry.token === undefined) {
        inert.push(`${owner}.${name}`);
        continue;
      }
      output[name] = entry === undefined ? undefined : walk(entry);
    }
    return output as NxObject;
  };
  return { value: walk(value), inert };
}

function tokenOf(instance: NxComponentInstance, handler: ActionHandlerValue): string | undefined {
  for (const [token, held] of instance.handlers) {
    if (held === handler) {
      return token;
    }
  }
  return undefined;
}

/** The ancestor, `from` included, farthest from it whose table holds `handler`: the body that created it. */
function creatorOf(from: InstanceNode | null, handler: ActionHandlerValue): { node: InstanceNode; token: string } | null {
  let found: { node: InstanceNode; token: string } | null = null;
  for (let node = from; node !== null; node = node.parent) {
    const token = tokenOf(node.instance, handler);
    if (token !== undefined) {
      found = { node, token };
    }
  }
  return found;
}

function depthOf(node: InstanceNode): number {
  let depth = 0;
  for (let parent = node.parent; parent !== null; parent = parent.parent) {
    depth += 1;
  }
  return depth;
}

interface Pending {
  readonly node: InstanceNode;
  readonly token: string;
  readonly action: NxObject;
}

/** What a node holds that a drawing pass or a dispatch replaces. */
interface Saved {
  readonly node: InstanceNode;
  readonly descriptor: NxObject;
  readonly instance: NxComponentInstance;
  readonly rendered: NxValue;
}

export class InstanceTree {
  readonly #program: Program;
  readonly #nodes = new Map<string, InstanceNode>();
  #visited = new Set<string>();
  #inert: string[] = [];

  constructor(program: Program) {
    this.#program = program;
  }

  get program(): Program {
    return this.#program;
  }

  /** Starts a drawing pass. Every node the pass does not visit is dropped at `finish`. */
  begin(): void {
    this.#visited = new Set();
    this.#inert = [];
  }

  /** The handler properties the pass found bound outside any component, by path from the root. */
  get inert(): readonly string[] {
    return this.#inert;
  }

  finish(): void {
    for (const key of [...this.#nodes.keys()]) {
      if (!this.#visited.has(key)) {
        this.#nodes.delete(key);
      }
    }
  }

  /**
   * The node for the authored descriptor drawn at `key` under `parent`: created on first sight,
   * kept while the descriptor is the same object, and initialized again with the state it holds
   * when the parent handed it a new one.
   */
  visit(key: string, descriptor: NxObject, parent: InstanceNode | null): InstanceNode {
    this.#visited.add(key);
    const component = descriptor.$type;
    if (typeof component !== "string") {
      throw new Error(`The value at ${key} is not a component descriptor.`);
    }
    // A descriptor under no instance came from pure evaluation, so a handler record in it has
    // no token and names nothing: it is stripped and reported rather than refused.
    let fields = descriptor;
    if (parent === null) {
      const stripped = stripInertHandlers(descriptor);
      fields = stripped.value as NxObject;
      this.#inert.push(...stripped.inert);
    }
    const existing = this.#nodes.get(key);
    if (existing !== undefined && existing.component === component && existing.parent === parent) {
      if (existing.descriptor !== descriptor) {
        const result = initializeComponent(this.#program, component, fieldsOf(fields), {
          parent: parent?.instance,
          state: existing.instance.state,
        });
        existing.descriptor = descriptor;
        existing.instance = result.instance;
        existing.rendered = result.rendered as NxValue;
      }
      return existing;
    }
    const result = initializeComponent(this.#program, component, fieldsOf(fields), { parent: parent?.instance });
    const node: InstanceNode = {
      key,
      component,
      parent,
      descriptor,
      instance: result.instance,
      rendered: result.rendered as NxValue,
    };
    this.#nodes.set(key, node);
    return node;
  }

  /**
   * Runs `work` as one change to the tree: when it throws, every node is put back as it was before,
   * so the drawing made before it still names the instances its callbacks dispatch against.
   */
  atomically<T>(work: () => T): T {
    const nodes = new Map(this.#nodes);
    const saved: Saved[] = [...nodes.values()].map((node) => ({
      node,
      descriptor: node.descriptor,
      instance: node.instance,
      rendered: node.rendered,
    }));
    const visited = this.#visited;
    const inert = this.#inert;
    try {
      return work();
    } catch (error) {
      this.#nodes.clear();
      for (const [key, node] of nodes) {
        this.#nodes.set(key, node);
      }
      for (const { node, descriptor, instance, rendered } of saved) {
        node.descriptor = descriptor;
        node.instance = instance;
        node.rendered = rendered;
      }
      this.#visited = visited;
      this.#inert = inert;
      throw error;
    }
  }

  /**
   * Dispatches the handler drawn under `token` in `source`'s output with `action`, against the
   * instance whose body created it, and routes what comes back. Returns the effects nothing in the
   * tree handled. Throws the runtime's diagnostic when a dispatch fails, with every instance as it
   * was before the call.
   *
   * <para>Every dispatch routes its effects to strict ancestors of the node it ran against, so the
   * pending node farthest from the root is dispatched first, with every entry pending for it in one
   * batch: a node is never dispatched twice in one chain, so no entry holds a token its node's
   * previous dispatch retired.</para>
   */
  dispatch(source: InstanceNode, token: string, action: NxObject): HostEffect[] {
    return this.atomically(() => {
      const handler = source.instance.handlers.get(token);
      if (handler === undefined) {
        throw new Error(`The '${source.component}' instance holds no handler under token '${token}'.`);
      }
      const creator = creatorOf(source, handler);
      if (creator === null) {
        throw new Error(`No instance created the handler under token '${token}'.`);
      }
      let pending: Pending[] = [{ ...creator, action }];
      const effects: HostEffect[] = [];
      while (pending.length > 0) {
        const node = pending.reduce((deepest, entry) =>
          depthOf(entry.node) > depthOf(deepest) ? entry.node : deepest, pending[0].node);
        const batch = pending.filter((entry) => entry.node === node);
        pending = pending.filter((entry) => entry.node !== node);
        const result = dispatchComponentActions(
          this.#program,
          node.instance,
          batch.map((entry) => ({
            $type: "ActionHandlerInvocation",
            token: entry.token,
            action: entry.action as NxCanonicalValue,
          })),
        );
        node.instance = result.instance;
        node.rendered = result.rendered as NxValue;
        for (const effect of result.effects) {
          const routed = this.#route(node, effect as NxObject);
          if (routed === null) {
            effects.push({ instance: node.component, action: effect as NxObject });
          } else {
            pending.push(routed);
          }
        }
      }
      return effects;
    });
  }

  /** Where an effect goes: to the handler the parent bound for the emit, dispatched against its creator, or nowhere. */
  #route(node: InstanceNode, effect: NxObject): Pending | null {
    const kind = node.instance.declaration.kind;
    if (kind.tag !== "component" || typeof effect.$type !== "string") {
      return null;
    }
    const emit = kind.emits.find((candidate) => candidate.action.name === effect.$type);
    if (emit === undefined) {
      return null;
    }
    const handler = node.instance.handlerProps.get(`on${emit.name}`);
    if (handler === undefined) {
      return null;
    }
    const creator = creatorOf(node.parent, handler);
    return creator === null ? null : { ...creator, action: effect };
  }
}
