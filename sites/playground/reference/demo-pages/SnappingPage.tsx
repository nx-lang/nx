import { useCallback, useRef, useState } from "react";
import { Colors, CornerRadius, SkiaButton, SkiaCarousel, SkiaDrawer, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaCarousel as SkiaCarouselCtrl, SkiaDrawer as SkiaDrawerCtrl } from "drawnui-react/core";
import { SlideCell, type Slide } from "./SlideCell";

// same palette as the Sandbox MainPageCarousels page
const SLIDES = [
  { title: "1", color: "#E94560", text: Colors.White }, { title: "2", color: "#0F3460", text: Colors.White }, { title: "3", color: "#533483", text: Colors.White }, { title: "4", color: "#A8DF8E", text: "#1A1A2E" },
];
const PEEK = [
  { title: "Slide 1", color: "#0D6EFD" }, { title: "Slide 2", color: "#6610F2" }, { title: "Slide 3", color: "#D63384" }, { title: "Slide 4", color: "#20C997" },
];
// templated sources: only the visible slides (+ neighbours) exist as cells, the rest is recycled
const LOOP_ITEMS: Slide[] = Array.from({ length: 12 }, (_, i) => ({ title: `Item ${i + 1}`, color: ["#0D6EFD", "#6610F2", "#D63384", "#20C997", "#FD7E14", "#0DCAF0"][i % 6] }));
const DYN_ITEMS: Slide[] = [{ title: "80 pt", color: "#0D6EFD", height: 80 }, { title: "160 pt", color: "#6610F2", height: 160 }, { title: "110 pt", color: "#20C997", height: 110 }];

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

function Toggle({ text, on, onTap }: { text: string; on: boolean; onTap: () => void }) {
  return <SkiaButton Text={`${text}: ${on ? "On" : "Off"}`} BackgroundColor={on ? "#20C997" : "#495057"} TextColor={on ? "#1A1A2E" : Colors.White} FontSize={13} Tapped={onTap} />;
}

