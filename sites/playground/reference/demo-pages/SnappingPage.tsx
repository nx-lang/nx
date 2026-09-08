import { useRef, useState } from "react";
import { Colors, SkiaButton, SkiaCarousel, SkiaDrawer, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, Thickness } from "drawnui-react";
import type { SkiaCarousel as SkiaCarouselCtrl, SkiaDrawer as SkiaDrawerCtrl } from "drawnui-react/core";

const SLIDES = [
  { title: "Slide 1", color: "#0D6EFD" }, { title: "Slide 2", color: "#6610F2" }, { title: "Slide 3", color: "#D63384" }, { title: "Slide 4", color: "#20C997" },
];

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

/** SkiaCarousel + SkiaDrawer: SnappingLayout descendants — swipe / drag, snap by velocity, programmatic state. */
export function SnappingPage() {
  const carousel = useRef<SkiaCarouselCtrl>(null);
  const drawer = useRef<SkiaDrawerCtrl>(null);
  const [index, setIndex] = useState(0);
  const [peekIndex, setPeekIndex] = useState(1);
  const [open, setOpen] = useState(false);

  return (
    <SkiaLayer VerticalOptions="Fill">
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16, 16, 16, 260)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Carousel & Drawer" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title={`SkiaCarousel — swipe horizontally · SelectedIndex=${index}`}>
          <SkiaCarousel ref={carousel} HeightRequest={160} SelectedIndex={index} SelectedIndexChanged={(_, i) => setIndex(i)} Bounces>
            {SLIDES.map((s) => (
              <SkiaShape key={s.title} Type="Rectangle" CornerRadius={12} BackgroundColor={s.color}>
                <SkiaLabel Text={s.title} FontSize={24} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
              </SkiaShape>
            ))}
          </SkiaCarousel>
          <SkiaRow Spacing={8}>
            <SkiaButton Text="Prev" BackgroundColor="#0D6EFD" Tapped={() => carousel.current?.GoPrev()} />
            <SkiaButton Text="Next" BackgroundColor="#0D6EFD" Tapped={() => carousel.current?.GoNext()} />
            <SkiaButton Text="ScrollTo(3)" BackgroundColor="#20C997" Tapped={() => carousel.current?.ScrollTo(3, true)} />
            <SkiaRow Spacing={6} VerticalOptions="Center" Margin={new Thickness(8, 0, 0, 0)}>
              {SLIDES.map((s, i) => <SkiaShape key={s.title} Type="Circle" WidthRequest={10} LockRatio={1} BackgroundColor={i === index ? Colors.White : "#6C757D"} />)}
            </SkiaRow>
          </SkiaRow>
        </Card>

        <Card title={`SidesOffset={40} Spacing={12} — neighbours peek in · SelectedIndex=${peekIndex}`}>
          <SkiaCarousel HeightRequest={140} SidesOffset={40} Spacing={12} SelectedIndex={peekIndex} SelectedIndexChanged={(_, i) => setPeekIndex(i)}>
            {SLIDES.map((s) => (
              <SkiaShape key={s.title} Type="Rectangle" CornerRadius={12} BackgroundColor={s.color}>
                <SkiaLabel Text={s.title} FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
              </SkiaShape>
            ))}
          </SkiaCarousel>
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
          <SkiaShape Type="Rectangle" CornerRadius={20} BackgroundColor="#F5F5F5" HorizontalOptions="Fill" VerticalOptions="Fill" Shadows={[{ X: 0, Y: -2, Blur: 8, Opacity: 0.4, Color: Colors.Black }]}>
            <SkiaStack Spacing={0} HorizontalOptions="Fill">
              <SkiaShape Type="Rectangle" CornerRadius={20} BackgroundColor="#0D6EFD" HeightRequest={56} HorizontalOptions="Fill">
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
