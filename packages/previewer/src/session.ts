import {
  NxIrRuntimeError,
  dispatchComponentActions,
  initializeComponent,
  type NxCanonicalValue,
  type NxHostRecord,
  type NxHostValue,
  type NxIrDiagnostic,
  type NxOriginEntry,
  type NxPreparedProgram,
  type NxRuntimeOptions,
  type NxRuntimeOrigins,
} from "@nx-lang/ir-runtime";

import { escapePointerToken, readPointer } from "./pointer.js";
import type {
  PreviewBatchEntry,
  PreviewCause,
  PreviewHandlerEntry,
  PreviewReloadResult,
  PreviewReplayResult,
  PreviewReplayStop,
  PreviewScenario,
  PreviewSessionOptions,
  PreviewTick,
} from "./types.js";

/** The default of {@link PreviewSessionOptions.maxTicks}. */
export const PREVIEW_DEFAULT_MAX_TICKS = 1000;

const handlerTypeName = "ActionHandler";
const invocationTypeName = "ActionHandlerInvocation";

/**
 * One run of one component of a program: a tree of ticks, a current tick, and the calls that add
 * ticks to it. Every call that adds a tick makes it a child of the current tick and makes it
 * current; a call that fails throws the runtime's {@link NxIrRuntimeError} and adds no tick.
 */
export interface PreviewSession {
  /** The component the session runs. */
  readonly component: string;
  /** The program the session last loaded, which {@link replay} runs. */
  readonly program: NxPreparedProgram;
  readonly current: PreviewTick;
  /** The ticks from the current tick's root to the current tick, in order. */
  readonly path: readonly PreviewTick[];
  /** Every tick the session keeps, in the order they were made. */
  readonly ticks: readonly PreviewTick[];
  /** The ticks with no parent: the first render, and the first tick of each replay. */
  readonly roots: readonly PreviewTick[];
  /** The children of a tick of this session, in the order they were made. */
  childrenOf(tick: PreviewTick): readonly PreviewTick[];
  /** Makes a tick of this session the current one. Nothing is evaluated. */
  goTo(tick: PreviewTick): void;
  /**
   * Dispatches a batch from the current tick under the program that rendered it. Each entry is an
   * entry as the runtime takes it or a {@link PreviewHandlerEntry}, which the session turns into an
   * invocation of the token at that pointer after checking that the action's `$type` is the action
   * the handler record names.
   */
  dispatch(batch: readonly PreviewBatchEntry[]): PreviewTick;
  /** Dispatches one action to the handler at `pointer` in the current tick's rendered output. */
  dispatchHandler(pointer: string, action: NxHostRecord): PreviewTick;
  /**
   * Renders the current tick's state with new props, under the program that rendered it. The
   * runtime validates the state as complete for the component.
   */
  setProps(props: NxHostRecord): PreviewTick;
  /**
   * The current path as a scenario: the root's props and the batches after it, with each entry's
   * handler pointer. A reload tick on the path adds nothing. A props change cannot be held in a
   * lifecycle, so a path that holds one fails with `nx-preview-scenario`.
   */
  scenario(name?: string): PreviewScenario;
  /**
   * Replays a scenario under {@link program}: a new root from the scenario's props, then each
   * batch in turn, an entry with a handler pointer by the token now at that pointer and any other
   * entry as it was recorded. The replay stops at the first entry that cannot be placed, or at a
   * batch the runtime fails, and keeps the ticks before it.
   *
   * @throws NxIrRuntimeError when the scenario is malformed, is for another component or module, or
   * its props do not render; no tick is added then.
   */
  replay(scenario: PreviewScenario): PreviewReplayResult;
  /**
   * Runs the component under a new program, keeping the reviewer where they were. It first renders
   * the current tick's props and state under the new program, as a reload child of the current
   * tick. When that fails but the props alone render, it replays the current path under the new
   * program, as a new root. When the props do not render, it keeps the old program and the current
   * tick. The old ticks stay in the tree in every case.
   */
  reload(program: NxPreparedProgram): PreviewReloadResult;
  /** The origin of the record at `pointer` in a tick's rendered output, or `undefined`. */
  originOf(tick: PreviewTick, pointer: string): NxOriginEntry | undefined;
}

