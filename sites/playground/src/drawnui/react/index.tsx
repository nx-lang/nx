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
import type { SkiaShaderCarousel as SkiaShaderCarouselCtrl } from "../controls/SkiaShaderCarousel";
import type { SkiaDrawer as SkiaDrawerCtrl } from "../controls/SkiaDrawer";
import type { SkiaLayout as SkiaLayoutCtrl } from "../controls/SkiaLayout";
import type { SkiaHotspot as SkiaHotspotCtrl } from "../controls/SkiaHotspot";
import type { SkiaButton as SkiaButtonCtrl } from "../controls/SkiaButton";
import type { SkiaImage as SkiaImageCtrl } from "../controls/SkiaImage";
import type { SkiaImageTiles as SkiaImageTilesCtrl } from "../controls/SkiaImageTiles";
import type { SkiaDecoratedGrid as SkiaDecoratedGridCtrl } from "../controls/SkiaDecoratedGrid";
import type { SkiaScrollBar as SkiaScrollBarCtrl } from "../controls/SkiaScrollBar";
import type { RefreshIndicator as RefreshIndicatorCtrl } from "../controls/RefreshIndicator";
import type { SkiaSvg as SkiaSvgCtrl } from "../controls/SkiaSvg";
import type { SkiaBackdrop as SkiaBackdropCtrl } from "../controls/SkiaBackdrop";
import type { SkiaEditor as SkiaEditorCtrl } from "../controls/SkiaEditor";
import type { SkiaSprite as SkiaSpriteCtrl } from "../controls/SkiaSprite";
import type { SkiaSpriteSet as SkiaSpriteSetCtrl } from "../controls/SkiaSpriteSet";
import type { SkiaLottie as SkiaLottieCtrl } from "../controls/SkiaLottie";
import type { SkiaGif as SkiaGifCtrl } from "../controls/SkiaGif";
import { SkiaScroll as SkiaScrollCtrl } from "../controls/SkiaScroll";
import type { SkiaShape as SkiaShapeCtrl } from "../controls/SkiaShape";
import type { Color, RenderingModeType } from "../core/Types";
import type { GesturesMode } from "../core/Gestures";
import { createDrawnRoot } from "./reconciler";

/**
 * Public settable properties of a control become its JSX props, same PascalCase names as C#.
 *
 * Function-typed members split in two: a METHOD (`Update(): void`) is not a prop and is dropped,
 * while an EVENT is declared optional (`Tapped?: (sender, e) => void`) and is exactly the thing a
 * caller passes in JSX. Optionality is the discriminator; without it every handler was missing
 * from the props type, so `<SkiaButton Tapped={…} />` worked at runtime but failed to typecheck
 * and never appeared in completion.
 */
