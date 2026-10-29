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
  declarationAtOrigin,
  evaluateFunction,
  type NxCanonicalValue,
  type NxHostRecord,
  type NxIrDiagnostic,
  type NxRuntimeOrigins,
  type NxOriginEntry,
  type NxPreparedProgram,
  type NxRuntimeOptions,
} from "@nx-lang/ir-runtime";
import {
  createPreviewSession,
  programFromImages,
  type PreviewSession,
  type PreviewSessionOptions,
  type PreviewTick,
} from "@nx-lang/previewer";
import {
  NxEvaluationError,
  NxHostCrashedError,
  type NxProgramArtifact,
  type NxValueText,
} from "@nx-lang/sdk-wasm";

import {
  FILE_NAME,
  MAX_SOURCE_BYTES,
  classifyDiagnostic,
  valueOutcome,
} from "../compile/policy.ts";
import type { Diagnostic } from "../compile/types.ts";
import { prepareDrawingValue } from "../graphics/prepare.ts";
import { previewerFor } from "./registry.ts";
import type { SourceDeclaration } from "@nx-lang/language-protocol";
import { fieldsOf } from "./fields.ts";
import type {
  PreviewAction,
  PreviewCall,
  PreviewProblem,
  PreviewReloadNotice,
  PreviewSessionView,
  PreviewRenderedOutput,
  PreviewState,
  PreviewStep,
  PreviewValue,
} from "./protocol.ts";
import { SourceIndex } from "./spans.ts";
import { authoredSites, type PreviewSelectionTarget } from "./selection.ts";

/**
 * The runtime's limits for every call a preview makes: calls nest no deeper than evaluation
 * allows, and a handler that never stops runs out of operations well inside the worker's deadline.
 */
export const PREVIEW_LIMITS = {
  maxOperations: 5_000_000,
  maxCallDepth: 200,
  maxRangeLength: 100_000,
  maxInputSize: 1_000_000,
} satisfies PreviewSessionOptions & NxRuntimeOptions;

/** One successful build of the visitor's source: its artifact, and the program linked from it. */
interface Build {
  readonly source: string;
  readonly artifact: NxProgramArtifact;
  program?: NxPreparedProgram;
  root?: { value: NxCanonicalValue; origins: NxRuntimeOrigins } | null;
  visualizers?: readonly SourceDeclaration[];
  index?: SourceIndex;
  readonly actions: Map<string, PreviewAction>;
}

/** What `root` returns, when it is an element of a component the source declares. */
interface RootElement {
  readonly component: string;
  readonly props: Record<string, NxCanonicalValue>;
}

/** Compiles a source the worker has no build of, or answers undefined when it does not compile. */
export type PreviewCompile = (source: string) => NxProgramArtifact | undefined;

/** Runs the preview in the worker. One per worker. */
export class PreviewWorkbench {
  /** The latest build compiled by this workbench. */
  #latest: Build | undefined;
  /**
   * The previous build the session's current step still runs after an edit.
   * Every other superseded build is disposed.
   */
  #other: Build | undefined;
  readonly #compile: PreviewCompile;
  readonly #visualizers:
    | ((source: string) => readonly SourceDeclaration[])
    | undefined;
  #session: PreviewSession | undefined;
  #structured:
    | {
        source: string;
        view: PreviewRenderedOutput;
        value: NxCanonicalValue;
        origins: NxRuntimeOrigins;
        build: Build;
      }
    | undefined;
  #diagnostics: Diagnostic[] = [];
  #requestedSource: string | null = null;
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
   * @param compile builds any source the workbench holds no build of, including edits and
   * sources needed again by an older session step.
   */
  constructor(
    compile: PreviewCompile,
    visualizers?: (source: string) => readonly SourceDeclaration[],
  ) {
    this.#compile = compile;
    this.#visualizers = visualizers;
  }

