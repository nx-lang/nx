import { type CSSProperties, type FC, type ReactNode, type Ref, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState } from "react";
import type { AccessibilityNode } from "../core/Accessibility";
import { Canvas as CanvasView } from "../core/Canvas";
import type { SkiaControl } from "../core/SkiaControl";
import type { SkiaLabel as SkiaLabelCtrl } from "../controls/SkiaLabel";
import type { TextSpan as TextSpanCtrl } from "../controls/TextSpan";
import type { SkiaRichLabel as SkiaRichLabelCtrl } from "../controls/SkiaRichLabel";
import type { SkiaSwitch as SkiaSwitchCtrl } from "../controls/SkiaSwitch";
import type { SkiaCheckbox as SkiaCheckboxCtrl } from "../controls/SkiaCheckbox";
import type { SkiaRadioButton as SkiaRadioButtonCtrl } from "../controls/SkiaRadioButton";
import type { SkiaProgress as SkiaProgressCtrl } from "../controls/SkiaProgress";
import type { SkiaSlider as SkiaSliderCtrl } from "../controls/SkiaSlider";
import type { SkiaCarousel as SkiaCarouselCtrl } from "../controls/SkiaCarousel";
import type { SkiaDrawer as SkiaDrawerCtrl } from "../controls/SkiaDrawer";
import type { SkiaLayout as SkiaLayoutCtrl } from "../controls/SkiaLayout";
import type { SkiaHotspot as SkiaHotspotCtrl } from "../controls/SkiaHotspot";
import type { SkiaButton as SkiaButtonCtrl } from "../controls/SkiaButton";
import type { SkiaImage as SkiaImageCtrl } from "../controls/SkiaImage";
import type { SkiaSvg as SkiaSvgCtrl } from "../controls/SkiaSvg";
import { SkiaScroll as SkiaScrollCtrl } from "../controls/SkiaScroll";
import type { SkiaShape as SkiaShapeCtrl } from "../controls/SkiaShape";
import type { Color, RenderingModeType } from "../core/Types";
import type { GesturesMode } from "../core/Gestures";
import { createDrawnRoot } from "./reconciler";

/** Public settable properties of a control become its JSX props, same PascalCase names as C#. */
type PropsOf<T> = Partial<{
  // eslint-disable-next-line @typescript-eslint/no-unsafe-function-type
  [K in keyof T as T[K] extends Function ? never : K extends "Children" | "Views" | "Parent" | "Spans" | "GridStructure" | "AccessibilityId" | "IsAccessibilityElement" | "HasTransform" | "RenderTransformMatrix" | "RenderObjectPrevious" | "LastMeasuredIndex" | "ItemsInsertedAtStart" | "UsingControlStyle" | "Track" | "Thumb" | "FrameOn" | "FrameOff" | "ViewCheckOn" | "ViewOn" | "ViewText" | "Ratio" | "StartThumbX" | "EndThumbX" | "SnapPoints" | "CurrentPosition" | "CurrentSnap" | "ContentOffsetBounds" | "InTransition" | "CanAnimate" | "MaxIndex" | "ChildrenTotal" | "IsAtStart" | "IsAtEnd" | "ScrollProgress" | "Horizontal" | "Rects" | "HasTapHandler" | "HasDecorations" | "LinesCount" | "Superview" | "DrawingRect" | "MeasuredSize" | "RenderingScale" | "NeedMeasure" | "_superview" | "HitBoxAuto" | "TotalDown" | "TotalTapped" | "TouchDown" | "PostAnimators" | "LoadedSource" | "IsLoading" | "DisplayRect" | "AspectScale" | "Content" | "ContentSize" | "ContentOffsetBounds" | "OverscrollDistance" | "OverScrolled" | "IsUserPanning" | "IsUserFocused" | "IsScrolling" | "IsTemplated" | "FirstVisibleIndex" | "LastVisibleIndex" | "DebugString" | "ChildrenFactory" | "ContextIndex" | "RenderObject" | "UsingCacheType" ? never : K]: T[K];
}>;

/** `ref` receives the engine control instance (react-reconciler getPublicInstance). */
type LeafProps<T> = PropsOf<T> & { ref?: Ref<T> };
type LayoutProps<T> = PropsOf<T> & { children?: ReactNode; ref?: Ref<T> };