type PropsOf<T> = Partial<{
  // eslint-disable-next-line @typescript-eslint/no-unsafe-function-type
  [K in keyof T as T[K] extends Function ? (undefined extends T[K] ? K : never) : K extends "Children" | "Views" | "Parent" | "Spans" | "GridStructure" | "AccessibilityId" | "IsAccessibilityElement" | "HasTransform" | "RenderTransformMatrix" | "RenderObjectPrevious" | "LastMeasuredIndex" | "ItemsInsertedAtStart" | "UsingControlStyle" | "Track" | "Thumb" | "FrameOn" | "FrameOff" | "ViewCheckOn" | "ViewOn" | "ViewText" | "Ratio" | "StartThumbX" | "EndThumbX" | "SnapPoints" | "CurrentPosition" | "CurrentSnap" | "ContentOffsetBounds" | "InTransition" | "CanAnimate" | "MaxIndex" | "ChildrenTotal" | "IsAtStart" | "IsAtEnd" | "ScrollProgress" | "ScrollAmount" | "TransitionProgress" | "LastIndex" | "ChildrenCount" | "Horizontal" | "Animator" | "Animation" | "IsPlaying" | "PlayWhenAvailable" | "TotalFrames" | "HasEffects" | "IsDisposed" | "Label" | "IsMultiline" | "HasSelection" | "LayoutVersion" | "LinesCount" | "MeasuredLineHeight" | "Focused" | "SpriteSheet" | "FrameWidth" | "FrameHeight" | "DurationMs" | "FrameDurationMs" | "CurrentSprite" | "Rects" | "HasTapHandler" | "HasDecorations" | "LinesCount" | "Superview" | "DrawingRect" | "MeasuredSize" | "RenderingScale" | "NeedMeasure" | "_superview" | "HitBoxAuto" | "TotalDown" | "TotalTapped" | "TouchDown" | "PostAnimators" | "LoadedSource" | "IsLoading" | "DisplayRect" | "AspectScale" | "Content" | "ContentSize" | "ContentOffsetBounds" | "OverscrollDistance" | "OverScrolled" | "IsUserPanning" | "IsUserFocused" | "IsScrolling" | "IsTemplated" | "FirstVisibleIndex" | "LastVisibleIndex" | "DebugString" | "ChildrenFactory" | "ContextIndex" | "RenderObject" | "UsingCacheType" | "TransitionEffect" | "TransitionFromIndex" | "TransitionToIndex" | "EffectPostRenderers" | "CachedImage" | "ImageBitmap" | "ShouldSubmitOnEnter" | "Header" | "Footer" | "RefreshIndicator" | "ScrollBar" | "ScrollBarHorizontal" | "IsRunning" | "VisibleRatio" | "CurrentIndex" | "LastCompositeRecord" | "DirtyChildrenInternal" | "IsRenderingWithComposition" ? never : K]: T[K];
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
/** Grid drawing gradient separator lines in its spacing (C# SkiaDecoratedGrid). */
export const SkiaDecoratedGrid = "SkiaDecoratedGrid" as unknown as FC<LayoutProps<SkiaDecoratedGridCtrl>>;
export const SkiaLabel = "SkiaLabel" as unknown as FC<LayoutProps<SkiaLabelCtrl>>;
/** Markdown label (C# SkiaRichLabel): Text is markdown, rendered as spans; LinkTapped for [text](url). */
export const SkiaRichLabel = "SkiaRichLabel" as unknown as FC<LeafProps<SkiaRichLabelCtrl>>;
/** Child of <SkiaLabel>: a styled fragment (C# TextSpan). */
export const TextSpan = "TextSpan" as unknown as FC<LeafProps<TextSpanCtrl>>;
export const SkiaHotspot = "SkiaHotspot" as unknown as FC<LeafProps<SkiaHotspotCtrl>>;
export const SkiaButton = "SkiaButton" as unknown as FC<LeafProps<SkiaButtonCtrl>>;
export const SkiaImage = "SkiaImage" as unknown as FC<LeafProps<SkiaImageCtrl>>;
/** Source repeated as TileWidth x TileHeight tiles over the box (C# SkiaImageTiles). */
export const SkiaImageTiles = "SkiaImageTiles" as unknown as FC<LeafProps<SkiaImageTilesCtrl>>;
export const SkiaSvg = "SkiaSvg" as unknown as FC<LeafProps<SkiaSvgCtrl>>;
/** Blurs / tints what is painted beneath it (C# SkiaBackdrop); children draw first. */
export const SkiaBackdrop = "SkiaBackdrop" as unknown as FC<LayoutProps<SkiaBackdropCtrl>>;
/** Drawn text input (C# SkiaEditor): Text, PlaceholderText, MaxLines, IsPassword, ControlStyle, TextChanged / TextSubmitted. */
export const SkiaEditor = "SkiaEditor" as unknown as FC<LeafProps<SkiaEditorCtrl>>;
/** Spritesheet player (C# SkiaSprite): Source, Columns, Rows, FramesPerSecond, FrameSequence. */
export const SkiaSprite = "SkiaSprite" as unknown as FC<LeafProps<SkiaSpriteCtrl>>;
/** Stateful sprite switcher (C# SkiaSpriteSet): Define() per State through a ref. */
export const SkiaSpriteSet = "SkiaSpriteSet" as unknown as FC<LayoutProps<SkiaSpriteSetCtrl>>;
export const SkiaLottie = "SkiaLottie" as unknown as FC<LeafProps<SkiaLottieCtrl>>;
export const SkiaGif = "SkiaGif" as unknown as FC<LeafProps<SkiaGifCtrl>>;
export const SkiaScroll = "SkiaScroll" as unknown as FC<LayoutProps<SkiaScrollCtrl>>;
/** Default scroll bar overlay (C# SkiaScrollBar); as a SkiaScroll child with Tag="ScrollBar" / "ScrollBarHorizontal". */
export const SkiaScrollBar = "SkiaScrollBar" as unknown as FC<LayoutProps<SkiaScrollBarCtrl>>;
/** Pull-to-refresh view base (C# RefreshIndicator); as a SkiaScroll child with Tag="RefreshIndicator", put the visuals inside. */
export const RefreshIndicator = "RefreshIndicator" as unknown as FC<LayoutProps<RefreshIndicatorCtrl>>;
export const SkiaShape = "SkiaShape" as unknown as FC<LayoutProps<SkiaShapeCtrl>>;
export const SkiaFrame = "SkiaFrame" as unknown as FC<LayoutProps<SkiaShapeCtrl>>;
export const SkiaSwitch = "SkiaSwitch" as unknown as FC<LeafProps<SkiaSwitchCtrl>>;
export const SkiaCheckbox = "SkiaCheckbox" as unknown as FC<LeafProps<SkiaCheckboxCtrl>>;
export const SkiaRadioButton = "SkiaRadioButton" as unknown as FC<LeafProps<SkiaRadioButtonCtrl>>;
export const SkiaProgress = "SkiaProgress" as unknown as FC<LeafProps<SkiaProgressCtrl>>;
export const SkiaSlider = "SkiaSlider" as unknown as FC<LeafProps<SkiaSliderCtrl>>;
export const SkiaCarousel = "SkiaCarousel" as unknown as FC<LayoutProps<SkiaCarouselCtrl>>;
/** Carousel whose slide changes are rendered by an SkSL transition (C# SkiaShaderCarousel); slides need UseCache="Image". */
export const SkiaShaderCarousel = "SkiaShaderCarousel" as unknown as FC<LayoutProps<SkiaShaderCarouselCtrl>>;
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
  /** Context-menu request (right click / long press / Menu key) no control handled; return true to suppress the browser menu. */
  ContextMenu?: CanvasView["ContextMenu"];
}