  /**
   * Retains a freshly compiled artifact and disposes the builds the workbench no longer needs.
   */
  #keepBuild(source: string, artifact: NxProgramArtifact): void {
    const previous = this.#latest;
    this.#latest = { source, artifact, actions: new Map() };
    if (previous === undefined || previous.artifact === artifact) {
      return;
    }
    // The session's current step may still run the build an edit replaced.
    if (
      previous.source !== source &&
      previous.source === this.#runningSource()
    ) {
      this.#other?.artifact.dispose();
      this.#other = previous;
    } else {
      previous.artifact.dispose();
    }
  }

  /**
   * Forgets the builds without disposing them: their host crashed, and the artifacts went with it.
   * The session needs no compiler and keeps running; what it needs one for is built again.
   */
  forgetBuild(): void {
    this.#latest = undefined;
    this.#other = undefined;
  }

  answer(call: PreviewCall): PreviewState {
    this.#problem = null;
    this.#notice = null;
    if (call.op === "start" || call.op === "reload") {
      this.#requestedSource = call.source;
      this.#diagnostics = [];
    }
    try {
      switch (call.op) {
        case "start":
          if (call.reset) {
            this.#structured = undefined;
            this.#session = undefined;
            this.#left.clear();
            this.#component = null;
            this.#components = [];
            this.#fromRoot = false;
            this.#rootProps = "";
            this.#latest?.artifact.dispose();
            this.#other?.artifact.dispose();
            this.forgetBuild();
          }
          this.#start(call.source, call.component);
          break;
        case "reload":
          this.#reload(call.source, call.component);
          break;
        case "dispatch":
          this.#dispatch(call.handler, call.action);
          break;
        case "goTo":
          this.#goTo(call.tick);
          break;
        case "stop":
          this.#structured = undefined;
          this.#session = undefined;
          this.#left.clear();
          break;
      }
    } catch (error) {
      rethrowCrash(error);
      this.#diagnostics = this.#diagnosticsOf(
        error,
        this.#latest?.source === this.#requestedSource
          ? this.#latest
          : undefined,
      );
      this.#problem =
        error instanceof NxEvaluationError
          ? { kind: "doesNotCompile" }
          : { kind: "programFailed", diagnostics: this.#diagnostics };
    }
    return this.#state();
  }

  /** Runs the selected renderer against the exact value already evaluated by this workbench. */
  async render(state: PreviewState, scale = 1): Promise<PreviewState> {
    if (
      state.renderer !== "graphics" ||
      this.#structured === undefined ||
      state.problem !== null
    ) {
      return state;
    }
    const { build, value, origins, source } = this.#structured;
    return {
      ...state,
      drawing: await prepareDrawingValue(
        build.artifact,
        source,
        value,
        origins,
        undefined,
        scale,
      ),
    };
  }

  dispose(): void {
    this.#latest?.artifact.dispose();
    this.#other?.artifact.dispose();
    this.forgetBuild();
    this.#session = undefined;
  }

  /** The source of the program the session's current step runs. */
  #runningSource(): string | undefined {
    const session = this.#session;
    return session === undefined
      ? this.#structured?.source
      : this.#sources.get(session.current.program);
  }

  /** The build of `source`: kept, or compiled now. */
  #buildOf(source: string): Build | undefined {
    if (new TextEncoder().encode(source).length > MAX_SOURCE_BYTES) {
      throw new Error("The source exceeds the playground limit.");
    }
    const kept = [this.#latest, this.#other].find(
      (build) => build?.source === source,
    );
    if (kept !== undefined) {
      return kept;
    }
    const artifact = this.#compile(source);
    if (artifact === undefined) {
      return undefined;
    }
    this.#keepBuild(source, artifact);
    return this.#latest;
  }

  /** The build the session's current step runs, or the latest when that cannot be built. */
  #runningBuild(): Build | undefined {
    const running = this.#runningSource();
    return (
      (running === undefined ? undefined : this.#buildOf(running)) ??
      this.#latest
    );
  }

  /** The build of `source`, linked, or why there is none. */
  #programFor(
    source: string,
  ): { build: Build; program: NxPreparedProgram } | PreviewProblem {
    const build = this.#buildOf(source);
    if (build === undefined) {
      return { kind: "doesNotCompile" };
    }
    try {
      // Every module, so a source that imports a library links against it.
      build.program ??= programFromImages(
        build.artifact.generateNxIr({ debug: true, modules: [] }),
        FILE_NAME,
      );
    } catch (error) {
      rethrowCrash(error);
      return {
        kind: "programFailed",
        diagnostics: this.#diagnosticsOf(error, build),
      };
    }
    this.#sources.set(build.program, source);
    return { build, program: build.program };
  }

  #root(
    build: Build,
    program: NxPreparedProgram,
  ): NonNullable<Build["root"]> | null {
    if (build.root === undefined) {
      if (!program.entry.module.declarationsByName.has("root")) {
        build.root = null;
      } else {
        const origins: NxRuntimeOrigins = { includeCallSites: true };
        build.root = {
          value: evaluateFunction(program, "root", [], {
            ...PREVIEW_LIMITS,
            origins,
          }),
          origins,
        };
      }
    }
    return build.root;
  }

  #start(source: string, component: string | undefined): void {
    const linked = this.#programFor(source);
    if ("kind" in linked) {
      this.#problem = linked;
      return;
    }
    const { build, program } = linked;
    const rootValue = this.#root(build, program);
    const components = (this.#components = componentsOf(program));
    const root =
      rootValue === null
        ? undefined
        : rootElement(
            program,
            components,
            rootValue.value,
            rootValue.origins.entries,
          );
    if (component !== undefined && !components.includes(component)) {
      component = undefined;
    }
    this.#structured = undefined;
    this.#session = undefined;
    this.#left.clear();
    if (
      component === undefined &&
      ((rootValue !== null && root === undefined) || components.length === 0)
    ) {
      this.#component = null;
      this.#problem = rootValue === null ? { kind: "noComponent" } : null;
      if (rootValue !== null) {
        const { value: result, origins } = rootValue;
        const built: number[] = [];
        const selectionTargets: PreviewSelectionTarget[] = [];
        let output: PreviewRenderedOutput["output"];
        try {
          const formatted = valueOutcome(build.artifact.formatValue(result));
          const value = this.#withOrigins(
            formatted.value,
            { origins: origins.entries },
            build,
            built,
            selectionTargets,
          );
          output = { value, truncated: formatted.truncated };
        } catch (error) {
          rethrowCrash(error);
          output = { diagnostics: this.#formatDiagnosticsOf(error, build) };
        }
        this.#structured = {
          source,
          build,
          value: result,
          origins,
          view: { output, built, selectionTargets, handlers: [], actions: {} },
        };
      }
      return;
    }
    const chosen =
      component !== undefined && components.includes(component)
        ? component
        : (root?.component ?? components[0]!);
    this.#component = chosen;
    this.#fromRoot = root?.component === chosen;
    const props = this.#fromRoot ? root!.props : {};
    this.#rootProps = JSON.stringify(props);
    try {
      this.#session = createPreviewSession(
        program,
        chosen,
        props,
        PREVIEW_LIMITS,
      );
    } catch (error) {
      const diagnostics = this.#diagnosticsOf(error, build);
      this.#problem = {
        kind: "startFailed",
        diagnostics,
        missingProp:
          error instanceof NxIrRuntimeError &&
          error.diagnostics.some(isMissingProp),
      };
    }
  }

  #reload(source: string, fallbackComponent?: string): void {
    const session = this.#session;
    if (session === undefined) {
      this.#start(
        source,
        this.#fromRoot ? undefined : (this.#component ?? fallbackComponent),
      );
      return;
    }
    const linked = this.#programFor(source);
    if ("kind" in linked) {
      // The edit does not compile: the session stands, and the page marks it out of date.
      return;
    }
    const { build, program } = linked;
    const components = (this.#components = componentsOf(program));
    const rootValue = this.#root(build, program);
    const root =
      rootValue === null
        ? undefined
        : rootElement(
            program,
            components,
            rootValue.value,
            rootValue.origins.entries,
          );
    const component = this.#component;
    if (this.#fromRoot && rootValue !== null && root === undefined) {
      this.#start(source, undefined);
      return;
    }
    if (
      component === null ||
      !components.includes(component) ||
      (this.#fromRoot && root !== undefined && root.component !== component)
    ) {
      // The component is gone, or `root` now names another one: start over for what is there now.
      this.#start(
        source,
        this.#fromRoot ? undefined : (component ?? undefined),
      );
      return;
    }

    const result = session.reload(program);
    if (result.outcome === "failed") {
      this.#notice = {
        kind: "failed",
        diagnostics: result.diagnostics.map((diagnostic) =>
          this.#diagnostic(diagnostic, build),
        ),
      };
      return;
    }
    if (result.outcome === "replayed") {
      this.#notice = {
        kind: "replayed",
        completed: result.completed,
        steps: session.path.length,
        diagnostics: (result.stopped?.diagnostics ?? []).map((diagnostic) =>
          this.#diagnostic(diagnostic, build),
        ),
      };
    }

    // `root`'s element names the component: new props in it render the state the reload kept.
    if (root?.component === component) {
      this.#fromRoot = true;
      // Props that fail stay unapplied, so every later reload tries them again and says so.
      const props = JSON.stringify(root.props);
      if (props !== this.#rootProps) {
        try {
          session.setProps(root.props);
          this.#rootProps = props;
        } catch (error) {
          this.#notice = {
            kind: "failed",
            diagnostics: this.#diagnosticsOf(error, build),
          };
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
    } catch (error) {
      this.#problem = {
        kind: "dispatchFailed",
        diagnostics: this.#diagnosticsOf(error, this.#runningBuild()),
      };
    }
  }

  #goTo(id: number): void {
    const session = this.#session;
    const target = session?.ticks.find((tick) => tick.id === id);
    if (session === undefined || target === undefined) {
      return;
    }
    // Going back past a tick remembers which child led on, for Forward.
    for (
      let tick: PreviewTick | undefined = session.current;
      tick !== undefined && tick !== target;
      tick = tick.parent
    ) {
      if (tick.parent !== undefined) {
        this.#left.set(tick.parent.id, tick.id);
      }
    }
    session.goTo(target);
  }

  #presentationMetadata(build: Build): readonly SourceDeclaration[] {
    if (build.visualizers === undefined) {
      try {
        build.visualizers = this.#visualizers?.(build.source) ?? [];
      } catch (error) {
        rethrowCrash(error);
        // A missing presentation must not discard the evaluated value or repeat the failed query.
        build.visualizers = [];
      }
    }
    return build.visualizers;
  }

  #state(): PreviewState {
    const session = this.#session;
    const build = this.#runningBuild();
    const view =
      session === undefined || build === undefined
        ? null
        : this.#view(session, build);
    const structured = this.#structured;
    return {
      renderer:
        session !== undefined ||
        (this.#component !== null && this.#components.includes(this.#component))
          ? "component"
          : structured?.build.program === undefined
            ? null
            : previewerFor(
                structured.build.program,
                structured.value,
                structured.origins.entries ?? [],
                this.#presentationMetadata(structured.build),
              ),
      diagnostics: [
        ...(build?.source === this.#requestedSource
          ? build.artifact.diagnostics().map(classifyDiagnostic)
          : []),
        ...this.#diagnostics,
      ],
      diagnosticsSource: this.#requestedSource,
      hasRoot:
        build?.source === this.#requestedSource
          ? build.root !== null
          : undefined,
      source:
        session === undefined
          ? (this.#structured?.source ?? build?.source ?? null)
          : (this.#runningSource() ?? null),
      components: this.#components,
      component: this.#component,
      problem: this.#problem,
      session: view,
      structured: this.#structured?.view ?? null,
      notice: this.#notice,
    };
  }

  #view(session: PreviewSession, build: Build): PreviewSessionView {
    const current = session.current;
    const path: PreviewStep[] = session.path.map((tick) => ({
      id: tick.id,
      label: labelOf(tick),
      kind: tick.cause.kind,
      children: session
        .childrenOf(tick)
        .map((child) => ({ id: child.id, label: labelOf(child) })),
    }));
    const children = session.childrenOf(current);
    const left = this.#left.get(current.id);
    const forward =
      children.find((child) => child.id === left)?.id ??
      children.at(-1)?.id ??
      null;

    // Spans index the source the current tick's program was built from. The build is that source's
    // unless it no longer compiles here, which only a replaced host without the source can cause.
    const sameSource = this.#sources.get(current.program) === build.source;
    let output: PreviewSessionView["output"];
    const built: number[] = [];
    const selectionTargets: PreviewSelectionTarget[] = [];
    try {
      const formatted = valueOutcome(
        build.artifact.formatValue(current.rendered),
      );
      const value = sameSource
        ? this.#withOrigins(
            formatted.value,
            current,
            build,
            built,
            selectionTargets,
          )
        : formatted.value;
      output = { value, truncated: formatted.truncated };
    } catch (error) {
      rethrowCrash(error);
      output = { diagnostics: this.#formatDiagnosticsOf(error, build) };
    }

    const handlers = handlersIn(current.rendered);
    const actions: Record<string, PreviewAction> = {};
    for (const { action } of handlers) {
      actions[action] ??= this.#action(build, action);
    }

    const state =
      Object.keys(current.state).length === 0 || this.#component === null
        ? null
        : this.#format(build, current.state, this.#component);
    const emitted = current.effects.flatMap((effect) => {
      const value = this.#format(build, effect);
      return value === null ? [] : [value];
    });
    return {
      path,
      current: current.id,
      forward,
      output,
      built,
      selectionTargets,
      state,
      emitted,
      handlers,
      actions,
    };
  }

  /** `value` with each record built from source declared where it was built. */
  #withOrigins(
    value: NxValueText,
    tick: Pick<PreviewTick, "origins">,
    build: Build,
    built: number[],
    targets: PreviewSelectionTarget[],
  ): NxValueText {
    const index = (build.index ??= new SourceIndex(build.source));
    const origins = new Map<string, ReturnType<typeof authoredSites>>();
    for (const entry of tick.origins ?? []) {
      const sites = authoredSites(entry, FILE_NAME);
      if (sites.length > 0) {
        origins.set(entry.path, sites);
      }
    }
    return {
      text: value.text,
      nodes: value.nodes.map((node, position) => {
        const sites =
          node.role === "record" ? origins.get(node.pointer) : undefined;
        const origin = sites?.[0];
        const span =
          origin === undefined ? null : index.span(origin.start, origin.end);
        if (span === null) {
          return node;
        }
        built.push(position);
        targets.push({ id: String(position), sources: sites! });
        return { ...node, declaration: span };
      }),
    };
  }

  #format(build: Build, value: unknown, state?: string): PreviewValue | null {
    try {
      const formatted = valueOutcome(
        build.artifact.formatValue(value, state === undefined ? {} : { state }),
      );
      return formatted.kind === "value"
        ? { value: formatted.value, truncated: formatted.truncated }
        : null;
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
        fields = fieldsOf(
          build.artifact.typeSchema({ name: type }, { direction: "input" }),
        );
      } catch (error) {
        rethrowCrash(error);
        // An action the program cannot describe is dispatched with no fields.
      }
      action = { type, label: type.split(".").at(-1)!, fields };
      build.actions.set(type, action);
    }
    return action;
  }

  #formatDiagnosticsOf(error: unknown, build: Build): Diagnostic[] {
    return this.#diagnosticsOf(error, build).map((item) =>
      item.code === "nx-text-unspellable"
        ? item
        : {
            ...item,
            code: "nx-output-format",
            message: /recursion limit exceeded/.test(item.message)
              ? "The value is too deeply nested to display as NX text."
              : item.message,
          },
    );
  }

  #diagnosticsOf(error: unknown, build: Build | undefined): Diagnostic[] {
    if (error instanceof NxIrRuntimeError) {
      return error.diagnostics.map((diagnostic) =>
        this.#diagnostic(diagnostic, build),
      );
    }
    if (error instanceof NxEvaluationError) {
      return error.diagnostics.map(classifyDiagnostic);
    }
    return [
      {
        severity: "error",
        message: error instanceof Error ? error.message : String(error),
        origin: "program",
        span: null,
      },
    ];
  }

  /** A runtime diagnostic, marked in the source when it points into the visitor's module. */
  #diagnostic(
    diagnostic: NxIrDiagnostic,
    build: Build | undefined,
  ): Diagnostic {
    const where = diagnostic.source;
    const index =
      build === undefined
        ? undefined
        : (build.index ??= new SourceIndex(build.source));
    const span =
      where !== undefined && where.identity === FILE_NAME
        ? (index?.span(where.start, where.end) ?? null)
        : null;
    return {
      severity: "error",
      code: diagnostic.code,
      message: diagnostic.message,
      origin: span === null ? "program" : "source",
      span,
    };
  }
}