/** Typed JSX tags resolved by the reconciler Registry. */
export const SkiaLayout = "SkiaLayout" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaStack = "SkiaStack" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaRow = "SkiaRow" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaLayer = "SkiaLayer" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaWrap = "SkiaWrap" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaGrid = "SkiaGrid" as unknown as FC<LayoutProps<SkiaLayoutCtrl>>;
export const SkiaLabel = "SkiaLabel" as unknown as FC<LayoutProps<SkiaLabelCtrl>>;
/** Markdown label (C# SkiaRichLabel): Text is markdown, rendered as spans; LinkTapped for [text](url). */
export const SkiaRichLabel = "SkiaRichLabel" as unknown as FC<LeafProps<SkiaRichLabelCtrl>>;
/** Child of <SkiaLabel>: a styled fragment (C# TextSpan). */
export const TextSpan = "TextSpan" as unknown as FC<LeafProps<TextSpanCtrl>>;
export const SkiaHotspot = "SkiaHotspot" as unknown as FC<LeafProps<SkiaHotspotCtrl>>;
export const SkiaButton = "SkiaButton" as unknown as FC<LeafProps<SkiaButtonCtrl>>;
export const SkiaImage = "SkiaImage" as unknown as FC<LeafProps<SkiaImageCtrl>>;
export const SkiaSvg = "SkiaSvg" as unknown as FC<LeafProps<SkiaSvgCtrl>>;
export const SkiaScroll = "SkiaScroll" as unknown as FC<LayoutProps<SkiaScrollCtrl>>;
export const SkiaShape = "SkiaShape" as unknown as FC<LayoutProps<SkiaShapeCtrl>>;
export const SkiaFrame = "SkiaFrame" as unknown as FC<LayoutProps<SkiaShapeCtrl>>;
export const SkiaSwitch = "SkiaSwitch" as unknown as FC<LeafProps<SkiaSwitchCtrl>>;
export const SkiaCheckbox = "SkiaCheckbox" as unknown as FC<LeafProps<SkiaCheckboxCtrl>>;
export const SkiaRadioButton = "SkiaRadioButton" as unknown as FC<LeafProps<SkiaRadioButtonCtrl>>;
export const SkiaProgress = "SkiaProgress" as unknown as FC<LeafProps<SkiaProgressCtrl>>;
export const SkiaSlider = "SkiaSlider" as unknown as FC<LeafProps<SkiaSliderCtrl>>;
export const SkiaCarousel = "SkiaCarousel" as unknown as FC<LayoutProps<SkiaCarouselCtrl>>;
export const SkiaDrawer = "SkiaDrawer" as unknown as FC<LayoutProps<SkiaDrawerCtrl>>;

export interface CanvasProps {
  BackgroundColor?: Color;
  RenderingMode?: RenderingModeType;
  /** Disabled (default) / Enabled / Lock, like DrawnUi Canvas.Gestures. */
  Gestures?: GesturesMode;
  children?: ReactNode;
  style?: CSSProperties;
  className?: string;
  /** Receives the engine Canvas (FPS, FrameTime, RenderingScale...). */
  ref?: Ref<CanvasView>;
}

/**
 * Mirrors DrawnUi Canvas: the bridge between the DOM (react-dom) and the drawn tree (DrawnUi reconciler).
 * Requires Super.UseDrawnUi()...BuildAsync() to have completed.
 */