/**
 * Starts a session: initializes `component` of `program` with `props` and makes the result the
 * session's first tick.
 *
 * @throws NxIrRuntimeError with the runtime's diagnostics when the component does not render, such
 * as for props its declaration rejects, or when `maxTicks` is not a positive integer.
 */
export function createPreviewSession(
  program: NxPreparedProgram,
  component: string,
  props: NxHostRecord = {},
  options: PreviewSessionOptions = {},
): PreviewSession {
  const session = new Session(program, component, options);
  session.start(props);
  return session;
}

/**
 * Starts a session by replaying a scenario under `program`, as {@link PreviewSession.replay} does,
 * with the scenario's first render as the session's first tick.
 *
 * @throws NxIrRuntimeError when the scenario is malformed, is for another module, or its props do
 * not render.
 */
export function replayScenario(
  program: NxPreparedProgram,
  scenario: PreviewScenario,
  options: PreviewSessionOptions = {},
): { readonly session: PreviewSession; readonly result: PreviewReplayResult } {
  const read = readScenario(scenario);
  const session = new Session(program, read.component, options);
  const result = session.replay(read);
  return { session, result };
}

/** A step of a replay after its first render. */
type ReplayStep =
  | { readonly kind: "batch"; readonly batch: readonly NxHostValue[]; readonly handlers: readonly (string | null)[] | undefined }
  | { readonly kind: "props"; readonly props: NxHostRecord };

/** A failure to place one entry of a batch. */
class PlacementError extends NxIrRuntimeError {
  public readonly entry: number;

  public constructor(entry: number, message: string) {
    super([previewDiagnostic("nx-preview-handler", message)]);
    this.entry = entry;
  }
}

class Session implements PreviewSession {
  public readonly component: string;
  #program: NxPreparedProgram;
  #current: PreviewTick | undefined;
  readonly #ticks = new Map<number, PreviewTick>();
  readonly #children = new Map<PreviewTick, PreviewTick[]>();
  /** Where each handler token of a tick's output sits, found when first asked for. */
  readonly #tokens = new WeakMap<PreviewTick, ReadonlyMap<string, string>>();
  readonly #limits: NxRuntimeOptions;
  readonly #origins: boolean;
  readonly #maxTicks: number;
  #nextId = 1;

