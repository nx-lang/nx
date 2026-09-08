import { useRef, useState } from "react";
import { Colors, SkiaButton, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, SkiaSvg, SkiaWrap, Thickness } from "drawnui-react";
import { Easing, type SkiaSvg as SkiaSvgCtrl } from "drawnui-react/core";

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#2B3035" HorizontalOptions="Fill">
      <SkiaStack Spacing={10} Padding={new Thickness(16, 12)}>
        <SkiaLabel Text={title} FontSize={12} TextColor="#6EA8FE" FontAttributes="Bold" TextTransform="Uppercase" FontFamilyFallback="FontSymbols,FontSymbols2" />
        {children}
      </SkiaStack>
    </SkiaShape>
  );
}

/** A 96x64 tile with a caption, used to show one transform each. */
function Tile({ text, ...transform }: { text: string; Rotation?: number; ScaleX?: number; ScaleY?: number; SkewX?: number; SkewY?: number; TranslationX?: number; TranslationY?: number; Opacity?: number; AnchorX?: number; AnchorY?: number }) {
  return (
    <SkiaStack Spacing={6} WidthRequest={120} Padding={new Thickness(0, 12)}>
      <SkiaShape Type="Rectangle" CornerRadius={10} BackgroundColor="#0D6EFD" WidthRequest={96} HeightRequest={56} HorizontalOptions="Center" {...transform}>
        <SkiaLabel Text="DrawnUI" FontSize={14} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
      </SkiaShape>
      <SkiaLabel Text={text} FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Center" HorizontalTextAlignment="Center" />
    </SkiaStack>
  );
}

/** Render transforms (MAUI VisualElement names), Opacity, hit-testing through transforms and the *ToAsync animations. */
export function TransformsPage() {
  const logo = useRef<SkiaSvgCtrl>(null);
  const [taps, setTaps] = useState(0);
  const [spinning, setSpinning] = useState(false);
  const spin = useRef<AbortController | null>(null);

  const toggleSpin = async () => {
    const svg = logo.current;
    if (!svg) return;
    if (spin.current) { spin.current.abort(); spin.current = null; setSpinning(false); return; }
    const c = new AbortController();
    spin.current = c;
    setSpinning(true);
    try {
      while (!c.signal.aborted) { svg.Rotation = 0; await svg.RotateToAsync(360, 1200, Easing.Linear, c.signal); }
    } catch { /* aborted */ }
  };

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Transforms" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaLabel Text="Applied at render around the arranged box, so layout is untouched and caches stay valid. Same names as MAUI: TranslationX/Y, Rotation, ScaleX/Y, SkewX/Y, AnchorX/Y, Opacity." FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Fill" HorizontalTextAlignment="Center" />

        <Card title="One property each">
          <SkiaWrap Spacing={4} HorizontalOptions="Center">
            <Tile text="none" />
            <Tile text="Rotation={15}" Rotation={15} />
            <Tile text="Scale={1.3}" ScaleX={1.3} ScaleY={1.3} />
            <Tile text="ScaleX={-1}" ScaleX={-1} />
            <Tile text="SkewX={20}" SkewX={20} />
            <Tile text="TranslationY={10}" TranslationY={10} />
            <Tile text="Opacity={0.35}" Opacity={0.35} />
            <Tile text="Rotation={15} AnchorX/Y={0}" Rotation={15} AnchorX={0} AnchorY={0} />
          </SkiaWrap>
        </Card>

        <Card title="Gestures map through transforms — tap the rotated, scaled button">
          <SkiaStack Spacing={8} HeightRequest={120} HorizontalOptions="Fill">
            <SkiaButton Text={`Tapped ${taps}×`} BackgroundColor="#D63384" HorizontalOptions="Center" VerticalOptions="Center" Rotation={-20} ScaleX={1.4} ScaleY={1.4} ApplyEffect="Ripple" Tapped={() => setTaps((t) => t + 1)} />
          </SkiaStack>
          <SkiaLabel Text="The hit rect is the drawn one (inverse RenderTransformMatrix), not the layout box; the ripple lands under the finger." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <Card title="Animations — FadeToAsync, ScaleToAsync, TranslateToAsync, RotateToAsync (Promises)">
          <SkiaStack Spacing={0} HeightRequest={140} HorizontalOptions="Fill">
            <SkiaSvg ref={logo} Source="images/drawnui.svg" WidthRequest={100} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" />
          </SkiaStack>
          <SkiaWrap Spacing={6}>
            <SkiaButton Text="Fade" BackgroundColor="#0D6EFD" Tapped={() => { void logo.current?.FadeToAsync(0.15, 300).then(() => logo.current?.FadeToAsync(1, 300)); }} />
            <SkiaButton Text="Scale" BackgroundColor="#0D6EFD" Tapped={() => { void logo.current?.ScaleToAsync(1.6, 1.6, 250, Easing.CubicOut).then(() => logo.current?.ScaleToAsync(1, 1, 250, Easing.CubicIn)); }} />
            <SkiaButton Text="Translate" BackgroundColor="#0D6EFD" Tapped={() => { void logo.current?.TranslateToAsync(140, 0, 300, Easing.CubicInOut).then(() => logo.current?.TranslateToAsync(0, 0, 300, Easing.CubicInOut)); }} />
            <SkiaButton Text="Rotate" BackgroundColor="#0D6EFD" Tapped={() => { const s = logo.current; if (s) { s.Rotation = 0; void s.RotateToAsync(360, 600, Easing.CubicInOut); } }} />
            <SkiaButton Text={spinning ? "Stop spin" : "Spin"} BackgroundColor={spinning ? "#DC3545" : "#20C997"} Tapped={() => void toggleSpin()} />
          </SkiaWrap>
          <SkiaLabel Text="Each *ToAsync cancels its previous run of the same kind (like the C# per-property CancellationTokenSource); pass an AbortSignal to cancel from outside." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
