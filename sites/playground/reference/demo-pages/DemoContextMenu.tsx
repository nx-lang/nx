import { useEffect } from "react";
import { useShell } from "drawnui-react";
import type { ContextMenuEventArgs } from "drawnui-react/core";
import pkg from "../../../package.json";

/**
 * The demo's Canvas-level ContextMenu fallback: a right click (or a long press, or the Menu key) that no control
 * took shows a drawn toast with the library versions instead of the browser's "Save image as…" menu.
 * Controls with their own ContextMenu handler (the Shapes page card) still win, they are asked first.
 */

type Handler = (e: ContextMenuEventArgs) => boolean;
let current: Handler | null = null;

/** Wire this to `<Canvas ContextMenu={(_, e) => handleContextMenu(e)}>`. */
export function handleContextMenu(e: ContextMenuEventArgs): boolean {
  return current ? current(e) : false;
}

const CANVASKIT = (pkg.dependencies["canvaskit-wasm"] ?? "").replace(/^[\^~]/, "");

/** Mount once inside SkiaShell (it needs the shell for the toast); renders nothing itself. */
export function DemoContextMenu() {
  const shell = useShell();
  useEffect(() => {
    current = () => {
      shell.ShowToast(`drawnui-react ${pkg.version} · CanvasKit ${CANVASKIT}`, 3000);
      return true;
    };
    return () => { current = null; };
  }, [shell]);
  return null;
}
