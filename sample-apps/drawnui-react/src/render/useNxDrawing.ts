import { useEffect, useRef, useState } from "react";
import type { Compile, Diagnostic } from "../compile";
import { drawValue } from "./DrawnTree";
import { evaluateRoot, prepare } from "./evaluate";
import type { ReactNode } from "react";

export interface NxDrawing {
  /** The most recent drawing that worked. Compilation failures leave it standing. */
  readonly node: ReactNode;
  readonly diagnostics: readonly Diagnostic[];
  /** Set when the pipeline itself failed — a transport error, or evaluation throwing. */
  readonly failure: string | null;
  readonly unknownControls: readonly string[];
  readonly compiling: boolean;
}

/**
 * Compiles source on a pause in typing and draws the result.
 *
 * Edits are debounced rather than compiled per keystroke: each compile crosses the network and
 * builds a program of ~600 lines, and a fiddle that recompiles mid-word makes the canvas flicker
 * through half-written states.
 */
export function useNxDrawing(source: string, compile: Compile, debounceMs = 350): NxDrawing {
  const [drawing, setDrawing] = useState<NxDrawing>({
    node: null,
    diagnostics: [],
    failure: null,
    unknownControls: [],
    compiling: true,
  });
  const lastGood = useRef<ReactNode>(null);

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
            setDrawing({
              node: lastGood.current,
              diagnostics: [],
              failure: error instanceof Error ? error.message : String(error),
              unknownControls: [],
              compiling: false,
            });
          }
          return;
        }
        if (cancelled) {
          return;
        }
        if (result.ir === null) {
          setDrawing({
            node: lastGood.current,
            diagnostics: result.diagnostics,
            failure: null,
            unknownControls: [],
            compiling: false,
          });
          return;
        }
        const unknown = new Set<string>();
        try {
          const program = prepare(result.ir);
          const node = drawValue(evaluateRoot(program), "root", {
            program,
            report: (type) => unknown.add(type),
          });
          lastGood.current = node;
          setDrawing({
            node,
            diagnostics: result.diagnostics,
            failure: null,
            unknownControls: [...unknown],
            compiling: false,
          });
        } catch (error) {
          setDrawing({
            node: lastGood.current,
            diagnostics: result.diagnostics,
            failure: error instanceof Error ? error.message : String(error),
            unknownControls: [...unknown],
            compiling: false,
          });
        }
      })();
    }, debounceMs);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source, compile, debounceMs]);

  return drawing;
}
