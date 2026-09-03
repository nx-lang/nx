import { Colors, SkiaImage, SkiaLabel, SkiaScroll, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaImageEffect, TransformAspect } from "drawnui-react/core";

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
];

const ASPECTS: TransformAspect[] = ["AspectCover", "AspectFit", "AspectFill", "AspectFitFill", "Fill", "Fit", "FitFill", "Cover", "None"];

/** SkiaImage: one source, every TransformAspect side by side (default is AspectCover = crop to fill). */
export function ImagesPage() {
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
