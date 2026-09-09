import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaImage, SkiaLabel, SkiaLayer, SkiaScroll, SkiaShaderCarousel, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import { type GestureEventProcessingInfo, ShaderDoubleTexturesEffect, type ISkiaGestureProcessor, SkiaControl, SkiaImage as SkiaImageCtrl, SkiaLabel as SkiaLabelCtrl, SkiaLayer as SkiaLayerCtrl, SkiaShaderEffect, type SkiaGesturesParameters, SkiaValueAnimator } from "drawnui-react/core";
import type { SkiaShaderCarousel as SkiaShaderCarouselCtrl } from "drawnui-react/core";

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#2B3035" HorizontalOptions="Fill">
      <SkiaStack Spacing={10} Padding={new Thickness(16, 12)}>
        <SkiaLabel Text={title} FontSize={12} TextColor="#6EA8FE" FontAttributes="Bold" TextTransform="Uppercase" />
        {children}
      </SkiaStack>
    </SkiaShape>
  );
}

interface Ripple { origin: { X: number; Y: number }; time: number; progress: number }

/** Port of the Sandbox MultiRippleWithTouchEffect: a WWDC-style ripple starts where the control is touched (up to 10 at once). */
class MultiRippleWithTouchEffect extends ShaderDoubleTexturesEffect implements ISkiaGestureProcessor {
  private ripples = new Set<Ripple>();

  constructor() {
    super();
    this.ShaderSource = "shaders/ripples.sksl";
  }

  protected override CreateUniforms(destination: import("drawnui-react/core").SKRect, textureBounds: import("drawnui-react/core").SKRect | undefined, values: Float32Array): void {
    super.CreateUniforms(destination, textureBounds, values);
    const active = [...this.ripples].sort((a, b) => b.time - a.time).slice(0, 10);
    const origins: number[] = [], progresses: number[] = [];
    for (let i = 0; i < 10; i++) {
      const r = active[i];
      origins.push(r ? r.origin.X : 0, r ? r.origin.Y : 0);
      progresses.push(r ? r.progress : -1); // -1 = inactive
    }
    this.Set(values, "origins", ...origins);
    this.Set(values, "progresses", ...progresses);
  }

  ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    const parent = this.Parent;
    if (args.Type !== "Down" || !parent) return null;
    const box = parent.DrawingRect;
    const ripple: Ripple = { origin: { X: apply.MappedLocation.X + apply.ChildOffset.X - box.Left, Y: apply.MappedLocation.Y + apply.ChildOffset.Y - box.Top }, time: performance.now(), progress: 0 };
    this.ripples.add(ripple);
    void parent.AnimateRangeAsync((v) => { ripple.progress = v; this.Update(); }, 0, 1, 4500).then(() => this.ripples.delete(ripple));
    return null;
  }
}

const PLASMA = `
uniform float2 iResolution;
uniform float2 iOffset;
uniform float  iTime;
uniform float4 iMouse;

half4 main(float2 fragCoord) {
    float2 uv = (fragCoord - iOffset) / iResolution.xy;
    float t = iTime * 0.6;
    float v = sin(uv.x * 6.0 + t) + sin((uv.y * 6.0 + t) * 0.8) + sin((uv.x + uv.y) * 4.0 - t) + sin(length(uv - 0.5) * 12.0 - t * 1.5);
    v *= 0.25;
    float3 col = 0.5 + 0.5 * cos(6.2831 * (v + float3(0.0, 0.33, 0.67)) + t);
    return half4(col, 1.0);
}`;

const WAVE = `
uniform shader iImage1;
uniform float2 iResolution;
uniform float2 iImageResolution;
uniform float2 iOffset;
uniform float  iTime;
uniform float4 iMouse;
uniform float  strength;

half4 main(float2 fragCoord) {
    float2 uv = (fragCoord - iOffset) / iResolution.xy;
    float2 d = float2(sin(uv.y * 20.0 + iTime * 3.0), cos(uv.x * 20.0 + iTime * 2.0)) * strength;
    float2 p = (uv + d) * iImageResolution;
    return iImage1.eval(p);
}`;

