import type { GrDirectContext, Surface, WebGLContextHandle } from "canvaskit-wasm";
import type { CachedObject, SkiaControl } from "./SkiaControl";
import type { AnimatorBase } from "./Animators";
import { Super } from "./Super";
import { SkiaAccessibilityManager } from "./Accessibility";
import { type Color, Colors, type RenderingModeType, SKRect } from "./Types";
import {
  GestureEventProcessingInfo, type GesturesMode, SKPoint, SkiaGesturesParameters, TouchActionEventArgs,
  type TouchActionResult, type TouchActionType,
} from "./Gestures";

/**
 * Mirrors DrawnUi Canvas (DrawnView): hosts one Content control on an HTML canvas element,
 * owns RenderingScale (devicePixelRatio), the surface, the on-demand frame loop and raw input.
 * Frames are drawn only after Update() (invalidation), never continuously.
 * Gestures are accumulated and processed in order at the START of the next frame, like DrawnUi.
 */
export class Canvas {
  /** Points a pointer may travel between Down and Up and still count as a tap (AppoMobi TouchEffect default). */
  static TappedCancelMoveThresholdPoints = 16;

  BackgroundColor: Color = Colors.Transparent;
  /** Accelerated = WebGL surface, Default = software. Read once at first frame. */
  RenderingMode: RenderingModeType = "Accelerated";
  RenderingScale = 1;
  /** Duration of the last Draw in ms (measure + arrange + render, excluding GPU flush). */
  FrameTime = 0;
  /** Frames per second over the last second of drawn frames. */
  FPS = 0;
  private frameTimes: number[] = [];
  /** Registry + rate-limited snapshot of accessible controls, rendered by the DOM overlay (DrawnUi AccessibilityManager). */
  readonly AccessibilityManager = new SkiaAccessibilityManager();

  private content?: SkiaControl;
  get Content(): SkiaControl | undefined { return this.content; }
  set Content(value: SkiaControl | undefined) {
    if (this.content) this.content._superview = undefined;
    this.content = value;
    if (value) { value.Parent = undefined; value._superview = this; }
    this.Update();
  }

  private surface?: Surface;
  /** GL context + GrContext live for the Canvas lifetime; only the on-screen surface is recreated on resize. */
  private glHandle?: WebGLContextHandle;
  private grContext?: GrDirectContext;
  private frameId = 0;
  private disposed = false;
  private readonly observer: ResizeObserver;

  constructor(readonly Element: HTMLCanvasElement) {
    if (!Super.CK) throw new Error("DrawnUi: call Super.UseDrawnUi()...BuildAsync() before creating a Canvas");
    this.observer = new ResizeObserver(() => this.OnResized());
    this.observer.observe(Element);
    this.OnResized();
  }

  /** Request a redraw (full measure + arrange + render) on the next animation frame. */
  Update(): void {
    if (this.frameId || !this.surface || this.disposed) return;
    const surface = this.surface;
    this.frameId = surface.requestAnimationFrame((c) => {
      this.frameId = 0;
      if (this.surface === surface && !this.disposed) { this.Draw(c); surface.flush(); this.DrainDisposeQueue(); }
    });
  }

  /** Deletes the current surface; a pending frame on it is cancelled first (drawing on a deleted surface faults). */
  private ReleaseSurface(): void {
    if (this.frameId) { cancelAnimationFrame(this.frameId); this.frameId = 0; }
    this.surface?.delete();
    this.surface = undefined;
  }

  /** On-screen surface at the element's current pixel size, reusing the GL/GrContext when accelerated. */
  private CreateSurface(w: number, h: number): Surface | undefined {
    const CK = Super.CK;
    if (this.RenderingMode === "Accelerated") {
      if (!this.grContext) {
        this.glHandle = CK.GetWebGLContext(this.Element);
        this.grContext = (this.glHandle ? CK.MakeWebGLContext(this.glHandle) : null) ?? undefined;
      }
      if (this.grContext) {
        const gl = CK.MakeOnScreenGLSurface(this.grContext, w, h, CK.ColorSpace.SRGB);
        if (gl) return gl;
      }
    }
    return CK.MakeSWCanvasSurface(this.Element) ?? undefined;
  }

