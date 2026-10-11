/**
 * What the page and the worker's preview say to each other.
 *
 * <para>The session lives in the worker, beside the compiler, so a handler that runs long runs off
 * the page. Every call answers with the whole of what the page draws, built from the current tick,
 * so the page keeps no tick of its own.</para>
 */
import type { NxHostRecord } from "@nx-lang/ir-runtime";
import type { NxTextSpan, NxValueText } from "@nx-lang/sdk-wasm";

import type { Diagnostic } from "../compile/types.ts";

/** A request to the worker's preview. */
export type PreviewCall =
  /**
   * Starts a session on the build of `source`: for `component` when given, otherwise for the
   * component `root` returns an element of, otherwise for the first component the source declares.
   */
  | { readonly op: "start"; readonly source: string; readonly component?: string }
  /** Dispatches `action` to the handler at `handler`, a JSON pointer into the current output. */
  | { readonly op: "dispatch"; readonly handler: string; readonly action: NxHostRecord }
  /** Makes the tick `tick` current. Nothing runs. */
  | { readonly op: "goTo"; readonly tick: number }
  /** Reloads the session with the build of `source`, following new props from `root`. */
  | { readonly op: "reload"; readonly source: string }
  /** Drops the session. */
  | { readonly op: "stop" };

/** Why the preview has nothing, or less than it had, to show. */
export type PreviewProblem =
  /** No build of the source asked for: it does not compile. */
  | { readonly kind: "doesNotCompile" }
  /** The source declares no component to run, or only external ones. */
  | { readonly kind: "noComponent" }
  /** The component would not start, with the runtime's diagnostics; `missingProp` when a prop is required. */
  | { readonly kind: "startFailed"; readonly diagnostics: readonly Diagnostic[]; readonly missingProp: boolean }
  /** The last dispatch failed and added no step. */
  | { readonly kind: "dispatchFailed"; readonly diagnostics: readonly Diagnostic[] };

/** What the last reload did, when it was not simply keeping the state. */
export type PreviewReloadNotice =
  /**
   * The state no longer fit, so the run was replayed under the edit: `steps` steps, and when it did
   * not complete, the next one failed with `diagnostics`.
   */
  | { readonly kind: "replayed"; readonly completed: boolean; readonly steps: number; readonly diagnostics: readonly Diagnostic[] }
  /** The new program could not render the current props: the old one is still running. */
  | { readonly kind: "failed"; readonly diagnostics: readonly Diagnostic[] };

/** One step of the current path, and where else its parent's run went. */
export interface PreviewStep {
  readonly id: number;
  /** What made it, in a word or two: `Start`, the action dispatched, `Props`, `Edit`. */
  readonly label: string;
  readonly kind: "initial" | "batch" | "props" | "reload";
  /** The steps that follow this one in every run kept, oldest first. */
  readonly children: readonly { readonly id: number; readonly label: string }[];
}

/** One field of an action's form, from the action's input schema. */
export type PreviewField =
  | { readonly name: string; readonly required: boolean; readonly kind: "string"; readonly default?: string }
  | { readonly name: string; readonly required: boolean; readonly kind: "integer" | "number"; readonly default?: number }
  | { readonly name: string; readonly required: boolean; readonly kind: "boolean"; readonly default?: boolean }
  | { readonly name: string; readonly required: boolean; readonly kind: "cases"; readonly cases: readonly string[]; readonly default?: string }
  | { readonly name: string; readonly required: boolean; readonly kind: "json"; readonly default?: unknown };

/** An action the output's handlers take, and what its form asks for. */
export interface PreviewAction {
  /** The action's type, `Button.Tapped`, which the dispatched record carries as `$type`. */
  readonly type: string;
  /** Its own name, `Tapped`, which its buttons say. */
  readonly label: string;
  readonly fields: readonly PreviewField[];
}

/** A value as the page shows it, cut to the output limit. */
export interface PreviewValue {
  readonly value: NxValueText;
  readonly truncated: boolean;
}

/** What a running session shows. */
export interface PreviewSessionView {
  /** The current path, from its root to the current step. */
  readonly path: readonly PreviewStep[];
  readonly current: number;
  /** The step Forward goes to: the child last left, or the newest child; null at the end. */
  readonly forward: number | null;
  /** The current output, or why it has no spelling. */
  readonly output: PreviewValue | { readonly diagnostics: readonly Diagnostic[] };
  /** Indices of the output's record nodes whose `declaration` is where the record was built. */
  readonly built: readonly number[];
  /** The state, one `name=value` line per field, or null for a component that keeps none. */
  readonly state: PreviewValue | null;
  /** What the current step's handlers returned for the host. */
  readonly emitted: readonly PreviewValue[];
  /** Each handler in the output, by pointer, and the action it takes. */
  readonly handlers: readonly { readonly pointer: string; readonly action: string }[];
  /** The actions those handlers take, by type. */
  readonly actions: Readonly<Record<string, PreviewAction>>;
}

/** Everything the page draws for the preview. */
export interface PreviewState {
  /** The source whose build runs the current step; spans index it. Null with no build. */
  readonly source: string | null;
  /** The components a preview can run, in declaration order. */
  readonly components: readonly string[];
  /** The component running, or that was asked for. */
  readonly component: string | null;
  readonly problem: PreviewProblem | null;
  readonly session: PreviewSessionView | null;
  readonly notice: PreviewReloadNotice | null;
}

/** A span of the source, as the page marks and selects it. */
export type PreviewSpan = NxTextSpan;