const TRANSITIONS = ["cube", "fade", "swirl", "doorway", "bounce", "waterdrop", "pixelize", "windowslice", "crosszoom", "pagecurl", "morph", "heart", "kaleidoscope", "wind"];
const PHOTOS = ["images/hugrobot2.jpg", "images/8.jpg", "images/dungeon.jpg", "images/nebula.jpg"];

/** A slide of the shader carousel: MUST be cached as Image, the transition effect samples the cache. */
class PhotoSlide extends SkiaLayerCtrl {
  private readonly image = new SkiaImageCtrl();
  private readonly label = new SkiaLabelCtrl();
  constructor() {
    super();
    this.UseCache = "Image";
    this.HorizontalOptions = "Fill"; this.VerticalOptions = "Fill";
    this.image.Aspect = "AspectCover"; this.image.HorizontalOptions = "Fill"; this.image.VerticalOptions = "Fill";
    this.label.FontSize = 28; this.label.TextColor = Colors.White; this.label.HorizontalOptions = "Center"; this.label.VerticalOptions = "Center";
    this.label.DropShadowColor = "#000000"; this.label.DropShadowSize = 4;
    this.AddSubView(this.image); this.AddSubView(this.label);
  }
  protected override OnBindingContextChanged(): void {
    const i = this.ContextIndex;
    this.image.Source = PHOTOS[i % PHOTOS.length];
    this.label.Text = `Slide ${i + 1}`;
  }
}

