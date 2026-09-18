/**
 * The engine classes React mounts, and how props reach them — the part of the reconciler that has
 * no React in it, so a cell built outside React (a templated list's) uses the very same steps.
 */
import type { SkiaControl } from "../core/SkiaControl";
import { SkiaLabel } from "../controls/SkiaLabel";
import { TextSpan } from "../controls/TextSpan";
import { SkiaRichLabel } from "../controls/SkiaRichLabel";
import { SkiaSwitch } from "../controls/SkiaSwitch";
import { SkiaCheckbox } from "../controls/SkiaCheckbox";
import { SkiaRadioButton } from "../controls/SkiaRadioButton";
import { SkiaProgress } from "../controls/SkiaProgress";
import { SkiaSlider } from "../controls/SkiaSlider";
import { SkiaCarousel } from "../controls/SkiaCarousel";
import { SkiaShaderCarousel } from "../controls/SkiaShaderCarousel";
import { SkiaDrawer } from "../controls/SkiaDrawer";
import { SkiaGrid, SkiaLayer, SkiaLayout, SkiaRow, SkiaStack, SkiaWrap } from "../controls/SkiaLayout";
import { SkiaHotspot } from "../controls/SkiaHotspot";
import { SkiaButton } from "../controls/SkiaButton";
import { SkiaImage } from "../controls/SkiaImage";
import { SkiaImageTiles } from "../controls/SkiaImageTiles";
import { SkiaDecoratedGrid } from "../controls/SkiaDecoratedGrid";
import { SkiaScrollBar } from "../controls/SkiaScrollBar";
import { RefreshIndicator } from "../controls/RefreshIndicator";
import { SkiaSvg } from "../controls/SkiaSvg";
import { SkiaBackdrop } from "../controls/SkiaBackdrop";
import { SkiaEditor } from "../controls/SkiaEditor";
import { SkiaSprite } from "../controls/SkiaSprite";
import { SkiaSpriteSet } from "../controls/SkiaSpriteSet";
import { SkiaLottie } from "../controls/SkiaLottie";
import { SkiaGif } from "../controls/SkiaGif";
import { SkiaScroll } from "../controls/SkiaScroll";
import { SkiaFrame, SkiaShape } from "../controls/SkiaShape";

/** Anything React can mount: controls, plus TextSpan (a SkiaLabel child that is not a control, as in C#). */
export type HostInstance = SkiaControl | TextSpan;

/** JSX tag name -> engine class. Add a control here to expose it to React. */
export const Registry: Record<string, new () => HostInstance> = {
  SkiaLayout, SkiaStack, SkiaRow, SkiaLayer, SkiaWrap, SkiaGrid, SkiaDecoratedGrid, SkiaBackdrop, SkiaEditor, SkiaSprite, SkiaSpriteSet, SkiaLabel, SkiaRichLabel, TextSpan, SkiaHotspot, SkiaButton, SkiaImage, SkiaImageTiles, SkiaSvg, SkiaLottie, SkiaGif, SkiaScroll, SkiaScrollBar, RefreshIndicator, SkiaShape, SkiaFrame,
  SkiaSwitch, SkiaCheckbox, SkiaRadioButton, SkiaProgress, SkiaSlider, SkiaCarousel, SkiaShaderCarousel, SkiaDrawer,
};

export type Props = Record<string, unknown>;
const SKIP = new Set(["children", "key", "ref"]);
/** Props that change how the subtree is composited, not what it contains: repaint, keep caches (DrawnUi RedrawCanvas). */
const REPAINT_ONLY = new Set(["TranslationX", "TranslationY", "Rotation", "ScaleX", "ScaleY", "Scale", "SkewX", "SkewY", "AnchorX", "AnchorY", "Opacity"]);

/**
 * Assigns changed props straight onto the control (same names as the C# properties).
 * Handler props (functions) are swapped without invalidating: inline arrows change identity every render.
 */
export function applyProps(inst: HostInstance, prev: Props | null, next: Props): void {
  let changed = false, repaint = false;
  for (const k in next) {
    if (SKIP.has(k) || (prev && prev[k] === next[k])) continue;
    (inst as unknown as Props)[k] = next[k];
    if (typeof next[k] === "function") continue;
    if (REPAINT_ONLY.has(k)) repaint = true; else changed = true;
  }
  if (prev) for (const k in prev) if (!SKIP.has(k) && !(k in next)) {
    (inst as unknown as Props)[k] = (new (inst.constructor as new () => HostInstance)() as unknown as Props)[k];
    changed = true;
  }
  if (changed && prev) inst.Update();
  else if (repaint && prev) (inst as SkiaControl).RepaintComposition?.();
}