  public constructor(program: NxPreparedProgram, component: string, options: PreviewSessionOptions) {
    this.component = component;
    this.#program = program;
    this.#origins = options.origins ?? true;
    this.#maxTicks = options.maxTicks ?? PREVIEW_DEFAULT_MAX_TICKS;
    if (!Number.isSafeInteger(this.#maxTicks) || this.#maxTicks < 1) {
      throw new NxIrRuntimeError([
        previewDiagnostic("nx-preview-options", `maxTicks must be a positive integer, not ${String(options.maxTicks)}.`),
      ]);
    }
    const limits: { -readonly [K in keyof NxRuntimeOptions]: NxRuntimeOptions[K] } = {};
    for (const name of ["maxOperations", "maxInputSize", "maxCallDepth", "maxRangeLength"] as const) {
      const value = options[name];
      if (value !== undefined) {
        limits[name] = value;
      }
    }
    this.#limits = limits;
  }

  public get program(): NxPreparedProgram {
    return this.#program;
  }

  public get current(): PreviewTick {
    if (this.#current === undefined) {
      throw new Error("The session has no tick yet.");
    }
    return this.#current;
  }

  public get path(): readonly PreviewTick[] {
    const path: PreviewTick[] = [];
    for (let tick: PreviewTick | undefined = this.current; tick !== undefined; tick = tick.parent) {
      path.push(tick);
    }
    return path.reverse();
  }

  public get ticks(): readonly PreviewTick[] {
    return [...this.#ticks.values()];
  }

  public get roots(): readonly PreviewTick[] {
    return this.ticks.filter((tick) => tick.parent === undefined);
  }

  public childrenOf(tick: PreviewTick): readonly PreviewTick[] {
    this.#require(tick);
    return [...(this.#children.get(tick) ?? [])];
  }

  public goTo(tick: PreviewTick): void {
    this.#require(tick);
    this.#current = tick;
  }

  /** Renders the first tick. */
  public start(props: NxHostRecord): PreviewTick {
    return this.#initialize(this.#program, { kind: "initial", props: frozenCopy(props) }, undefined, undefined);
  }

  public dispatch(batch: readonly PreviewBatchEntry[]): PreviewTick {
    if (!Array.isArray(batch)) {
      throw new NxIrRuntimeError([previewDiagnostic("nx-preview-batch", "A batch is a list of entries.")]);
    }
    return this.#dispatch(this.current, batch, undefined);
  }

  public dispatchHandler(pointer: string, action: NxHostRecord): PreviewTick {
    return this.dispatch([{ handler: pointer, action }]);
  }

  public setProps(props: NxHostRecord): PreviewTick {
    const tick = this.current;
    return this.#initialize(tick.program, { kind: "props", props: frozenCopy(props) }, tick, tick.state);
  }

  public scenario(name?: string): PreviewScenario {
    const [root, ...rest] = this.path;
    if (root === undefined || root.cause.kind !== "initial") {
      throw new Error("The current path does not start with a first render.");
    }
    const batches: (readonly NxHostValue[])[] = [];
    const handlers: (readonly (string | null)[])[] = [];
    for (const tick of rest) {
      if (tick.cause.kind === "batch") {
        batches.push(tick.cause.batch);
        handlers.push(tick.cause.handlers);
      } else if (tick.cause.kind === "props") {
        throw new NxIrRuntimeError([
          previewDiagnostic(
            "nx-preview-scenario",
            `The current path changes the props at tick ${tick.id}, which a scenario cannot hold.`,
          ),
        ]);
      }
    }
    return frozenCopy({
      ...(name === undefined ? {} : { name }),
      module: root.program.entry.module.identity,
      component: this.component,
      props: root.cause.props,
      batches,
      handlers,
    });
  }

  public replay(scenario: PreviewScenario): PreviewReplayResult {
    const read = readScenario(scenario);
    if (read.component !== this.component) {
      throw scenarioError(`The scenario is for '${read.component}', and the session runs '${this.component}'.`);
    }
    const module = this.#program.entry.module.identity;
    if (read.module !== module) {
      throw scenarioError(`The scenario is for the module '${read.module}', and the session's program is '${module}'.`);
    }
    const steps: ReplayStep[] = read.batches.map((batch, index) => ({
      kind: "batch",
      batch,
      handlers: read.handlers?.[index],
    }));
    return this.#replay(this.#program, read.props ?? {}, steps);
  }

  public reload(program: NxPreparedProgram): PreviewReloadResult {
    const tick = this.current;
    try {
      const reloaded = this.#initialize(program, { kind: "reload" }, tick, tick.state);
      this.#program = program;
      return { outcome: "kept", tick: reloaded };
    } catch (error) {
      if (!(error instanceof NxIrRuntimeError)) {
        throw error;
      }
    }
    // The state did not render under the new program. Whether the props do is what decides
    // between replaying the path and keeping the old program.
    try {
      initializeComponent(program, this.component, tick.props, this.#options(undefined));
    } catch (error) {
      if (error instanceof NxIrRuntimeError) {
        return { outcome: "failed", diagnostics: error.diagnostics };
      }
      throw error;
    }
    const [root, ...rest] = this.path;
    if (root === undefined || root.cause.kind !== "initial") {
      throw new Error("The current path does not start with a first render.");
    }
    const steps: ReplayStep[] = [];
    for (const step of rest) {
      if (step.cause.kind === "batch") {
        steps.push({ kind: "batch", batch: step.cause.batch, handlers: step.cause.handlers });
      } else if (step.cause.kind === "props") {
        steps.push({ kind: "props", props: step.cause.props });
      }
    }
    try {
      const replayed = this.#replay(program, root.cause.props, steps);
      this.#program = program;
      return { outcome: "replayed", ...replayed };
    } catch (error) {
      if (error instanceof NxIrRuntimeError) {
        return { outcome: "failed", diagnostics: error.diagnostics };
      }
      throw error;
    }
  }

  public originOf(tick: PreviewTick, pointer: string): NxOriginEntry | undefined {
    this.#require(tick);
    return tick.origins?.find((entry) => entry.path === pointer);
  }

  /**
   * Replays from a new root: the first render from `props`, which throws when it fails, then each
   * step in turn until one fails.
   */
  #replay(program: NxPreparedProgram, props: NxHostRecord, steps: readonly ReplayStep[]): PreviewReplayResult {
    let tick = this.#initialize(program, { kind: "initial", props: frozenCopy(props) }, undefined, undefined);
    let batch = 0;
    for (const step of steps) {
      try {
        if (step.kind === "batch") {
          tick = this.#dispatch(tick, step.batch, step.handlers);
          batch += 1;
        } else {
          tick = this.#initialize(tick.program, { kind: "props", props: frozenCopy(step.props) }, tick, tick.state);
        }
      } catch (error) {
        if (!(error instanceof NxIrRuntimeError)) {
          throw error;
        }
        const stopped: PreviewReplayStop =
          error instanceof PlacementError
            ? { batch, entry: error.entry, diagnostics: error.diagnostics }
            : { batch, diagnostics: error.diagnostics };
        return { tick, completed: false, stopped };
      }
    }
    return { tick, completed: true };
  }

  /**
   * Initializes the component under `program` and adds the result as a child of `parent`, or as a
   * root, rendering `state` in place of the initial one when it is given. The props are the
   * cause's, or the parent's for a reload.
   */
  #initialize(
    program: NxPreparedProgram,
    cause: PreviewCause,
    parent: PreviewTick | undefined,
    state: Readonly<Record<string, NxCanonicalValue>> | undefined,
  ): PreviewTick {
    const props = cause.kind === "initial" || cause.kind === "props" ? cause.props : parent?.props ?? {};
    const report: NxRuntimeOrigins = {};
    const result = initializeComponent(program, this.component, props, {
      ...this.#options(report),
      ...(state === undefined ? {} : { state }),
    });
    return this.#add({
      cause,
      parent,
      program,
      props,
      instance: result.instance,
      state: result.state,
      rendered: result.rendered,
      effects: [],
      origins: this.#origins ? report.entries ?? [] : undefined,
    });
  }

  /** Places `entries` against `tick`'s output, dispatches them, and adds the result as its child. */
  #dispatch(
    tick: PreviewTick,
    entries: readonly PreviewBatchEntry[],
    recorded: readonly (string | null)[] | undefined,
  ): PreviewTick {
    const batch: NxHostValue[] = [];
    const handlers: (string | null)[] = [];
    entries.forEach((entry, index) => {
      const placed = this.#place(tick, entry, recorded?.[index] ?? null, index);
      batch.push(placed.entry);
      handlers.push(placed.pointer);
    });
    const report: NxRuntimeOrigins = {};
    const frozenBatch = frozenCopy(batch);
    const result = dispatchComponentActions(tick.program, tick.instance, frozenBatch, this.#options(report));
    return this.#add({
      cause: { kind: "batch", batch: frozenBatch, handlers },
      parent: tick,
      program: tick.program,
      props: tick.props,
      instance: result.instance,
      state: result.state,
      rendered: result.rendered,
      effects: result.effects,
      origins: this.#origins ? report.entries ?? [] : undefined,
    });
  }

  /**
   * Turns one entry into the entry the runtime takes and the pointer of the handler it invokes in
   * `tick`'s output. A handler entry, or a recorded entry with a `pointer`, invokes the token at
   * that pointer; an invocation by token must name a token the output holds; anything else, an
   * action the component emits, is passed as it is and invokes no handler there.
   */
  #place(
    tick: PreviewTick,
    entry: PreviewBatchEntry,
    pointer: string | null,
    index: number,
  ): { readonly entry: NxHostValue; readonly pointer: string | null } {
    if (isHandlerEntry(entry)) {
      return this.#invoke(tick, entry.handler, entry.action, index);
    }
    if (!isRecord(entry) || entry.$type !== invocationTypeName) {
      return { entry: entry as NxHostValue, pointer: null };
    }
    if (pointer !== null) {
      return this.#invoke(tick, pointer, entry.action as NxHostRecord, index);
    }
    const token = entry.token;
    const at = typeof token === "string" ? this.#tokensOf(tick).get(token) : undefined;
    if (at === undefined) {
      throw new PlacementError(
        index,
        `Entry ${index} invokes the handler token ${JSON.stringify(token)}, which the output of tick ${tick.id} does not hold.`,
      );
    }
    return { entry, pointer: at };
  }

  /** The invocation of the handler at `pointer` in `tick`'s output with `action`. */
  #invoke(
    tick: PreviewTick,
    pointer: string,
    action: NxHostRecord | undefined,
    index: number,
  ): { readonly entry: NxHostValue; readonly pointer: string } {
    if (typeof pointer !== "string") {
      throw new PlacementError(index, `Entry ${index} names its handler by ${JSON.stringify(pointer)}, which is no JSON pointer.`);
    }
    const record = readPointer(tick.rendered, pointer);
    if (
      !isRecord(record) ||
      record.$type !== handlerTypeName ||
      typeof record.action !== "string" ||
      typeof record.token !== "string"
    ) {
      throw new PlacementError(index, `The output of tick ${tick.id} has no ActionHandler record at '${pointer}'.`);
    }
    if (!isRecord(action) || typeof action.$type !== "string") {
      throw new PlacementError(index, `Entry ${index} for the handler at '${pointer}' carries no action record with a '$type'.`);
    }
    if (action.$type !== record.action) {
      throw new PlacementError(
        index,
        `The handler at '${pointer}' takes '${record.action}', not '${action.$type}'.`,
      );
    }
    return { entry: { $type: invocationTypeName, token: record.token, action }, pointer };
  }

  /** Where each handler token of `tick`'s output sits. */
  #tokensOf(tick: PreviewTick): ReadonlyMap<string, string> {
    let tokens = this.#tokens.get(tick);
    if (tokens === undefined) {
      const found = new Map<string, string>();
      collectTokens(tick.rendered, "", found);
      tokens = found;
      this.#tokens.set(tick, tokens);
    }
    return tokens;
  }

  #options(report: NxRuntimeOrigins | undefined): NxRuntimeOptions {
    return this.#origins && report !== undefined ? { ...this.#limits, origins: report } : this.#limits;
  }

  #add(fields: Omit<PreviewTick, "id">): PreviewTick {
    const tick: PreviewTick = Object.freeze({
      id: this.#nextId,
      cause: deepFreeze(fields.cause),
      parent: fields.parent,
      program: fields.program,
      props: deepFreeze(fields.props),
      instance: fields.instance,
      state: deepFreeze(fields.state),
      rendered: deepFreeze(fields.rendered),
      effects: deepFreeze(fields.effects),
      origins: fields.origins === undefined ? undefined : deepFreeze(fields.origins),
    });
    this.#nextId += 1;
    this.#ticks.set(tick.id, tick);
    this.#children.set(tick, []);
    if (tick.parent !== undefined) {
      this.#children.get(tick.parent)?.push(tick);
    }
    this.#current = tick;
    this.#prune();
    return tick;
  }

  /**
   * Drops the oldest ticks that are off the current path and have no children until the session
   * is within `maxTicks`. Dropping leaves only, oldest first, keeps every kept tick's parent kept.
   */
  #prune(): void {
    if (this.#ticks.size <= this.#maxTicks) {
      return;
    }
    const onPath = new Set(this.path);
    while (this.#ticks.size > this.#maxTicks) {
      let dropped: PreviewTick | undefined;
      for (const tick of this.#ticks.values()) {
        if (!onPath.has(tick) && this.#children.get(tick)?.length === 0) {
          dropped = tick;
          break;
        }
      }
      if (dropped === undefined) {
        return;
      }
      this.#ticks.delete(dropped.id);
      this.#children.delete(dropped);
      if (dropped.parent !== undefined) {
        const siblings = this.#children.get(dropped.parent);
        siblings?.splice(siblings.indexOf(dropped), 1);
      }
    }
  }

  #require(tick: PreviewTick): void {
    if (this.#ticks.get((tick as PreviewTick | undefined)?.id ?? 0) !== tick) {
      throw new NxIrRuntimeError([previewDiagnostic("nx-preview-tick", "The tick is not one this session keeps.")]);
    }
  }
}

