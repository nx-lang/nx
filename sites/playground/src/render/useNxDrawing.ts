import { useEffect, useRef, useState } from "react";
import type { Compile, Diagnostic } from "../compile";
import { drawRoot, type Dispatch } from "./DrawnTree";
import { catalogModule } from "./catalog";
import { evaluateRoot, prepare } from "./evaluate";
import { InstanceTree, type HostEffect } from "./instances";
import type { NxValue } from "./values";
import type { ReactNode } from "react";

/** How many host effects the drawing keeps; the most recent ones, oldest first. */
const EFFECTS_KEPT = 20;

export interface NxDrawing {
  /** The most recent drawing that worked. Compilation failures leave it standing. */
  readonly node: ReactNode;
  readonly diagnostics: readonly Diagnostic[];
  /**
   * Set when the pipeline itself failed — a transport error, evaluation throwing — or when the
   * most recent dispatch did, in which case the drawing is the one from before the event.
   */
  readonly failure: string | null;
  readonly unknownControls: readonly string[];
  /** Handlers bound where no instance can run them, as `SkiaButton.onTapped`. */
  readonly inertHandlers: readonly string[];
  /** The actions that left the tree since the last compile, oldest first. */
  readonly effects: readonly HostEffect[];
  readonly compiling: boolean;
}

/** A compiled program being drawn: what a dispatch needs to redraw it. */
interface Session {
  readonly tree: InstanceTree;
  readonly root: NxValue;
  effects: HostEffect[];
}

/**
 * Compiles source on a pause in typing and draws the result.
 *
 * Edits are debounced rather than compiled per keystroke: each compile crosses to the worker and
 * builds the visitor's module against the catalog, and an editor that recompiles mid-word makes the
 * canvas flicker through half-written states.
 *
 * A DrawnUI event on a drawn control dispatches through the session's instance tree and redraws
 * from the root without a compile; a recompile replaces the tree, so every instance starts again.
 */
export function useNxDrawing(source: string, compile: Compile, debounceMs = 350): NxDrawing {
  const [drawing, setDrawing] = useState<NxDrawing>({
    node: null,
    diagnostics: [],
    failure: null,
    unknownControls: [],
    inertHandlers: [],
    effects: [],
    compiling: true,
  });
  const lastGood = useRef<ReactNode>(null);
  const session = useRef<Session | null>(null);
  /** The most recent compile's own failure, which a dispatch redraw keeps showing. */
  const pipelineFailure = useRef<string | null>(null);

  /** A compile that failed keeps the standing drawing and its instances, not the effects it listed. */
  function failCompile(diagnostics: readonly Diagnostic[], failure: string | null): void {
    pipelineFailure.current = failure;
    if (session.current !== null) {
      session.current.effects = [];
    }
    setDrawing({
      node: lastGood.current,
      diagnostics,
      failure,
      unknownControls: [],
      inertHandlers: [],
      effects: [],
      compiling: false,
    });
  }

  useEffect(() => {
    let cancelled = false;
    setDrawing((previous) => ({ ...previous, compiling: true }));

    const timer = setTimeout(() => {
      void (async () => {
        let result;
        try {
          result = await compile(source);
        } catch (error) {
          if (!cancelled) {
            failCompile([], error instanceof Error ? error.message : String(error));
          }
          return;
        }
        if (cancelled) {
          return;
        }
        if (result.ir === null) {
          failCompile(result.diagnostics, null);
          return;
        }
        try {
          const program = prepare(result.ir, catalogModule());
          const current: Session = {
            tree: new InstanceTree(program),
            root: evaluateRoot(program),
            effects: [],
          };
          const drawn = draw(current, dispatcherFor(current));
          // Only a session that drew replaces the one the standing drawing dispatches through.
          session.current = current;
          pipelineFailure.current = null;
          setDrawing({ ...drawn, diagnostics: result.diagnostics, failure: null, effects: [], compiling: false });
        } catch (error) {
          failCompile(result.diagnostics, error instanceof Error ? error.message : String(error));
        }
      })();
    }, debounceMs);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source, compile, debounceMs]);

  /** Draws the session from its root, keeping the result as the last good drawing. */
  function draw(current: Session, dispatch: Dispatch): Pick<NxDrawing, "node" | "unknownControls" | "inertHandlers"> {
    const unknown = new Set<string>();
    const inert = new Set<string>();
    const node = drawRoot(current.root, {
      tree: current.tree,
      instance: null,
      dispatch,
      reportUnknown: (type) => unknown.add(type),
      reportInert: (where) => inert.add(where),
    });
    lastGood.current = node;
    return { node, unknownControls: [...unknown], inertHandlers: [...inert] };
  }

  /**
   * The dispatch for one session: a later compile's drawing ignores events from this one. A
   * dispatch redraw changes only the drawing, its effects and its dispatch failure; the
   * diagnostics, the pipeline's own failure, and whether a compile is running belong to the most
   * recent compile, which may have failed since.
   */
  function dispatcherFor(current: Session): Dispatch {
    const dispatch: Dispatch = (origin, token, action) => {
      if (session.current !== current) {
        return;
      }
      try {
        // The dispatch and the redraw change the tree together, so a redraw that throws leaves
        // the instances the standing drawing's callbacks name.
        const { effects, drawn } = current.tree.atomically(() => {
          const effects = current.tree.dispatch(origin, token, action);
          return { effects, drawn: draw(current, dispatch) };
        });
        current.effects = [...current.effects, ...effects].slice(-EFFECTS_KEPT);
        const kept = current.effects;
        setDrawing((previous) => ({ ...previous, ...drawn, effects: kept, failure: pipelineFailure.current }));
      } catch (error) {
        // Every instance is as it was, and so is the drawing.
        setDrawing((previous) => ({ ...previous, failure: error instanceof Error ? error.message : String(error) }));
      }
    };
    return dispatch;
  }

  return drawing;
}
