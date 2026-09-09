import { useRef, useState } from "react";
import { Colors, SkiaButton, SkiaGif, SkiaLabel, SkiaLottie, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaLottie as SkiaLottieCtrl, SkiaGif as SkiaGifCtrl } from "drawnui-react/core";

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

/** SkiaLottie (CanvasKit Skottie) and SkiaGif: AnimatedFramesRenderer descendants. */
export function AnimationsPage() {
  const lottie = useRef<SkiaLottieCtrl>(null);
  const gif = useRef<SkiaGifCtrl>(null);
  const [status, setStatus] = useState("loading…");
  const [toggle, setToggle] = useState(false);
  const [gifStatus, setGifStatus] = useState("loading…");
  const [speed, setSpeed] = useState(1);

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Lottie & GIF" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title={`SkiaLottie — Source="lottie/shield.json" Repeat={-1} · ${status}`}>
          <SkiaRow Spacing={16}>
            <SkiaLottie ref={lottie} Source="lottie/shield.json" WidthRequest={160} HeightRequest={160} Repeat={-1} SpeedRatio={speed}
              Success={(c) => setStatus(`loaded, ${c.TotalFrames} frames, playing`)} Error={(_, e) => setStatus(`error: ${e.message}`)}
              Started={() => setStatus("Started")} Finished={() => setStatus("Finished")} />
            <SkiaStack Spacing={8} VerticalOptions="Center" HorizontalOptions="Fill">
              <SkiaWrap Spacing={8}>
                <SkiaButton Text="Start" BackgroundColor="#0D6EFD" FontSize={13} Tapped={() => lottie.current?.Start()} />
                <SkiaButton Text="Stop" BackgroundColor="#6C757D" FontSize={13} Tapped={() => lottie.current?.Stop()} />
                <SkiaButton Text="Seek(30)" BackgroundColor="#6C757D" FontSize={13} Tapped={() => { lottie.current?.Stop(); lottie.current?.Seek(30); }} />
                <SkiaButton Text="GoToEnd" BackgroundColor="#6C757D" FontSize={13} Tapped={() => { lottie.current?.Stop(); lottie.current?.GoToEnd(); }} />
              </SkiaWrap>
              <SkiaRow Spacing={8}>
                <SkiaLabel Text="SpeedRatio" FontSize={13} TextColor="#ADB5BD" VerticalOptions="Center" />
                {[0.5, 1, 2].map((v) => <SkiaButton key={v} Text={`${v}x`} BackgroundColor={speed === v ? "#533483" : "#495057"} FontSize={13} Tapped={() => setSpeed(v)} />)}
              </SkiaRow>
              <SkiaLabel Text="Skottie renders the vector animation every frame into an ImageDoubleBuffered cache; the animator is the C# RangeAnimator over InPoint..OutPoint." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
            </SkiaStack>
          </SkiaRow>
        </Card>

        <Card title="ColorTint / Colors — colors replaced in the JSON before parsing (C# ApplyTint)">
          <SkiaRow Spacing={12}>
            <SkiaLottie Source="lottie/ok.json" WidthRequest={90} HeightRequest={90} Repeat={-1} />
            <SkiaLottie Source="lottie/ok.json" WidthRequest={90} HeightRequest={90} Repeat={-1} ColorTint="#20C997" />
            <SkiaLottie Source="lottie/ok.json" WidthRequest={90} HeightRequest={90} Repeat={-1} Colors={["#D63384", "#FFC107"]} />
            <SkiaLottie Source="lottie/shield.json" WidthRequest={90} HeightRequest={90} Repeat={-1} ColorTint="#0DCAF0" SpeedRatio={0.5} />
          </SkiaRow>
        </Card>

        <Card title={`IsOn toggle — AutoPlay={false}, DefaultFrame=0 / DefaultFrameWhenOn=-1 · IsOn=${toggle}`}>
          <SkiaRow Spacing={16}>
            <SkiaLottie Source="lottie/ok.json" WidthRequest={90} HeightRequest={90} AutoPlay={false} DefaultFrame={0} DefaultFrameWhenOn={-1} IsOn={toggle} />
            <SkiaButton Text={toggle ? "IsOn = false" : "IsOn = true"} BackgroundColor="#6610F2" VerticalOptions="Center" Tapped={() => setToggle((v) => !v)} />
            <SkiaLabel Text="Stopped animations show DefaultFrame, or DefaultFrameWhenOn (-1 = last frame) when IsOn: the C# recipe for animated checkboxes." FontSize={12} TextColor="#ADB5BD" VerticalOptions="Center" HorizontalOptions="Fill" />
          </SkiaRow>
        </Card>

        <Card title={`SkiaGif — Source="images/banana.gif" Aspect=AspectFitFill · ${gifStatus}`}>
          <SkiaRow Spacing={16}>
            <SkiaGif ref={gif} Source="images/banana.gif" WidthRequest={140} HeightRequest={140} Repeat={-1} BackgroundColor="#212529"
              Started={(c) => { const a = (c as SkiaGifCtrl).Animation; setGifStatus(`${a?.TotalFrames ?? 0} frames, ${a?.DurationMs ?? 0} ms, playing`); }} Finished={() => setGifStatus("Finished")} Error={(_, e) => setGifStatus(`error: ${e.message}`)} />
            <SkiaGif Source="images/banana.gif" WidthRequest={70} HeightRequest={140} Repeat={-1} SpeedRatio={2} Aspect="AspectCover" BackgroundColor="#212529" />
            <SkiaStack Spacing={8} VerticalOptions="Center" HorizontalOptions="Fill">
              <SkiaWrap Spacing={8}>
                <SkiaButton Text="Start" BackgroundColor="#0D6EFD" FontSize={13} Tapped={() => gif.current?.Start()} />
                <SkiaButton Text="Stop" BackgroundColor="#6C757D" FontSize={13} Tapped={() => gif.current?.Stop()} />
                <SkiaButton Text="Seek(-1)" BackgroundColor="#6C757D" FontSize={13} Tapped={() => { gif.current?.Stop(); gif.current?.Seek(-1); }} />
              </SkiaWrap>
              <SkiaLabel Text="Every frame is decoded once (CanvasKit AnimatedImage); the animator runs over 0..DurationMs and picks the frame by time, like C# GifAnimation." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
            </SkiaStack>
          </SkiaRow>
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
