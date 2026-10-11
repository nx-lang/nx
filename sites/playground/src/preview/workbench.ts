/**
 * The preview as the worker runs it: one preview session of a component of the visitor's source,
 * on the latest build the worker made, and what the page draws of it.
 *
 * <para>Kept apart from the worker shell so Node tests drive it with the module the browser loads.
 * The compiler is only asked to emit NX IR, spell values and describe actions; the session itself
 * runs on `@nx-lang/ir-runtime` through `@nx-lang/previewer`.</para>
 */
import {
  NxIrRuntimeError,
  evaluateFunction,
  type NxCanonicalValue,
  type NxHostRecord,
  type NxIrDiagnostic,
  type NxPreparedProgram,
  type NxRuntimeOptions
} from "@nx-lang/ir-runtime";
import {
  createPreviewSession,
  programFromImages,
  type PreviewSession,
  type PreviewSessionOptions,
  type PreviewTick
} from "@nx-lang/previewer";
import { NxEvaluationError, NxHostCrashedError, type NxProgramArtifact, type NxValueText } from "@nx-lang/sdk-wasm";

import { FILE_NAME, classifyDiagnostic, valueOutcome } from "../compile/evaluate.ts";
import type { Diagnostic } from "../compile/types.ts";
import { fieldsOf } from "./fields.ts";
import type {
  PreviewAction,
  PreviewCall,
  PreviewProblem,
  PreviewReloadNotice,
  PreviewSessionView,
  PreviewState,
  PreviewStep,
  PreviewValue
} from "./protocol.ts";
import { SourceIndex } from "./spans.ts";

/**
 * The runtime's limits for every call a preview makes: calls nest no deeper than evaluation
 * allows, and a handler that never stops runs out of operations well inside the worker's deadline.
 */
export const PREVIEW_LIMITS = {
  maxOperations: 5_000_000,
  maxCallDepth: 200,
  maxRangeLength: 100_000,
  maxInputSize: 1_000_000
} satisfies PreviewSessionOptions & NxRuntimeOptions;

/** One successful build of the visitor's source: its artifact, and the program linked from it. */
interface Build {
  readonly source: string;
  readonly artifact: NxProgramArtifact;
  program?: NxPreparedProgram;
  index?: SourceIndex;
  readonly actions: Map<string, PreviewAction>;
}

/** What `root` returns, when it is an element of a component the source declares. */
interface RootElement {
  readonly component: string;
  readonly props: Record<string, NxCanonicalValue>;
}

/** Runs the preview in the worker. One per worker. */
export class PreviewWorkbench {
  #build: Build | undefined;
  #session: PreviewSession | undefined;
  #component: string | null = null;
  /** The components of the build the preview last linked. */
  #components: string[] = [];
  /** Whether the session's component and props are the element `root` returns. */
  #fromRoot = false;
  /** The props `root` gave, as JSON, to tell when an edit changed them. */
  #rootProps = "";
  /** The source each program the session has run was built from. */
  readonly #sources = new WeakMap<NxPreparedProgram, string>();
  /** For a tick the visitor went back past, the child they left it through. */
  readonly #left = new Map<number, number>();
  #problem: PreviewProblem | null = null;
  #notice: PreviewReloadNotice | null = null;

  /**
   * Takes the artifact of a build that compiled, for the preview's next call. The workbench owns it
   * from now on and disposes the one before.
   */
  keepBuild(source: string, artifact: NxProgramArtifact): void {
    if (this.#build !== undefined && this.#build.artifact !== artifact) {
      this.#build.artifact.dispose();
    }
    this.#build = { source, artifact, actions: new Map() };
  }

  /**
   * Forgets the build without disposing it: its host crashed, and the artifact went with it. The
   * session needs no compiler and keeps running; the next build brings back what does.
   */
  forgetBuild(): void {
    this.#build = undefined;
  }

  answer(call: PreviewCall): PreviewState {
    this.#problem = null;
    this.#notice = null;
    switch (call.op) {
      case "start":
        this.#start(call.source, call.component);
        break;
      case "reload":
        this.#reload(call.source);
        break;
      case "dispatch":
        this.#dispatch(call.handler, call.action);
        break;
      case "goTo":
        this.#goTo(call.tick);
        break;
      case "stop":
        this.#session = undefined;
        this.#left.clear();
        break;
    }
    return this.#state();
  }