/** SkiaShaderEffect (SkSL on a control's output, generative shaders, touch ripples) and SkiaShaderCarousel transitions. */
export function ShadersPage() {
  const [error, setError] = useState("");
  const ripple = useMemo(() => { const e = new MultiRippleWithTouchEffect(); e.SecondarySource = "images/nebula.jpg"; e.OnCompilationError = (_, err) => setError(err); return e; }, []);
  const [strength, setStrength] = useState(0.01);
  const wave = useMemo(() => { const e = new SkiaShaderEffect(); e.ShaderCode = WAVE; e.OnCompilationError = (_, err) => setError(err); return e; }, []);
  useEffect(() => { wave.SetUniform("strength", strength); }, [wave, strength]);
  const plasma = useMemo(() => { const e = new SkiaShaderEffect(); e.ShaderCode = PLASMA; e.UseBackground = "Never"; e.AutoCreateInputTexture = false; e.OnCompilationError = (_, err) => setError(err); return e; }, []);
  const [blit, setBlit] = useState(false);
  const blitFx = useMemo(() => { const e = new SkiaShaderEffect(); e.ShaderSource = "shaders/blit.sksl"; e.OnCompilationError = (_, err) => setError(err); return e; }, []);

  // iTime shaders repaint only when something asks for frames: a looping animator on the host ticks them (C# needs the same)
  const waveHost = useRef<SkiaImageCtrl>(null);
  const plasmaHost = useRef<SkiaLayerCtrl>(null);
  const [running, setRunning] = useState(true);
  useEffect(() => {
    const hosts = [waveHost.current, plasmaHost.current].filter((h) => !!h) as unknown as SkiaControl[];
    if (!running || hosts.length === 0) return;
    const animators = hosts.map((h) => { const a = new SkiaValueAnimator(h); a.mMinValue = 0; a.mMaxValue = 1; a.Speed = 1000; a.Repeat = -1; a.OnUpdated = () => { wave.Update(); plasma.Update(); }; a.Start(); return a; });
    return () => animators.forEach((a) => a.Stop());
  }, [running, wave, plasma]);

  const carousel = useRef<SkiaShaderCarouselCtrl>(null);
  const [transition, setTransition] = useState("cube");
  const [fromTo, setFromTo] = useState("");
  const template = useCallback(() => new PhotoSlide(), []);
  const items = useMemo(() => PHOTOS.map((_, i) => i), []);

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Shaders" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        {error ? <SkiaLabel Text={`Shader error: ${error}`} FontSize={12} TextColor="#FF6B6B" HorizontalOptions="Fill" /> : null}

        <Card title={`SkiaShaderCarousel — TransitionShader="shaders/transitions/${transition}.sksl" · IsLooped · LinearSpeedMs=750 ${fromTo}`}>
          <SkiaShaderCarousel ref={carousel} HeightRequest={280} IsLooped LinearSpeedMs={750} TransitionShader={`shaders/transitions/${transition}.sksl`} ItemsSource={items} ItemTemplate={template}
            FromToChanged={(c) => setFromTo(`· ${c.TransitionFromIndex} → ${c.TransitionToIndex}`)} />
          <SkiaWrap Spacing={6}>
            <SkiaButton Text="‹ Prev" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.GoPrev()} />
            <SkiaButton Text="Next ›" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.GoNext()} />
            {TRANSITIONS.map((t) => <SkiaButton key={t} Text={t} BackgroundColor={transition === t ? "#533483" : "#495057"} FontSize={12} Tapped={() => setTransition(t)} />)}
          </SkiaWrap>
          <SkiaLabel FontFamilyFallback="FontSymbols,FontSymbols2" Text="Slides never move: a ShaderTransitionEffect blends the Image caches of the outgoing and incoming slides (iImage1 / iImage2, progress, ratio) through a gl-transitions style transition(uv) wrapped by the adapter template. Swipe, or drag slowly to scrub the transition; a swipe during a transition wraps it up first (InterruptedTransitionMs)." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <Card title='SkiaShaderEffect on a SkiaImage — ShaderSource="shaders/ripples.sksl" (Sandbox MultiRippleWithTouchEffect) · tap to ripple'>
          <SkiaImage Source="images/hugrobot2.jpg" Aspect="AspectCover" HorizontalOptions="Fill" HeightRequest={260} UseCache="Image" VisualEffects={[ripple]} />
          <SkiaLabel Text="The effect is an ISkiaGestureProcessor: every Down starts a ripple at the touch point, animated 0→1 over 4.5 s through Parent.AnimateRangeAsync and passed as the origins[10] / progresses[10] array uniforms; iImage1 is the image's own cache, iImage2 (SecondarySource) the reflection texture." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <Card title={`Inline ShaderCode + SetUniform("strength", ${strength}) + iTime · ${running ? "animating" : "paused"}`}>
          <SkiaImage ref={waveHost} Source="images/8.jpg" Aspect="AspectCover" HorizontalOptions="Fill" HeightRequest={200} UseCache="Image" VisualEffects={[wave]} />
          <SkiaWrap Spacing={6}>
            {[0, 0.005, 0.01, 0.03].map((v) => <SkiaButton key={v} Text={`strength ${v}`} BackgroundColor={strength === v ? "#533483" : "#495057"} FontSize={12} Tapped={() => setStrength(v)} />)}
            <SkiaButton Text={running ? "Pause" : "Run"} BackgroundColor="#0D6EFD" FontSize={12} Tapped={() => setRunning((r) => !r)} />
          </SkiaWrap>
        </Card>

        <Card title='Generative shader — UseBackground="Never" on a SkiaLayer (no input texture)'>
          <SkiaLayer ref={plasmaHost} HorizontalOptions="Fill" HeightRequest={140} VisualEffects={[plasma]} />
        </Card>

        <Card title={`ShaderSource="shaders/blit.sksl" (pass-through) toggled through VisualEffects · ${blit ? "on" : "off"}`}>
          <SkiaImage Source="images/dungeon.jpg" Aspect="AspectCover" HorizontalOptions="Fill" HeightRequest={160} UseCache="Image" VisualEffects={blit ? [blitFx] : []} />
          <SkiaButton Text={blit ? "Remove effect" : "Add effect"} BackgroundColor="#0D6EFD" FontSize={12} Tapped={() => setBlit((b) => !b)} />
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