/** The components a preview can run: those the entry module declares that are neither external nor abstract. */
export function componentsOf(program: NxPreparedProgram): string[] {
  return [...program.componentEntrypoints.values()]
    .filter(
      (declaration) =>
        declaration.module === program.entry.module &&
        declaration.kind.tag === "component" &&
        !declaration.kind.isExternal &&
        !declaration.kind.isAbstract,
    )
    .sort((first, second) => first.index - second.index)
    .map((declaration) => declaration.name);
}

/** What `root` returns, when that is an element of one of `components`. */
export function rootElement(
  program: NxPreparedProgram,
  components: readonly string[],
  evaluated?: NxCanonicalValue,
  entries?: readonly NxOriginEntry[],
): RootElement | undefined {
  let value: NxCanonicalValue;
  try {
    const origins: NxRuntimeOrigins = { includeCallSites: true };
    value =
      evaluated === undefined
        ? evaluateFunction(program, "root", [], { ...PREVIEW_LIMITS, origins })
        : evaluated;
    entries ??= origins.entries;
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return undefined;
  }
  const { $type, ...props } = value as Record<string, NxCanonicalValue>;
  const origin = entries?.find((entry) => entry.path === "");
  const declaration =
    origin === undefined ? undefined : declarationAtOrigin(program, origin);
  return typeof $type === "string" &&
    declaration?.module === program.entry.module &&
    components.includes(declaration.name)
    ? { component: declaration.name, props }
    : undefined;
}