  /**
   * Resize = the browser wipes the bitmap when width/height change, so the new frame is drawn
   * SYNCHRONOUSLY here. ResizeObserver callbacks run after layout and before paint, so the
   * redrawn content lands in the same paint and no blank frame is ever shown while dragging.
   */
  private OnResized(): void {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.round(this.Element.clientWidth * dpr));
    const h = Math.max(1, Math.round(this.Element.clientHeight * dpr));
    if (this.surface && this.Element.width === w && this.Element.height === h && this.RenderingScale === dpr) return;
    this.RenderingScale = dpr;
    this.Element.width = w;
    this.Element.height = h;
    this.ReleaseSurface();
    this.surface = this.CreateSurface(w, h);
    if (!this.surface) throw new Error("DrawnUi: cannot create surface");
    this.DrawNow();
  }

  /** Draws one frame immediately (outside the rAF loop) and presents it. */
  private DrawNow(): void {
    const surface = this.surface;
    if (!surface || this.disposed) return;
    this.Draw(surface.getCanvas());
    surface.flush();
    this.DrainDisposeQueue();
  }

  private Draw(canvas: import("canvaskit-wasm").Canvas): void {
    const started = performance.now();
    this.ProcessPendingGestures();
    const executed = this.ExecuteAnimators(Math.round(started * 1_000_000));
    canvas.clear(Super.ParseColor(this.BackgroundColor));
    const root = this.content;
    if (root) {
      const scale = this.RenderingScale;
      const w = this.Element.width, h = this.Element.height;
      root.Measure(w, h, scale);
      root.Arrange(new SKRect(0, 0, w, h), root.WidthRequest, root.HeightRequest, scale);
      root.Render({ Context: { Canvas: canvas, Surface: this.surface }, Destination: new SKRect(0, 0, w, h), Scale: scale });
    }
    this.AccessibilityManager.OnFrameEnd(this.RenderingScale, this.Element.width, this.Element.height, () => this.Update());
    const now = performance.now();
    this.FrameTime = now - started;
    this.frameTimes.push(now);
    while (this.frameTimes.length && this.frameTimes[0] < now - 1000) this.frameTimes.shift();
    this.FPS = this.frameTimes.length;
    if (executed > 0) this.Update(); // animators running: keep frames coming
  }

  // ---- deferred disposal (DrawnUi DisposeObject: never delete Skia objects mid-frame) ----

  private readonly disposeQueue: CachedObject[] = [];

  /** Queues a cache for deletion after the current frame has been flushed. */
  DisposeObject(obj: CachedObject): void { this.disposeQueue.push(obj); }

  private DrainDisposeQueue(): void {
    if (this.disposeQueue.length === 0) return;
    for (const o of this.disposeQueue) o.Dispose();
    this.disposeQueue.length = 0;
  }

  // ---- animators (DrawnView.AnimatingControls) ----

  readonly AnimatingControls = new Map<number, AnimatorBase>();

  RegisterAnimator(animator: AnimatorBase): boolean {
    if (this.disposed) return false;
    this.AnimatingControls.set(animator.Uid, animator);
    return true;
  }

  UnregisterAnimator(uid: number): void { this.AnimatingControls.delete(uid); }

  /** Ticks every registered animator once; returns how many ran. */
  protected ExecuteAnimators(frameTimeNanos: number): number {
    let executed = 0;
    for (const a of [...this.AnimatingControls.values()]) {
      if (!a.Parent) { this.AnimatingControls.delete(a.Uid); continue; }
      a.TickFrame(frameTimeNanos);
      executed++;
    }
    return executed;
  }

  Dispose(): void {
    this.disposed = true;
    this.Gestures = "Disabled";
    this.observer.disconnect();
    this.Content = undefined;
    this.ReleaseSurface();
    this.grContext?.delete();
    this.grContext = undefined;
    if (this.glHandle) { Super.CK.deleteContext(this.glHandle); this.glHandle = undefined; }
  }

  // ---- gestures: raw pointer -> TouchActionEventArgs -> recognized SkiaGesturesParameters -> queue ----

  private gestures: GesturesMode = "Disabled";
  get Gestures(): GesturesMode { return this.gestures; }
  set Gestures(value: GesturesMode) {
    if (this.gestures === value) return;
    if (this.gestures !== "Disabled") this.DetachInput();
    this.gestures = value;
    if (value !== "Disabled") this.AttachInput();
  }

  private readonly activeTouchIds = new Set<number>();
  private readonly pointerDownArgs = new Map<number, TouchActionEventArgs>();
  private readonly previousTouchArgs = new Map<number, TouchActionEventArgs>();
  private readonly pendingGestures: SkiaGesturesParameters[] = [];

  private readonly onPointer = (e: PointerEvent) => {
    const type: TouchActionType | undefined =
      e.type === "pointerdown" ? "Pressed" :
      e.type === "pointermove" ? "Moved" :
      e.type === "pointerup" ? "Released" :
      e.type === "pointercancel" ? "Cancelled" : undefined;
    if (!type) return;
    if (type === "Moved" && !this.activeTouchIds.has(e.pointerId)) return; // hover not ported (TouchActionResult.Pointer)
    // Capture so Up outside the element still arrives; throws for synthetic events (tests) — harmless.
    if (type === "Pressed") { try { this.Element.setPointerCapture(e.pointerId); } catch { /* synthetic pointer */ } }

    const rect = this.Element.getBoundingClientRect();
    const args = new TouchActionEventArgs();
    args.Id = e.pointerId;
    args.Type = type;
    args.Scale = this.RenderingScale;
    args.Location = new SKPoint((e.clientX - rect.left) * this.RenderingScale, (e.clientY - rect.top) * this.RenderingScale);
    this.OnTouchAction(args);
  };
  private readonly preventTouch = (e: TouchEvent) => e.preventDefault();

  /** Mouse wheel -> TouchActionResult.Wheel (page scroll suppressed while gestures are enabled). */
  private readonly onWheel = (e: WheelEvent) => {
    e.preventDefault();
    const rect = this.Element.getBoundingClientRect();
    const args = new TouchActionEventArgs();
    args.Id = -1;
    args.Type = "Wheel";
    args.Scale = this.RenderingScale;
    args.Location = new SKPoint((e.clientX - rect.left) * this.RenderingScale, (e.clientY - rect.top) * this.RenderingScale);
    args.StartingLocation = args.Location;
    args.Wheel = { Delta: e.deltaY !== 0 ? e.deltaY : e.deltaX };
    this.OnGestureEvent(args, "Wheel");
  };

  private AttachInput(): void {
    const el = this.Element;
    el.style.touchAction = "none";
    el.style.userSelect = "none";
    for (const t of ["pointerdown", "pointermove", "pointerup", "pointercancel"]) el.addEventListener(t, this.onPointer as EventListener);
    el.addEventListener("wheel", this.onWheel, { passive: false });
    if (this.gestures === "Lock") el.addEventListener("touchmove", this.preventTouch, { passive: false });
  }

  private DetachInput(): void {
    const el = this.Element;
    el.style.touchAction = "";
    el.style.userSelect = "";
    for (const t of ["pointerdown", "pointermove", "pointerup", "pointercancel"]) el.removeEventListener(t, this.onPointer as EventListener);
    el.removeEventListener("wheel", this.onWheel);
    el.removeEventListener("touchmove", this.preventTouch);
    this.activeTouchIds.clear(); this.pointerDownArgs.clear(); this.previousTouchArgs.clear();
  }

  /** Port of DrawnUi.Blazor Canvas.OnTouchAction: per-pointer state machine producing Down / Panning / Tapped / Up. */
  OnTouchAction(args: TouchActionEventArgs): void {
    if (this.gestures === "Disabled") return;
    const id = args.Id;

    if (args.Type === "Pressed") {
      this.activeTouchIds.add(id);
      args.NumberOfTouches = this.activeTouchIds.size;
      args.StartingLocation = args.Location;
      args.IsInContact = true;
      this.pointerDownArgs.set(id, args);
      this.previousTouchArgs.set(id, args);
      this.OnGestureEvent(args, "Down");
      return;
    }

    args.NumberOfTouches = this.activeTouchIds.size;
    TouchActionEventArgs.FillDistanceInfo(args, this.previousTouchArgs.get(id));
    const downArgs = this.pointerDownArgs.get(id);
    args.StartingLocation = downArgs ? downArgs.StartingLocation : args.Location;

    if (args.Type === "Moved") {
      if (args.Distance.Delta.X !== 0 || args.Distance.Delta.Y !== 0) this.OnGestureEvent(args, "Panning");
      this.previousTouchArgs.set(id, args);
      return;
    }

    if (args.Type === "Released" || args.Type === "Cancelled") {
      args.IsInContact = args.NumberOfTouches > 1;
      if (!args.IsInContact && downArgs && args.Type === "Released") {
        const threshold = Canvas.TappedCancelMoveThresholdPoints * Math.max(0.1, this.RenderingScale);
        if (Math.abs(args.Distance.Total.X) < threshold && Math.abs(args.Distance.Total.Y) < threshold) this.OnGestureEvent(args, "Tapped");
      }
      this.OnGestureEvent(args, "Up");
      this.previousTouchArgs.delete(id);
      this.pointerDownArgs.delete(id);
      this.activeTouchIds.delete(id);
    }
  }

  private OnGestureEvent(args: TouchActionEventArgs, result: TouchActionResult): void {
    this.pendingGestures.push(SkiaGesturesParameters.Create(result, args));
    this.Update();
  }

  private ProcessPendingGestures(): void {
    if (this.pendingGestures.length === 0) return;
    const batch = this.pendingGestures.splice(0);
    const root = this.content;
    if (!root) return;
    for (const args of batch) this.ProcessGestures(root, args);
  }

  /** Entry into the control tree, same shape as DrawnUi Canvas.ProcessGestures. */
  protected ProcessGestures(root: SkiaControl, args: SkiaGesturesParameters): SkiaControl | null {
    return root.ProcessGestures(args, new GestureEventProcessingInfo(args.Event.Location, SKPoint.Empty, SKPoint.Empty, null));
  }
}
