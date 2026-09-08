import { Colors, SkiaLabel, SkiaRow, SkiaScroll, SkiaStack, SkiaSvg, Thickness } from "drawnui-react";

const STAR = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#FFD700" d="M12 2l3.09 6.26L22 9.27l-5 4.87L18.18 21 12 17.77 5.82 21 7 14.14l-5-4.87 6.91-1.01z"/></svg>`;

/** SkiaSvg: file source, inline SvgString, TintColor, LockRatio sizing. */
export function SvgPage() {
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)}>
        <SkiaLabel Text="SkiaSvg" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaSvg Source="images/drawnui.svg" WidthRequest={200} LockRatio={1} HorizontalOptions="Center" />
        <SkiaLabel Text='Source="images/drawnui.svg" WidthRequest={200} LockRatio={1}' FontSize={12} TextColor="#94A3B8" HorizontalOptions="Center" />

        <SkiaLabel Text="TintColor" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaRow Spacing={24} HorizontalOptions="Center">
          <SkiaSvg Source="images/drawnui.svg" WidthRequest={72} LockRatio={1} TintColor={Colors.White} />
          <SkiaSvg Source="images/drawnui.svg" WidthRequest={72} LockRatio={1} TintColor="#FF6B6B" />
          <SkiaSvg Source="images/drawnui.svg" WidthRequest={72} LockRatio={1} TintColor="#4ECDC4" />
          <SkiaSvg Source="images/drawnui.svg" WidthRequest={72} LockRatio={1} TintColor="#FFD93D" />
        </SkiaRow>

        <SkiaLabel Text="SvgString (inline markup) at three sizes" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 12, 0, 0)} />
        <SkiaRow Spacing={24} HorizontalOptions="Center" VerticalOptions="Center">
          <SkiaSvg SvgString={STAR} WidthRequest={32} LockRatio={1} VerticalOptions="Center" />
          <SkiaSvg SvgString={STAR} WidthRequest={64} LockRatio={1} VerticalOptions="Center" />
          <SkiaSvg SvgString={STAR} WidthRequest={128} LockRatio={1} VerticalOptions="Center" />
        </SkiaRow>
        <SkiaLabel Text="Rasterized by the browser at the displayed pixel size, re-rasterized only when that size changes." FontSize={12} TextColor="#94A3B8" HorizontalOptions="Center" />
      </SkiaStack>
    </SkiaScroll>
  );
}
