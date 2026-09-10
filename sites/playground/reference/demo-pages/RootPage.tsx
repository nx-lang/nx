import { useEffect, useState } from "react";
import { PUBLISH } from "../publish";
import { SAMPLES } from "./catalog";
import { Aria, Colors, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, SkiaSvg, SkiaWrap, TextSpan, Thickness, useShell } from "drawnui-react";


/** Root menu styled after drawnui.net: dark body, logo + bold title, sample cards below. */
const SUBTITLE_MARGIN = new Thickness(0, -12, 0, 0); // hoisted: a new Thickness per render would remeasure
const MAX_WIDTH = 820, PAGE_PADDING = 24, GAP = 16;
// title gradients cycle through the drawnui.net accents
const TITLE_GRADIENTS: [string, string][] = [["#6EA8FE", "#0D6EFD"], ["#D63384", "#FD7E14"], ["#20C997", "#0DCAF0"], ["#FFC107", "#FD7E14"], ["#A98EFF", "#6610F2"], ["#0DCAF0", "#6EA8FE"]];

const FOOTER_MARGIN = new Thickness(0, 16, 0, 0);

/** Opens the repository in a new tab; called from the drawn footer link. */
function OpenRepository(): void {
  window.open("https://github.com/DrawnUi/DrawnUi.React", "_blank", "noopener,noreferrer");
}

/** Card width in points: two columns when the page is wide enough, one otherwise (SkiaWrap flows them). */
function useCardWidth(): number {
  const compute = () => { const inner = Math.min(MAX_WIDTH, window.innerWidth) - PAGE_PADDING * 2; return inner >= 640 ? (inner - GAP) / 2 : inner; };
  const [width, setWidth] = useState(compute);
  useEffect(() => { const onResize = () => setWidth(compute()); window.addEventListener("resize", onResize); return () => window.removeEventListener("resize", onResize); }, []);
  return width;
}

export function RootPage() {
  const shell = useShell();
  const cardWidth = useCardWidth();
  return (
    <SkiaScroll Orientation="Vertical">

      <SkiaStack Spacing={24} Padding={new Thickness(24, 24, 24, 40)} HorizontalOptions="Center" MaximumWidthRequest={820} UseCache="Image">
        <SkiaSvg Source="images/drawnui.svg" WidthRequest={120} LockRatio={1} HorizontalOptions="Center" Margin={new Thickness(0, 16, 0, 0)} AccessibilityRole={Aria.RoleImg} AccessibilityLabel="DrawnUI logo" />
        <SkiaLabel Text="DrawnUI for React" FontSize={48} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" AccessibilityRole={Aria.RoleHeading} />
        <SkiaLabel Text="A UI rendering engine on top of CanvasKit: layouts, controls, gestures, effects and animations" FontSize={16} TextColor="#ADB5BD" HorizontalOptions="Center" HorizontalTextAlignment="Center" Margin={SUBTITLE_MARGIN} />

        {/* samples */}
        {/* two columns on wide screens, one on phones: fixed-width cards flowing in a SkiaWrap */}
        <SkiaWrap Spacing={GAP} HorizontalOptions="Fill">
          {SAMPLES.map((s, i) => (
            <SkiaShape key={s.route} Type="Rectangle" CornerRadius={12} BackgroundColor="#2B3035" StrokeColor="#373B3E" StrokeWidth={1} WidthRequest={cardWidth} AnimationTapped="Ripple" Tapped={(_, e) => { if ((e.Parameters.Event.Pointer?.Button ?? "Left") === "Left") void shell.GoToAsync(s.route); }}
              AccessibilityRole={Aria.RoleButton} AccessibilityLabel={s.title} AccessibilityHint={s.text}>
              <SkiaStack Spacing={6} Padding={new Thickness(24, 20, 48, 20)}>
                <SkiaLabel Text={s.title} FontSize={22} FontFamily="FontTextBold" TextColor={Colors.White} FillGradient={{ Type: "Linear", Angle: 0, Colors: TITLE_GRADIENTS[i % TITLE_GRADIENTS.length] }} AccessibilityRole={Aria.RolePresentation} />
                <SkiaLabel Text={s.text} FontSize={13} TextColor="#ADB5BD" AccessibilityRole={Aria.RolePresentation} />
              </SkiaStack>
              <SkiaLabel Text="›" FontSize={28} TextColor="#6EA8FE" HorizontalOptions="End" VerticalOptions="Center" Margin={new Thickness(0, 0, 20, 0)} AccessibilityRole={Aria.RolePresentation} />
            </SkiaShape>
          ))}
        </SkiaWrap>

        {/* the original single centred label; the repository fragment is a tappable span (coloured, no underline) */}
        <SkiaLabel FontSize={12} TextColor="#6C757D" HorizontalOptions="Center" Margin={FOOTER_MARGIN}>
          <TextSpan Text="helloreact.drawnui.net · " />
          <TextSpan Text="github.com/DrawnUi/DrawnUi.React" TextColor="#6EA8FE" Tapped={OpenRepository} />
          <TextSpan Text={` · MIT · Publish ${PUBLISH}`} />
        </SkiaLabel>
      </SkiaStack>
      
    </SkiaScroll>
  );
}
