import { useRef, useState } from "react";
import { Colors, RefreshIndicator, SkiaImage, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, Thickness } from "drawnui-react";
import { CornerRadius, type SkiaScroll as SkiaScrollCtrl } from "drawnui-react/core";

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

const PALETTE = ["#0F3460", "#533483", "#1B4332", "#7B2D26", "#495057", "#0D6EFD", "#D63384", "#2D6A4F"];

function Rows({ count, prefix }: { count: number; prefix: string }) {
  return (
    <SkiaStack Spacing={6} Padding={new Thickness(8)}>
      {Array.from({ length: count }, (_, i) => (
        <SkiaShape key={i} Type="Rectangle" CornerRadius={6} BackgroundColor={PALETTE[i % PALETTE.length]} HorizontalOptions="Fill" HeightRequest={40} UseCache="Operations">
          <SkiaLabel Text={`${prefix} ${i + 1}`} FontSize={13} TextColor={Colors.White} VerticalOptions="Center" Margin={new Thickness(12, 0)} />
        </SkiaShape>
      ))}
    </SkiaStack>
  );
}

/** SkiaScroll features: Header (flow / sticky / behind + parallax), Footer, scroll bars, pull to refresh, SnapToChildren, TrackIndexPosition. */
export function ScrollPage() {
  const refreshScroll = useRef<SkiaScrollCtrl>(null);
  const [refreshState, setRefreshState] = useState("pull down past 60 pt");
  const [snapIndex, setSnapIndex] = useState(-1);
  const [trackIndex, setTrackIndex] = useState(-1);
  const startRefresh = (s: SkiaScrollCtrl) => {
    setRefreshState("refreshing… (2 s)");
    setTimeout(() => { s.IsRefreshing = false; setRefreshState("done · pull again"); }, 2000);
  };

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720} UseCache="Operations">
        <SkiaLabel Text="SkiaScroll" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title='Header + Footer in the flow · Tag="Header" / Tag="Footer" children scroll with the content'>
          <SkiaScroll HeightRequest={240} BackgroundColor="#212529" IgnoreWrongDirection>
            <SkiaLayer Tag="Header" HeightRequest={70} BackgroundColor="#0F3460">
              <SkiaLabel Text="Header (70 pt) · scrolls away" FontSize={16} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaLayer>
            <Rows count={14} prefix="Row" />
            <SkiaLayer Tag="Footer" HeightRequest={50} BackgroundColor="#533483">
              <SkiaLabel Text="Footer (50 pt) · after the content" FontSize={14} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaLayer>
          </SkiaScroll>
        </Card>

        <Card title="HeaderSticky · the header stays at the top, drawn over the content">
          <SkiaScroll HeightRequest={220} BackgroundColor="#212529" HeaderSticky IgnoreWrongDirection>
            <SkiaLayer Tag="Header" HeightRequest={44} BackgroundColor="#0D6EFD">
              <SkiaLabel Text="Sticky header" FontSize={15} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaLayer>
            <Rows count={14} prefix="Under sticky" />
          </SkiaScroll>
        </Card>

        <Card title="HeaderBehind + HeaderParallaxRatio=0.5 · the content covers the header, which moves at half speed">
          <SkiaScroll HeightRequest={260} BackgroundColor="#212529" HeaderBehind HeaderParallaxRatio={0.5} ContentOffset={-24} IgnoreWrongDirection>
            <SkiaLayer Tag="Header" HeightRequest={160}>
              <SkiaImage Source="images/baboon.jpg" Aspect="AspectCover" HorizontalOptions="Fill" VerticalOptions="Fill" />
              <SkiaLabel Text="Parallax cover" FontSize={22} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" DropShadowColor="#000000" DropShadowSize={4} />
            </SkiaLayer>
            <SkiaShape Type="Rectangle" CornerRadius={new CornerRadius(24, 24, 0, 0)} BackgroundColor="#2B3035" HorizontalOptions="Fill">
              <Rows count={12} prefix="Content over the cover" />
            </SkiaShape>
          </SkiaScroll>
        </Card>

        <Card title='ScrollBarsVisibility="Vertical" + ScrollBarThumbColor · default SkiaScrollBar, auto-hides 1 s after scrolling'>
          <SkiaScroll HeightRequest={200} BackgroundColor="#212529" ScrollBarsVisibility="Vertical" ScrollBarThumbColor="#6EA8FE" ScrollBarTrackColor="#FFFFFF22" IgnoreWrongDirection>
            <Rows count={16} prefix="Scrollbar row" />
          </SkiaScroll>
          <SkiaScroll Orientation="Horizontal" HeightRequest={70} BackgroundColor="#212529" ScrollBarsVisibility="Horizontal" ScrollBarThumbColor="#FFC107">
            <SkiaRow Spacing={8} Padding={new Thickness(8)}>
              {Array.from({ length: 14 }, (_, i) => <SkiaShape key={i} Type="Rectangle" CornerRadius={6} WidthRequest={120} HeightRequest={50} BackgroundColor={PALETTE[i % PALETTE.length]} UseCache="Operations"><SkiaLabel Text={`H ${i + 1}`} FontSize={13} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" /></SkiaShape>)}
            </SkiaRow>
          </SkiaScroll>
        </Card>

        <Card title={`RefreshEnabled + RefreshIndicator (Tag="RefreshIndicator") · RefreshDistanceLimit 60, RefreshShowDistance 50 · ${refreshState}`}>
          <SkiaScroll ref={refreshScroll} HeightRequest={220} BackgroundColor="#212529" RefreshEnabled RefreshDistanceLimit={60} RefreshCommand={startRefresh} IgnoreWrongDirection>
            <RefreshIndicator Tag="RefreshIndicator" HeightRequest={50}>
              <SkiaShape Type="Rectangle" CornerRadius={20} BackgroundColor="#0D6EFD" HorizontalOptions="Center" VerticalOptions="Center" WidthRequest={160} HeightRequest={36}>
                <SkiaLabel Text="↻ refresh" FontFamilyFallback="FontSymbols,FontSymbols2" FontSize={14} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
              </SkiaShape>
            </RefreshIndicator>
            <Rows count={12} prefix="Pull down" />
          </SkiaScroll>
        </Card>

        <Card title={`SnapToChildren="Center" + TrackIndexPosition="Center" (horizontal) · CurrentIndex ${snapIndex}`}>
          <SkiaScroll Orientation="Horizontal" HeightRequest={120} BackgroundColor="#212529" SnapToChildren="Center" TrackIndexPosition="Center" CurrentIndexChanged={(_, i) => setSnapIndex(i)}>
            <SkiaRow Spacing={12} Padding={new Thickness(8)}>
              {Array.from({ length: 10 }, (_, i) => <SkiaShape key={i} Type="Rectangle" CornerRadius={10} WidthRequest={200} HeightRequest={100} BackgroundColor={PALETTE[i % PALETTE.length]} UseCache="Operations"><SkiaLabel Text={`Snap ${i}`} FontSize={18} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" /></SkiaShape>)}
            </SkiaRow>
          </SkiaScroll>
        </Card>

        <Card title={`TrackIndexPosition="Start" (vertical) · CurrentIndex ${trackIndex} · SnapToChildren="Side"`}>
          <SkiaScroll HeightRequest={180} BackgroundColor="#212529" TrackIndexPosition="Start" SnapToChildren="Side" CurrentIndexChanged={(_, i) => setTrackIndex(i)} IgnoreWrongDirection>
            <Rows count={16} prefix="Tracked" />
          </SkiaScroll>
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