function previewDiagnostic(code: string, message: string): NxIrDiagnostic {
  return { severity: "error", code, message };
}

function scenarioError(message: string): NxIrRuntimeError {
  return new NxIrRuntimeError([previewDiagnostic("nx-preview-scenario", message)]);
}

function isRecord(value: unknown): value is { readonly [key: string]: unknown } {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Whether a batch entry is given by handler: an object with no `$type` and a `handler`. */
function isHandlerEntry(entry: PreviewBatchEntry): entry is PreviewHandlerEntry {
  return isRecord(entry) && entry.$type === undefined && Object.hasOwn(entry, "handler");
}

/** Records the pointer of every `ActionHandler` record in `value` by its token. */
function collectTokens(value: NxCanonicalValue, pointer: string, found: Map<string, string>): void {
  if (Array.isArray(value)) {
    (value as readonly NxCanonicalValue[]).forEach((item, index) => collectTokens(item, `${pointer}/${index}`, found));
  } else if (isRecord(value)) {
    if (value.$type === handlerTypeName && typeof value.token === "string" && !found.has(value.token)) {
      found.set(value.token, pointer);
    }
    for (const [key, item] of Object.entries(value as { readonly [key: string]: NxCanonicalValue })) {
      collectTokens(item, `${pointer}/${escapePointerToken(key)}`, found);
    }
  }
}

/** Checks a scenario's shape, so a malformed one fails before anything runs. */
function readScenario(scenario: PreviewScenario): PreviewScenario {
  if (!isRecord(scenario)) {
    throw scenarioError("A scenario is an object.");
  }
  const { name, module, component, props, batches, handlers } = scenario;
  if (name !== undefined && typeof name !== "string") {
    throw scenarioError("A scenario's 'name' is a string.");
  }
  if (typeof module !== "string" || typeof component !== "string") {
    throw scenarioError("A scenario names its 'module' and 'component' as strings.");
  }
  if (props !== undefined && !isRecord(props)) {
    throw scenarioError("A scenario's 'props' is an object.");
  }
  if (!Array.isArray(batches) || !batches.every((batch) => Array.isArray(batch))) {
    throw scenarioError("A scenario's 'batches' is a list of lists.");
  }
  if (handlers !== undefined) {
    if (!Array.isArray(handlers) || handlers.length !== batches.length) {
      throw scenarioError("A scenario's 'handlers' has one list for each batch.");
    }
    handlers.forEach((pointers, index) => {
      if (
        !Array.isArray(pointers) ||
        pointers.length !== batches[index]?.length ||
        !pointers.every((pointer) => pointer === null || typeof pointer === "string")
      ) {
        throw scenarioError(`The handlers of batch ${index} are not a pointer or null for each of its entries.`);
      }
    });
  }
  return scenario;
}

/** A copy of a value the host passed, frozen to its leaves. */
function frozenCopy<T>(value: T): T {
  return deepFreeze(structuredClone(value));
}

/** Freezes a plain value to its leaves and returns it. */
function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === "object") {
    Object.freeze(value);
    for (const item of Object.values(value)) {
      deepFreeze(item);
    }
  }
  return value;
}