export function Canvas({ BackgroundColor, RenderingMode, Gestures, children, style, className, ref: viewRef }: CanvasProps) {
  const ref = useRef<HTMLCanvasElement>(null);
  const view = useRef<CanvasView>(null);
  const root = useRef<ReturnType<typeof createDrawnRoot>>(null);
  const [engine, setEngine] = useState<CanvasView | null>(null);

  useLayoutEffect(() => {
    const v = new CanvasView(ref.current!);
    if (RenderingMode) v.RenderingMode = RenderingMode;
    view.current = v;
    root.current = createDrawnRoot(v);
    setEngine(v);
    return () => { root.current?.unmount(); v.Dispose(); view.current = null; root.current = null; setEngine(null); };
    // RenderingMode is read once at surface creation, like DrawnUi.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // After the effect above, so the handle resolves to the created view.
  useImperativeHandle(viewRef, () => view.current!, []);

  useLayoutEffect(() => {
    const v = view.current!;
    if (BackgroundColor !== undefined && v.BackgroundColor !== BackgroundColor) { v.BackgroundColor = BackgroundColor; v.Update(); }
    v.Gestures = Gestures ?? "Disabled";
    root.current!.render(children);
  });

  // The drawn surface is aria-hidden; the overlay is the accessibility tree (DrawnUi.Blazor pattern).
  return (
    <div className={className} style={{ position: "relative", ...style }}>
      <canvas ref={ref} aria-hidden style={{ display: "block", width: "100%", height: "100%" }} />
      {engine && <AccessibilityOverlay view={engine} />}
    </div>
  );
}

const A11Y_CSS = `
.drawnui-a11y-overlay{position:absolute;inset:0;overflow:hidden;pointer-events:none}
.drawnui-a11y-node{position:absolute;margin:0;padding:0;border:0;background:transparent;color:transparent;overflow:hidden;white-space:nowrap;pointer-events:none;font:inherit}
.drawnui-a11y-node:focus{outline:none}
.drawnui-a11y-node:focus-visible{outline:3px solid rgba(13,110,253,.85);outline-offset:1px;border-radius:3px}
`;

/**
 * Invisible ARIA elements mirroring the accessible drawn controls, positioned over the canvas.
 * Improvement over DrawnUi.Blazor: `pointer-events:none`, so hover and every pointer gesture still reach the canvas —
 * keyboard (Tab / Enter / Space) and screen-reader activation arrive as DOM events and are routed back as a Tapped.
 */
function AccessibilityOverlay({ view }: { view: CanvasView }) {
  const [nodes, setNodes] = useState<AccessibilityNode[]>(() => view.AccessibilityManager.Snapshot);
  useEffect(() => {
    const mgr = view.AccessibilityManager;
    setNodes(mgr.Snapshot);
    const off = mgr.OnChanged(() => setNodes(mgr.Snapshot));
    const offLive = mgr.OnLiveRegionUpdated(() => { mgr.ForceRebuildOnNextFrame(); view.Update(); });
    return () => { off(); offLive(); };
  }, [view]);
  if (nodes.length === 0) return null;
  // the browser scrolls an overflow:hidden container to reveal a focused child; the overlay must stay pinned to the canvas
  const pin = (e: React.SyntheticEvent<HTMLDivElement>) => { e.currentTarget.scrollTop = 0; e.currentTarget.scrollLeft = 0; };
  return (
    <div className="drawnui-a11y-overlay" onScroll={pin}>
      <style>{A11Y_CSS}</style>
      {nodes.map((n) => {
        const pos: CSSProperties = { left: n.Rect.Left, top: n.Rect.Top, width: n.Rect.Width, height: n.Rect.Height };
        const activate = () => n.Source.OnAccessibilityActivated();
        return n.CanInteract ? (
          <div key={n.Id} role={n.Role} aria-label={n.Label} title={n.Hint}
            aria-pressed={n.Role === "button" ? n.IsPressed : undefined}
            aria-checked={n.Role === "switch" || n.Role === "checkbox" || n.Role === "radio" ? n.IsPressed : undefined}
            aria-live={n.Live as "polite" | "assertive" | undefined}
            tabIndex={0} className="drawnui-a11y-node" style={{ ...pos, fontSize: 0, userSelect: "none" }}
            onClick={activate}
            onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); activate(); } }}
            onFocus={(e) => { pin({ currentTarget: e.currentTarget.parentElement as HTMLDivElement } as React.SyntheticEvent<HTMLDivElement>); SkiaScrollCtrl.EnsureVisible(n.Source); n.Source.OnAccessibilityFocused(true); n.Source.NotifyAccessibilityFocused(true); }}
            onBlur={() => { n.Source.OnAccessibilityFocused(false); n.Source.NotifyAccessibilityFocused(false); }}>
            {n.Label}
          </div>
        ) : (
          // static text is exposed as real (transparent) text content; aria-label only for roles that need a name
          <div key={n.Id} role={n.Role} aria-label={n.Role === "text" ? undefined : n.Label} title={n.Hint} aria-live={n.Live as "polite" | "assertive" | undefined} className="drawnui-a11y-node" style={pos}>
            {n.Label}
          </div>
        );
      })}
    </div>
  );
}

export type { SkiaControl };

// One import for apps: React tags + every engine type (Colors, Thickness, Super, gestures, animators...).
// The engine `Canvas` class is shadowed by the React <Canvas> above; reach it via "drawnui-react/core".
export * from "../index";

// React-level SkiaShell (routes, back nav bar, useShell())
export * from "./SkiaShell";
