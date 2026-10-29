import type { Diagnostic } from "../compile/types.ts";
import type { NxGraphicsScene } from "@nx-lang/sdk-wasm";
export type {
  NxGraphicsPoint as Point,
  NxGraphicsRect as Rect,
  NxGraphicsBounds as Bounds,
  NxGraphicsPathElement as PathElement,
  NxGraphicsColor as Color,
  NxGraphicsTextRequest as TextRequest,
  NxGraphicsTextRun as TextRun,
  NxGraphicsMeasurement as Measurement,
  NxGraphicsNode as GraphicsNode,
  NxGraphicsScene as GraphicsScene,
} from "@nx-lang/sdk-wasm";
export type { SourceSite } from "../preview/selection.ts";
export interface DrawingResult {
  readonly source: string;
  readonly scene: NxGraphicsScene | null;
  readonly diagnostics: readonly Diagnostic[];
}
export const BROWSER_RENDERER = "browser-canvas2d-v1";
