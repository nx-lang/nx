import { Aria, Colors, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, SkiaSvg, Thickness, useShell } from "drawnui-react";

const SAMPLES: { route: string; title: string; text: string }[] = [
  { route: "cells", title: "Recycled cells", text: "100 000 items in a SkiaScroll, RecyclingTemplate + MeasureFirst, UseCache=Image" },
  { route: "uneven", title: "Uneven cells", text: "Rows of different heights — MeasureVisible, LoadMore at both ends, ImageDoubleBuffered cells" },
  { route: "images", title: "Images", text: "SkiaImage — every TransformAspect, alignment, clipping" },
  { route: "svg", title: "SVG", text: "SkiaSvg — file and inline sources, TintColor, LockRatio" },
  { route: "shapes", title: "Shapes", text: "SkiaShape — rectangle, circle, ellipse, arc, polygon, line, path; stroke, corner radii, clipping" },
  { route: "text", title: "Text", text: "SkiaLabel — word wrap, MaxLines, alignment, spans, weights, glyph fallback" },
  { route: "layouts", title: "Layouts", text: "Every SkiaLayout type — Absolute, Column, Row, Wrap, Grid (tracks, spans, spacing)" },
  { route: "looks", title: "Platform Looks", text: "SkiaSwitch, SkiaCheckbox, SkiaRadioButton, SkiaProgress, SkiaSlider, SkiaButton — Default, Windows, Cupertino, Material, Material3" },
  { route: "snapping", title: "Carousel & Drawer", text: "SkiaCarousel (swipe, SidesOffset peek, SelectedIndex) and SkiaDrawer (drag from an edge, snap by velocity)" },
  { route: "transforms", title: "Transforms", text: "Rotation, Scale, Skew, Translation, Opacity — hit-testing through them, *ToAsync animations" },
  { route: "a11y", title: "Accessibility", text: "ARIA overlay over the canvas — roles, labels, hints, toggles, live regions, keyboard" },
];

/** Root menu styled after drawnui.net: dark body, logo + bold title, sample cards below. */
export function RootPage() {
  const shell = useShell();
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={24} Padding={new Thickness(24, 24, 24, 40)} HorizontalOptions="Center" MaximumWidthRequest={820}>
        <SkiaSvg Source="images/drawnui.svg" WidthRequest={120} LockRatio={1} HorizontalOptions="Center" Margin={new Thickness(0, 16, 0, 0)} AccessibilityRole={Aria.RoleImg} AccessibilityLabel="DrawnUI logo" />
        <SkiaLabel Text="DrawnUI for React" FontSize={48} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" AccessibilityRole={Aria.RoleHeading} />

        {/* samples */}
        <SkiaLabel Text="Snippets" FontSize={32} TextColor="#DEE2E6" Margin={new Thickness(0, 8, 0, 0)} AccessibilityRole={Aria.RoleHeading} />
        {SAMPLES.map((s) => (
          <SkiaShape key={s.route} Type="Rectangle" CornerRadius={12} BackgroundColor="#2B3035" StrokeColor="#373B3E" StrokeWidth={1} AnimationTapped="Ripple" Tapped={() => void shell.GoToAsync(s.route)}
            AccessibilityRole={Aria.RoleButton} AccessibilityLabel={s.title} AccessibilityHint={s.text}>
            <SkiaStack Spacing={6} Padding={new Thickness(24, 20, 56, 20)}>
              <SkiaLabel Text={s.title} FontSize={24} FontFamily="FontTextBold" TextColor={Colors.White} AccessibilityRole={Aria.RolePresentation} />
              <SkiaLabel Text={s.text} FontSize={14} TextColor="#ADB5BD" AccessibilityRole={Aria.RolePresentation} />
            </SkiaStack>
            <SkiaLabel Text="›" FontSize={28} TextColor="#6EA8FE" HorizontalOptions="End" VerticalOptions="Center" Margin={new Thickness(0, 0, 24, 0)} AccessibilityRole={Aria.RolePresentation} />
          </SkiaShape>
        ))}

        <SkiaLabel Text="helloreact.drawnui.net · github.com/DrawnUi/DrawnUi.React · MIT" FontSize={12} TextColor="#6C757D" HorizontalOptions="Center" Margin={new Thickness(0, 16, 0, 0)} />
      </SkiaStack>
    </SkiaScroll>
  );
}