  dispose(): void {
    this.#build?.artifact.dispose();
    this.#build = undefined;
    this.#session = undefined;
  }

  /** The build of `source`, linked, or why there is none. */
  #programFor(source: string): { build: Build; program: NxPreparedProgram } | PreviewProblem {
    const build = this.#build;
    if (build === undefined || build.source !== source) {
      return { kind: "doesNotCompile" };
    }
    try {
      build.program ??= programFromImages(build.artifact.generateNxIr({ debug: true }), FILE_NAME);
    } catch (error) {
      rethrowCrash(error);
      return { kind: "startFailed", diagnostics: this.#diagnostics(error, build), missingProp: false };
    }
    this.#sources.set(build.program, source);
    return { build, program: build.program };
  }

  #start(source: string, component: string | undefined): void {
    this.#session = undefined;
    this.#left.clear();
    const linked = this.#programFor(source);
    if ("kind" in linked) {
      this.#problem = linked;
      return;
    }
    const { build, program } = linked;
    const components = (this.#components = componentsOf(program));
    if (components.length === 0) {
      this.#component = null;
      this.#problem = { kind: "noComponent" };
      return;
    }
    const root = rootElement(program, components);
    const chosen = component !== undefined && components.includes(component)
      ? component
      : root?.component ?? components[0]!;
    this.#component = chosen;
    this.#fromRoot = root?.component === chosen;
    const props = this.#fromRoot ? root!.props : {};
    this.#rootProps = JSON.stringify(props);
    try {
      this.#session = createPreviewSession(program, chosen, props, PREVIEW_LIMITS);
    } catch (error) {
      const diagnostics = this.#diagnostics(error, build);
      this.#problem = {
        kind: "startFailed",
        diagnostics,
        missingProp: error instanceof NxIrRuntimeError && error.diagnostics.some(isMissingProp)
      };
    }
  }

  #reload(source: string): void {
    const session = this.#session;
    if (session === undefined) {
      this.#start(source, this.#component ?? undefined);
      return;
    }
    const linked = this.#programFor(source);
    if ("kind" in linked) {
      // The edit does not compile: the session stands, and the page marks it out of date.
      return;
    }
    const { build, program } = linked;
    const components = (this.#components = componentsOf(program));
    const root = rootElement(program, components);
    const component = this.#component;
    if (component === null || !components.includes(component) || (this.#fromRoot && root !== undefined && root.component !== component)) {
      // The component is gone, or `root` now names another one: start over for what is there now.
      this.#start(source, this.#fromRoot ? undefined : component ?? undefined);
      return;
    }

    const result = session.reload(program);
    if (result.outcome === "failed") {
      this.#notice = { kind: "failed", diagnostics: result.diagnostics.map((diagnostic) => this.#diagnostic(diagnostic, build)) };
      return;
    }
    if (result.outcome === "replayed") {
      this.#notice = {
        kind: "replayed",
        completed: result.completed,
        steps: session.path.length,
        diagnostics: (result.stopped?.diagnostics ?? []).map((diagnostic) => this.#diagnostic(diagnostic, build))
      };
    }
    this.#left.clear();

    // `root`'s element names the component: new props in it render the state the reload kept.
    if (root?.component === component) {
      this.#fromRoot = true;
      const props = JSON.stringify(root.props);
      if (props !== this.#rootProps) {
        this.#rootProps = props;
        try {
          session.setProps(root.props);
        } catch (error) {
          this.#notice = { kind: "failed", diagnostics: this.#diagnostics(error, build) };
        }
      }
    }
  }

  #dispatch(handler: string, action: NxHostRecord): void {
    const session = this.#session;
    if (session === undefined) {
      return;
    }
    try {
      session.dispatchHandler(handler, action);
      this.#left.clear();
    } catch (error) {
      this.#problem = { kind: "dispatchFailed", diagnostics: this.#diagnostics(error, this.#build) };
    }
  }

  #goTo(id: number): void {
    const session = this.#session;
    const target = session?.ticks.find((tick) => tick.id === id);
    if (session === undefined || target === undefined) {
      return;
    }
    // Going back past a tick remembers which child led on, for Forward.
    for (let tick: PreviewTick | undefined = session.current; tick !== undefined && tick !== target; tick = tick.parent) {
      if (tick.parent !== undefined) {
        this.#left.set(tick.parent.id, tick.id);
      }
    }
    session.goTo(target);
  }

  #state(): PreviewState {
    const build = this.#build;
    const session = this.#session;
    const view = session === undefined || build === undefined ? null : this.#view(session, build);
    return {
      source: session === undefined ? (build?.source ?? null) : (this.#sources.get(session.current.program) ?? null),
      components: this.#components,
      component: this.#component,
      problem: this.#problem,
      session: view,
      notice: this.#notice
    };
  }

  #view(session: PreviewSession, build: Build): PreviewSessionView {
    const current = session.current;
    const path: PreviewStep[] = session.path.map((tick) => ({
      id: tick.id,
      label: labelOf(tick),
      kind: tick.cause.kind,
      children: session.childrenOf(tick).map((child) => ({ id: child.id, label: labelOf(child) }))
    }));
    const children = session.childrenOf(current);
    const left = this.#left.get(current.id);
    const forward = children.find((child) => child.id === left)?.id ?? children.at(-1)?.id ?? null;

    // Spans index the source the current tick's program was built from, which is the latest build
    // unless an edit since failed to reload.
    const sameSource = this.#sources.get(current.program) === build.source;
    let output: PreviewSessionView["output"];
    const built: number[] = [];
    try {
      const formatted = valueOutcome(build.artifact.formatValue(current.rendered));
      if (formatted.kind !== "value") {
        throw new Error("unreachable");
      }
      const value = sameSource ? this.#withOrigins(formatted.value, current, build, built) : formatted.value;
      output = { value, truncated: formatted.truncated };
    } catch (error) {
      rethrowCrash(error);
      output = { diagnostics: this.#diagnostics(error, build) };
    }

    const handlers = handlersIn(current.rendered);
    const actions: Record<string, PreviewAction> = {};
    for (const { action } of handlers) {
      actions[action] ??= this.#action(build, action);
    }

    const state = Object.keys(current.state).length === 0 || this.#component === null
      ? null
      : this.#format(build, current.state, this.#component);
    const emitted = current.effects.flatMap((effect) => {
      const value = this.#format(build, effect);
      return value === null ? [] : [value];
    });
    return { path, current: current.id, forward, output, built, state, emitted, handlers, actions };
  }

  /** `value` with each record built from source declared where it was built. */
  #withOrigins(value: NxValueText, tick: PreviewTick, build: Build, built: number[]): NxValueText {
    const index = (build.index ??= new SourceIndex(build.source));
    const origins = new Map<string, { start: number; end: number }>();
    for (const entry of tick.origins ?? []) {
      if (entry.module === FILE_NAME) {
        origins.set(entry.path, entry);
      }
    }
    return {
      text: value.text,
      nodes: value.nodes.map((node, position) => {
        const origin = node.role === "record" ? origins.get(node.pointer) : undefined;
        const span = origin === undefined ? null : index.span(origin.start, origin.end);
        if (span === null) {
          return node;
        }
        built.push(position);
        return { ...node, declaration: span };
      })
    };
  }

  #format(build: Build, value: unknown, state?: string): PreviewValue | null {
    try {
      const formatted = valueOutcome(build.artifact.formatValue(value, state === undefined ? {} : { state }));
      return formatted.kind === "value" ? { value: formatted.value, truncated: formatted.truncated } : null;
    } catch (error) {
      rethrowCrash(error);
      return null;
    }
  }

  #action(build: Build, type: string): PreviewAction {
    let action = build.actions.get(type);
    if (action === undefined) {
      let fields: PreviewAction["fields"] = [];
      try {
        fields = fieldsOf(build.artifact.typeSchema({ name: type }, { direction: "input" }));
      } catch (error) {
        rethrowCrash(error);
        // An action the program cannot describe is dispatched with no fields.
      }
      action = { type, label: type.split(".").at(-1)!, fields };
      build.actions.set(type, action);
    }
    return action;
  }

  #diagnostics(error: unknown, build: Build | undefined): Diagnostic[] {
    if (error instanceof NxIrRuntimeError) {
      return error.diagnostics.map((diagnostic) => this.#diagnostic(diagnostic, build));
    }
    if (error instanceof NxEvaluationError) {
      return error.diagnostics.map(classifyDiagnostic);
    }
    return [{ severity: "error", message: error instanceof Error ? error.message : String(error), origin: "program", span: null }];
  }

  /** A runtime diagnostic, marked in the source when it points into the visitor's module. */
  #diagnostic(diagnostic: NxIrDiagnostic, build: Build | undefined): Diagnostic {
    const where = diagnostic.source;
    const index = build === undefined ? undefined : (build.index ??= new SourceIndex(build.source));
    const span = where !== undefined && where.identity === FILE_NAME ? index?.span(where.start, where.end) ?? null : null;
    return {
      severity: "error",
      code: diagnostic.code,
      message: diagnostic.message,
      origin: span === null ? "program" : "source",
      span
    };
  }
}

