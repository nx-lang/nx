import { useEffect, useMemo, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaImage, SkiaImageTiles, SkiaLabel, SkiaScroll, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import { type SkiaImageEffect, type SkiaImageTiles as SkiaImageTilesCtrl, SkiaImageManager, Super, type TransformAspect } from "drawnui-react/core";

const EFFECTS: { title: string; props: Record<string, unknown> }[] = [
  { title: "BlackAndWhite", props: { AddEffect: "BlackAndWhite" as SkiaImageEffect } },
  { title: "Sepia", props: { AddEffect: "Sepia" } },
  { title: "Pastel", props: { AddEffect: "Pastel" } },
  { title: "InvertColors", props: { AddEffect: "InvertColors" } },
  { title: "Tint #0D6EFD Multiply", props: { AddEffect: "Tint", ColorTint: "#0D6EFD", EffectBlendMode: "Multiply" } },
  { title: "Darken={80}", props: { AddEffect: "Darken", Darken: 80 } },
  { title: "Lighten={80}", props: { AddEffect: "Lighten", Lighten: 80 } },
  { title: "Contrast={1.5}", props: { AddEffect: "Contrast", Contrast: 1.5 } },
  { title: "Saturation={2}", props: { AddEffect: "Saturation", Saturation: 2 } },
  { title: "Blur={3}", props: { Blur: 3 } },
  { title: "ZoomX/Y={1.8}", props: { ZoomX: 1.8, ZoomY: 1.8 } },
  { title: "HorizontalOffset={-40}", props: { HorizontalOffset: -40, Aspect: "AspectFit" as TransformAspect } },
  { title: "HSL Gamma=0.6 (hue) Sat=1 Bright=0.5", props: { AddEffect: "HSL", BackgroundColor: "#FFFFFF", Gamma: 0.6, Saturation: 1, Brightness: 0.5 } },
];

/** Preload queue demo: 8 distinct urls of the same photo (cache-busting query) through the priority queue, 5 in flight at once. */
const PRELOAD = Array.from({ length: 8 }, (_, i) => `images/glass2.jpg?queue=${i}`);

const ASPECTS: TransformAspect[] = ["AspectCover", "AspectFit", "AspectFill", "AspectFitFill", "Fill", "Fit", "FitFill", "Cover", "None"];

/** SkiaImage: one source, every TransformAspect side by side (default is AspectCover = crop to fill). */
export function ImagesPage() {
  // AddEffect="Custom": the color filter is yours (C# PaintColorFilter), here a red/blue channel swap
  const swapRB = useMemo(() => Super.CK.ColorFilter.MakeMatrix([0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0]), []);
  const dilate = useMemo(() => Super.CK.ImageFilter.MakeDilate(4, 4, null), []);
  useEffect(() => () => { swapRB.delete(); dilate.delete(); }, [swapRB, dilate]);
  const tiles = useRef<SkiaImageTilesCtrl>(null);
  const [offset, setOffset] = useState(0);
  useEffect(() => { const id = setInterval(() => setOffset((o) => o + 4), 50); return () => clearInterval(id); }, []);
  const [queue, setQueue] = useState("idle");
  const preload = () => {
    for (const s of PRELOAD) SkiaImageManager.Instance.Clear(s);
    const m = SkiaImageManager.Instance;
    let peak = 0;
    const tick = setInterval(() => { peak = Math.max(peak, m.RunningCount); setQueue(`running ${m.RunningCount} · queued ${m.QueuedCount} · peak ${peak}`); }, 10);
    const started = performance.now();
    void m.PreloadImages(PRELOAD, "Low").then(() => { clearInterval(tick); setQueue(`8 loaded in ${Math.round(performance.now() - started)} ms · peak in flight ${peak} (MaxParallelLoads=${SkiaImageManager.MaxParallelLoads})`); });
  };
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)}>
        <SkiaLabel Text="SkiaImage · Aspect" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaLabel Text="Same 512×512 photo in a 220×120 box. Overflow is clipped to the box." FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Center" />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={720}>
          {ASPECTS.map((aspect) => (
            <SkiaStack key={aspect} Spacing={4} WidthRequest={220}>
              <SkiaImage Source="images/baboon.jpg" WidthRequest={220} HeightRequest={120} Aspect={aspect} BackgroundColor={Colors.Black} />
              <SkiaLabel Text={aspect} FontSize={15} TextColor={Colors.White} />
              <SkiaLabel Text={`Aspect="${aspect}"`} FontSize={12} TextColor="#94A3B8" />
            </SkiaStack>
          ))}
        </SkiaWrap>
        <SkiaLabel Text="Effects · AddEffect, Blur, Zoom, offsets" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={720}>
          {EFFECTS.map((e) => (
            <SkiaStack key={e.title} Spacing={4} WidthRequest={160}>
              <SkiaImage Source="images/baboon.jpg" WidthRequest={160} HeightRequest={100} BackgroundColor={Colors.Black} {...e.props} />
              <SkiaLabel Text={e.title} FontSize={12} TextColor="#94A3B8" />
            </SkiaStack>
          ))}
        </SkiaWrap>
        <SkiaLabel Text='Custom filters · AddEffect="Custom" + PaintColorFilter / PaintImageFilter' FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={720}>
          <SkiaStack Spacing={4} WidthRequest={160}>
            <SkiaImage Source="images/baboon.jpg" WidthRequest={160} HeightRequest={100} AddEffect="Custom" PaintColorFilter={swapRB} />
            <SkiaLabel Text="PaintColorFilter = MakeMatrix (R↔B)" FontFamilyFallback="FontSymbols,FontSymbols2" FontSize={12} TextColor="#94A3B8" />
          </SkiaStack>
          <SkiaStack Spacing={4} WidthRequest={160}>
            <SkiaImage Source="images/baboon.jpg" WidthRequest={160} HeightRequest={100} PaintImageFilter={dilate} />
            <SkiaLabel Text="PaintImageFilter = MakeDilate(4)" FontSize={12} TextColor="#94A3B8" />
          </SkiaStack>
        </SkiaWrap>
        <SkiaLabel Text="SkiaImageTiles · TileWidth/TileHeight, TileOffsetX animates" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaImageTiles ref={tiles} Source="images/baboon.jpg" TileWidth={64} TileHeight={64} TileOffsetX={offset} TileOffsetY={offset / 2} HorizontalOptions="Center" WidthRequest={400} HeightRequest={160} BackgroundColor={Colors.Black} />
        <SkiaLabel Text={`SkiaImageManager preload queue · ${queue}`} FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaButton Text="PreloadImages(8 urls, Low)" BackgroundColor="#0D6EFD" FontSize={13} HorizontalOptions="Center" Tapped={preload} />
        <SkiaLabel Text="Alignment inside the box" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={720}>
          <SkiaImage Source="images/baboon.jpg" WidthRequest={120} HeightRequest={120} Aspect="AspectFit" HorizontalAlignment="Start" VerticalAlignment="Start" BackgroundColor={Colors.Black} />
          <SkiaImage Source="images/baboon.jpg" WidthRequest={120} HeightRequest={120} Aspect="Fit" HorizontalAlignment="Center" VerticalAlignment="Center" BackgroundColor={Colors.Black} />
          <SkiaImage Source="images/baboon.jpg" WidthRequest={120} HeightRequest={120} Aspect="Fit" HorizontalAlignment="End" VerticalAlignment="End" BackgroundColor={Colors.Black} />
        </SkiaWrap>
      </SkiaStack>
    </SkiaScroll>
  );
}