/** Every `ActionHandler` record in `value`, by pointer, in document order. */
export function handlersIn(
  value: NxCanonicalValue,
): { pointer: string; action: string }[] {
  const found: { pointer: string; action: string }[] = [];
  const walk = (node: unknown, pointer: string) => {
    if (Array.isArray(node)) {
      node.forEach((item, index) => walk(item, `${pointer}/${index}`));
    } else if (typeof node === "object" && node !== null) {
      const record = node as Record<string, unknown>;
      if (
        record["$type"] === "ActionHandler" &&
        typeof record["action"] === "string"
      ) {
        found.push({ pointer, action: record["action"] });
        return;
      }
      for (const [key, child] of Object.entries(record)) {
        walk(
          child,
          `${pointer}/${key.replaceAll("~", "~0").replaceAll("/", "~1")}`,
        );
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
          const action = (
            record["$type"] === "ActionHandlerInvocation"
              ? record["action"]
              : record
          ) as Record<string, unknown> | undefined;
          const type =
            typeof action?.["$type"] === "string" ? action["$type"] : "Action";
          return type.split(".").at(-1)!;
        })
        .join(", ");
  }
}

/** Whether the runtime refused props for leaving out one the component requires. */
function isMissingProp(diagnostic: NxIrDiagnostic): boolean {
  return (
    diagnostic.code === "nx-ir-boundary-field" &&
    /^Missing required /.test(diagnostic.message)
  );
}

/** Lets a crashed host through to the worker, which replaces it. */
function rethrowCrash(error: unknown): void {
  if (error instanceof NxHostCrashedError) {
    throw error;
  }
}
