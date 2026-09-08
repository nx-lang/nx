import { createContext, useContext } from "react";
import type { Canvas as CanvasView } from "drawnui-react/core";

/** The engine Canvas of the demo, for diagnostics (FrameTime / FPS). */
export const CanvasViewContext = createContext<CanvasView | null>(null);
export const useCanvasView = () => useContext(CanvasViewContext);
