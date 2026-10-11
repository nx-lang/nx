import type {
  NxCanonicalValue,
  NxComponentInstance,
  NxHostRecord,
  NxHostValue,
  NxIrDiagnostic,
  NxOriginEntry,
  NxPreparedProgram,
} from "@nx-lang/ir-runtime";

/** How a session runs its calls. */
export interface PreviewSessionOptions {
  /** The most operations one call may cost, as the runtime counts them. Absent, calls are unlimited. */
  readonly maxOperations?: number;
  /** The largest input one call may be given, as the runtime measures it. Absent, input is unlimited. */
  readonly maxInputSize?: number;
  /** How deeply function calls may nest. The runtime's default when absent. */
  readonly maxCallDepth?: number;
  /** The most integers one range may hold. The runtime's default when absent. */
  readonly maxRangeLength?: number;
  /**
   * Whether every call passes an origins report, so each tick holds the origin entries of its
   * rendered output. On by default.
   */
  readonly origins?: boolean;
  /**
   * The most ticks the session keeps. When a new tick passes it, the session drops the oldest ticks
   * that are off the current path and have no children, until it is back within the limit or no
   * such tick is left. 1,000 by default.
   */
  readonly maxTicks?: number;
}

/** What made a tick. */
export type PreviewCause =
  /** The first render, from these props. Every root tick has this cause. */
  | { readonly kind: "initial"; readonly props: NxHostRecord }
  /**
   * A batch, as the runtime took it, and for each of its entries the JSON pointer of the handler
   * it invoked in the parent tick's output, or `null` for an entry that invoked no handler there.
   */
  | { readonly kind: "batch"; readonly batch: readonly NxHostValue[]; readonly handlers: readonly (string | null)[] }
  /** New props on the parent tick's state. */
  | { readonly kind: "props"; readonly props: NxHostRecord }
  /** The parent tick's props and state run under a new program. */
  | { readonly kind: "reload" };

/**
 * One snapshot of a session's component: what made it and what it rendered. A tick never changes;
 * its `parent` is the tick it was made from, and a session gives its children.
 */
export interface PreviewTick {
  /** The tick's number in its session, counting from 1 in the order ticks were made. */
  readonly id: number;
  readonly cause: PreviewCause;
  /** The tick this one was made from, or `undefined` for a root. */
  readonly parent: PreviewTick | undefined;
  /** The program the tick was rendered by, which a dispatch from it runs. */
  readonly program: NxPreparedProgram;
  /** The props the component was rendered with, as the host gave them. */
  readonly props: NxHostRecord;
  /** The runtime's instance, which a dispatch from this tick is given. */
  readonly instance: NxComponentInstance;
  readonly state: Readonly<Record<string, NxCanonicalValue>>;
  readonly rendered: NxCanonicalValue;
  /** What the batch's handlers returned for the host, in dispatch order; empty for any other cause. */
  readonly effects: readonly NxCanonicalValue[];
  /** The origin entries of the rendered output, or `undefined` when the session reports no origins. */
  readonly origins: readonly NxOriginEntry[] | undefined;
}

/**
 * A batch entry given by handler: the JSON pointer (RFC 6901) of an `ActionHandler` record in the
 * current tick's rendered output, and the action to invoke it with.
 */
export interface PreviewHandlerEntry {
  readonly handler: string;
  readonly action: NxHostRecord;
}

/**
 * One entry of a batch a host dispatches: an entry as the runtime takes it (an
 * `ActionHandlerInvocation` naming a token, or an action the component emits), or a
 * {@link PreviewHandlerEntry}.
 */
export type PreviewBatchEntry = NxHostValue | PreviewHandlerEntry;

/**
 * A run as a `program.json` lifecycle, with two optional members a conformance runner ignores:
 * `name`, and `handlers`, which holds for each entry of each batch the JSON pointer of the handler
 * it invoked, or `null`.
 */
export interface PreviewScenario {
  readonly name?: string;
  /** The identity of the program's entry module. */
  readonly module: string;
  readonly component: string;
  /** The props of the first render; none when absent. */
  readonly props?: NxHostRecord;
  readonly batches: readonly (readonly NxHostValue[])[];
  readonly handlers?: readonly (readonly (string | null)[])[];
}

/** Where a replay stopped. */
export interface PreviewReplayStop {
  /** The index of the batch that could not be placed or failed. */
  readonly batch: number;
  /**
   * The index of the entry that could not be placed, or `undefined` when the batch was placed and
   * the runtime failed it, or when a props change before it failed.
   */
  readonly entry?: number;
  readonly diagnostics: readonly NxIrDiagnostic[];
}

/** How a replay went. */
export interface PreviewReplayResult {
  /** The last tick the replay made, which is the session's current tick. */
  readonly tick: PreviewTick;
  /** Whether every batch was dispatched. */
  readonly completed: boolean;
  /** Where the replay stopped, when it did not complete. */
  readonly stopped?: PreviewReplayStop;
}

/**
 * How a reload went: the state was kept, the path was replayed, or the new program refused the
 * props and the session kept the old program and its current tick.
 */
export type PreviewReloadResult =
  | { readonly outcome: "kept"; readonly tick: PreviewTick }
  | ({ readonly outcome: "replayed" } & PreviewReplayResult)
  | { readonly outcome: "failed"; readonly diagnostics: readonly NxIrDiagnostic[] };

/** One emitted NX IR image: a module's identity and its bytes, the shape `generateNxIr` returns. */
export interface PreviewImage {
  readonly identity: string;
  readonly bytes: Uint8Array | ArrayBuffer;
}