/** The components a preview can run: those the entry module declares that are neither external nor abstract. */
export function componentsOf(program: NxPreparedProgram): string[] {
  return [...program.componentEntrypoints.values()]
    .filter((declaration) => declaration.module === program.entry.module && declaration.kind.tag === "component" && !declaration.kind.isExternal && !declaration.kind.isAbstract)
    .sort((first, second) => first.index - second.index)
    .map((declaration) => declaration.name);
}

/** What `root` returns, when that is an element of one of `components`. */
export function rootElement(program: NxPreparedProgram, components: readonly string[]): RootElement | undefined {
  let value: NxCanonicalValue;
  try {
    value = evaluateFunction(program, "root", [], PREVIEW_LIMITS);
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return undefined;
  }
  const { $type, ...props } = value as Record<string, NxCanonicalValue>;
  return typeof $type === "string" && components.includes($type) ? { component: $type, props } : undefined;
}

/** Every `ActionHandler` record in `value`, by pointer, in document order. */
export function handlersIn(value: NxCanonicalValue): { pointer: string; action: string }[] {
  const found: { pointer: string; action: string }[] = [];
  const walk = (node: unknown, pointer: string) => {
    if (Array.isArray(node)) {
      node.forEach((item, index) => walk(item, `${pointer}/${index}`));
    } else if (typeof node === "object" && node !== null) {
      const record = node as Record<string, unknown>;
      if (record["$type"] === "ActionHandler" && typeof record["action"] === "string") {
        found.push({ pointer, action: record["action"] });
        return;
      }
      for (const [key, child] of Object.entries(record)) {
        walk(child, `${pointer}/${key.replaceAll("~", "~0").replaceAll("/", "~1")}`);
      }
    }
  };
  walk(value, "");
  return found;
}

/** What made a tick, in a word or two. */
export function labelOf(tick: PreviewTick): string {
  switch (tick.cause.kind) {
    case "initial":
      return "Start";
    case "props":
      return "Props";
    case "reload":
      return "Edit";
    case "batch":
      return tick.cause.batch
        .map((entry) => {
          const record = entry as Record<string, unknown>;
          const action = (record["$type"] === "ActionHandlerInvocation" ? record["action"] : record) as Record<string, unknown> | undefined;
          const type = typeof action?.["$type"] === "string" ? action["$type"] : "Action";
          return type.split(".").at(-1)!;
        })
        .join(", ");
  }
}

/** Whether the runtime refused props for leaving out one the component requires. */
function isMissingProp(diagnostic: NxIrDiagnostic): boolean {
  return diagnostic.code === "nx-ir-boundary-field" && /^Missing required /.test(diagnostic.message);
}

/** Lets a crashed host through to the worker, which replaces it. */
function rethrowCrash(error: unknown): void {
  if (error instanceof NxHostCrashedError) {
    throw error;
  }
}