/**
 * One-shot, at the first Canvas mount: removes the HTML the `drawnUiStatic` Vite plugin generated for crawlers
 * (`[data-drawnui-static]`, placed after the mount element). A querySelectorAll once per page, never again; an
 * app built without the plugin finds nothing.
 */
let staticRemoved = false;
function removeStaticContent(): void {
  if (staticRemoved || typeof document === "undefined") return;
  staticRemoved = true;
  for (const el of document.querySelectorAll("[data-drawnui-static]")) el.remove();
}

/**
 * Mirrors DrawnUi Canvas: the bridge between the DOM (react-dom) and the drawn tree (DrawnUi reconciler).
 * Requires Super.UseDrawnUi()...BuildAsync() to have completed.
 */
export function Canvas({ BackgroundColor, RenderingMode, Gestures, children, style, className, ref: viewRef, ContextMenu }: CanvasProps) {
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
    removeStaticContent(); // the constructor drew frame 1: the crawlable block (drawnUiStatic) is duplicate content from here on
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
    v.ContextMenu = ContextMenu;
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
.drawnui-a11y-overlay{position:absolute;inset:0;overflow:hidden;pointer-events:none;user-select:none;-webkit-user-select:none}
.drawnui-a11y-node{position:absolute;margin:0;padding:0;border:0;background:transparent;color:transparent;overflow:hidden;white-space:nowrap;pointer-events:none;font:inherit;user-select:none;-webkit-user-select:none}
.drawnui-a11y-node::selection,.drawnui-a11y-node *::selection{background:transparent;color:transparent}
.drawnui-a11y-node:focus{outline:none}
.drawnui-a11y-node:focus-visible{outline:3px solid rgba(13,110,253,.85);outline-offset:1px;border-radius:3px}
.drawnui-a11y-text{overflow:visible}
.drawnui-a11y-text>span{position:absolute;white-space:pre;color:transparent;user-select:text;-webkit-user-select:text;pointer-events:auto;cursor:text;line-height:1;font-kerning:none;font-feature-settings:"kern" 0,"liga" 0,"calt" 0;text-rendering:geometricPrecision}
.drawnui-a11y-text>span::selection{background:rgba(110,168,254,.55);color:transparent}
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
  // selectable text spans take pointer events: the wheel over them is re-dispatched to the canvas so drawn scrolls keep working
  const overlayRef = useRef<HTMLDivElement>(null);
  const hasNodes = nodes.length > 0;
  useEffect(() => {
    const el = overlayRef.current;
    if (!el) return;
    const forward = (e: WheelEvent) => { e.preventDefault(); view.Element.dispatchEvent(new WheelEvent("wheel", { deltaX: e.deltaX, deltaY: e.deltaY, deltaMode: e.deltaMode, clientX: e.clientX, clientY: e.clientY, ctrlKey: e.ctrlKey, shiftKey: e.shiftKey, bubbles: false, cancelable: true })); };
    el.addEventListener("wheel", forward, { passive: false });
    return () => el.removeEventListener("wheel", forward);
  }, [view, hasNodes]);
  // the browser lays the invisible lines out with its own shaping; stretch each line with letter-spacing to the drawn width
  // so the selection highlight covers the drawn glyphs end to end
  useLayoutEffect(() => {
    const el = overlayRef.current;
    if (!el) return;
    for (const span of el.querySelectorAll<HTMLSpanElement>(".drawnui-a11y-text>span")) {
      const want = parseFloat(span.dataset.w ?? "0"), n = (span.textContent ?? "").replace(/\n$/, "").length - 1;
      span.style.letterSpacing = "0px";
      if (want <= 0 || n <= 0) continue;
      const have = span.getBoundingClientRect().width;
      if (Math.abs(have - want) > 0.5) span.style.letterSpacing = `${(want - have) / n}px`;
    }
  }, [nodes]);
  if (!hasNodes) return null;
  // the browser scrolls an overflow:hidden container to reveal a focused child; the overlay must stay pinned to the canvas
  const pin = (e: React.SyntheticEvent<HTMLDivElement>) => { e.currentTarget.scrollTop = 0; e.currentTarget.scrollLeft = 0; };
  return (
    <div className="drawnui-a11y-overlay" onScroll={pin} ref={overlayRef}>
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
        ) : n.TextLines ? (
          // AccessibilityTextSelectable: one positioned span per drawn line, in the drawn font, so the browser selects and copies it like HTML
          <div key={n.Id} role={n.Role} title={n.Hint} aria-live={n.Live as "polite" | "assertive" | undefined} className="drawnui-a11y-node drawnui-a11y-text" style={pos}>
            {n.TextLines.map((l, i) => (
              <span key={i} data-w={l.Width} style={{ left: l.Left, top: l.Top, height: l.Height, fontFamily: l.FontFamily, fontWeight: l.FontWeight, fontSize: l.FontSize, lineHeight: `${l.Height}px` }}>{l.Text}{i < n.TextLines!.length - 1 ? "\n" : ""}</span>
            ))}
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