/** SkiaCarousel + SkiaDrawer: SnappingLayout descendants — swipe / drag, snap by velocity, programmatic state. */
export function SnappingPage() {
  const carousel = useRef<SkiaCarouselCtrl>(null);
  const drawer = useRef<SkiaDrawerCtrl>(null);
  const loop = useRef<SkiaCarouselCtrl>(null);
  // sandbox-style playground state
  const [index, setIndex] = useState(0);
  const [looped, setLooped] = useState(false);
  const [bounces, setBounces] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [inTransition, setInTransition] = useState(false);
  const [preload, setPreload] = useState(true);
  const [sides, setSides] = useState(40);
  const [vertical, setVertical] = useState(false);
  const [appeared, setAppeared] = useState("");
  const [peekIndex, setPeekIndex] = useState(1);
  const [loopIndex, setLoopIndex] = useState(0);
  const [dynIndex, setDynIndex] = useState(0);
  const [open, setOpen] = useState(false);
  // ItemTemplate must be a stable reference: a new function on every render would rebuild the cell pool
  const template = useCallback(() => new SlideCell(), []);

  return (
    <SkiaLayer VerticalOptions="Fill">
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16, 16, 16, 260)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Carousel & Drawer" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title="SkiaCarousel playground (Sandbox MainPageCarousels)">
          <SkiaWrap Spacing={8}>
            <Toggle text="IsLooped" on={looped} onTap={() => setLooped((v) => !v)} />
            <Toggle text="Bounces" on={bounces} onTap={() => setBounces((v) => !v)} />
            <Toggle text="PreloadNeighboors" on={preload} onTap={() => setPreload((v) => !v)} />
            <Toggle text="IsVertical" on={vertical} onTap={() => setVertical((v) => !v)} />
            <SkiaButton Text={`SidesOffset: ${sides}`} BackgroundColor="#495057" FontSize={13} Tapped={() => setSides((s) => (s === 40 ? 0 : s === 0 ? 20 : 40))} />
          </SkiaWrap>
          <SkiaWrap Spacing={8}>
            <SkiaButton Text="← Prev" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.GoPrev()} />
            <SkiaButton Text="Next →" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.GoNext()} />
            <SkiaButton Text="ScrollTo(2)" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.ScrollTo(2, true)} />
            <SkiaButton Text="ScrollTo(0, no anim)" BackgroundColor="#0F3460" FontSize={13} Tapped={() => carousel.current?.ScrollTo(0, false)} />
            <SkiaButton Text="Set index 3" BackgroundColor="#0F3460" FontSize={13} Tapped={() => setIndex(3)} />
          </SkiaWrap>
          <SkiaCarousel ref={carousel} HeightRequest={250} BackgroundColor="#16213E" IsLooped={looped} Bounces={bounces} SwipeSpeed={speed} SidesOffset={sides} Spacing={20}
            PreloadNeighboors={preload} IsVertical={vertical} SelectedIndex={index} SelectedIndexChanged={(_, i) => setIndex(i)}
            TransitionChanged={(_, t) => setInTransition(t)} ItemAppearing={(_, i) => setAppeared(`ItemAppearing ${i}`)} ItemDisappearing={(_, i) => setAppeared(`ItemDisappearing ${i}`)}>
            {SLIDES.map((s) => (
              <SkiaShape key={s.title} Type="Rectangle" BackgroundColor={s.color} UseCache="Operations">
                <SkiaLabel Text={s.title} FontSize={60} FontFamily="FontTextBold" TextColor={s.text} HorizontalOptions="Center" VerticalOptions="Center" />
              </SkiaShape>
            ))}
          </SkiaCarousel>
          {/* indicators: the selected dot stretches to 24 like the DataTrigger in the sandbox */}
          <SkiaRow Spacing={8} HorizontalOptions="Center">
            {SLIDES.map((s, i) => <SkiaShape key={s.title} Type="Rectangle" CornerRadius={4} WidthRequest={i === index ? 24 : 8} HeightRequest={8} BackgroundColor={s.color} />)}
          </SkiaRow>
          <SkiaLabel Text={`Selected Index: ${index}   ·   InTransition: ${inTransition}   ·   ${looped ? "Looping enabled - infinite scroll" : "Looping disabled - bounded scroll"}   ·   ${appeared}`} FontSize={13} TextColor="#ADB5BD" HorizontalOptions="Fill" />
          <SkiaRow Spacing={8} VerticalOptions="Center">
            <SkiaLabel Text="Swipe Speed" FontSize={14} TextColor={Colors.White} VerticalOptions="Center" />
            {[0.5, 1, 2].map((v) => <SkiaButton key={v} Text={`${v.toFixed(1)}x`} BackgroundColor={speed === v ? "#533483" : "#495057"} FontSize={13} Tapped={() => setSpeed(v)} />)}
            <SkiaLabel Text={`Current: ${speed.toFixed(1)}x`} FontSize={12} TextColor="#ADB5BD" VerticalOptions="Center" />
          </SkiaRow>
        </Card>

        <Card title={`SidesOffset={40} Spacing={12} — neighbours peek in · SelectedIndex=${peekIndex}`}>
          <SkiaCarousel HeightRequest={140} SidesOffset={40} Spacing={12} SelectedIndex={peekIndex} SelectedIndexChanged={(_, i) => setPeekIndex(i)}>
            {PEEK.map((s) => (
              <SkiaShape key={s.title} Type="Rectangle" CornerRadius={12} BackgroundColor={s.color}>
                <SkiaLabel Text={s.title} FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
              </SkiaShape>
            ))}
          </SkiaCarousel>
        </Card>

        <Card title={`IsLooped + ItemsSource/ItemTemplate (12 recycled cells) · SelectedIndex=${loopIndex}`}>
          <SkiaCarousel ref={loop} HeightRequest={130} IsLooped SidesOffset={30} Spacing={10} LinearSpeedMs={350} ItemsSource={LOOP_ITEMS} ItemTemplate={template}
            SelectedIndex={loopIndex} SelectedIndexChanged={(_, i) => setLoopIndex(i)} />
          <SkiaRow Spacing={8}>
            <SkiaButton Text="Prev" BackgroundColor="#0D6EFD" Tapped={() => loop.current?.GoPrev()} />
            <SkiaButton Text="Next" BackgroundColor="#0D6EFD" Tapped={() => loop.current?.GoNext()} />
            <SkiaLabel FontFamilyFallback="FontSymbols,FontSymbols2" Text="Wraps last → first both ways (virtual anchors); LinearSpeedMs=350 = one slide per 350 ms without Bounces; cells are recycled through ItemTemplate." FontSize={12} TextColor="#ADB5BD" VerticalOptions="Center" HorizontalOptions="Fill" />
          </SkiaRow>
        </Card>

        <Card title={`DynamicSize — auto height follows the selected slide · SelectedIndex=${dynIndex}`}>
          <SkiaCarousel DynamicSize VerticalOptions="Start" Bounces ItemsSource={DYN_ITEMS} ItemTemplate={template} SelectedIndex={dynIndex} SelectedIndexChanged={(_, i) => setDynIndex(i)} />
          <SkiaLabel Text="No HeightRequest: the carousel measures the selected cell (80 / 160 / 110 pt) and re-measures on every index change." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <Card title="SkiaDrawer — drag the header below, or:">
          <SkiaRow Spacing={8}>
            <SkiaButton Text={open ? "Close drawer" : "Open drawer"} BackgroundColor="#6610F2" Tapped={() => drawer.current && (drawer.current.IsOpen = !drawer.current.IsOpen)} />
            <SkiaLabel Text={`IsOpen: ${open}`} FontSize={14} TextColor="#DEE2E6" VerticalOptions="Center" />
          </SkiaRow>
          <SkiaLabel Text="Direction=FromBottom HeaderSize=56, sits in a SkiaLayer with VerticalOptions=End; snaps by velocity, Bounces enabled." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>
      </SkiaStack>
    </SkiaScroll>

      {/* the drawer lives in its own full-size layer over the page, anchored to the bottom edge */}
      <SkiaLayer VerticalOptions="Fill">
        <SkiaDrawer ref={drawer} Direction="FromBottom" HeaderSize={56} HeightRequest={320} VerticalOptions="End" HorizontalOptions="Fill" Bounces IsOpenChanged={(_, v) => setOpen(v)}>
          <SkiaShape Type="Rectangle" CornerRadius={new CornerRadius(20, 20, 0, 0)} BackgroundColor="#F5F5F5" HorizontalOptions="Fill" VerticalOptions="Fill" Shadows={[{ X: 0, Y: -2, Blur: 8, Opacity: 0.4, Color: Colors.Black }]}>
            <SkiaStack Spacing={0} HorizontalOptions="Fill">
              {/* top corners only, like a MAUI CornerRadius="20,20,0,0" */}
              <SkiaShape Type="Rectangle" CornerRadius={new CornerRadius(20, 20, 0, 0)} BackgroundColor="#0D6EFD" HeightRequest={56} HorizontalOptions="Fill">
                <SkiaShape Type="Rectangle" CornerRadius={3} BackgroundColor="#FFFFFF" WidthRequest={44} HeightRequest={5} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
                <SkiaLabel Text="Drag me" FontSize={16} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" Margin={new Thickness(0, 10, 0, 0)} />
              </SkiaShape>
              <SkiaStack Spacing={12} Padding={new Thickness(20)}>
                <SkiaLabel Text="Drawer content" FontSize={20} FontFamily="FontTextBold" TextColor="#111827" />
                <SkiaLabel Text="Everything inside is a normal drawn tree: buttons keep working, the drawer only takes vertical drags. Release with a flick to snap open or closed." FontSize={14} TextColor="#374151" HorizontalOptions="Fill" />
                <SkiaButton Text="Close" ControlStyle="Material" Tapped={() => drawer.current?.Close()} />
              </SkiaStack>
            </SkiaStack>
          </SkiaShape>
        </SkiaDrawer>
      </SkiaLayer>
    </SkiaLayer>
  );
}
