import { Colors, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import { CornerRadius, SkiaPoint } from "drawnui-react/core";

const HEART = "M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z";
const STAR: SkiaPoint[] = [0.5, 0.0, 0.62, 0.38, 1.0, 0.38, 0.69, 0.61, 0.81, 1.0, 0.5, 0.76, 0.19, 1.0, 0.31, 0.61, 0.0, 0.38, 0.38, 0.38]
  .reduce<SkiaPoint[]>((acc, v, i, arr) => (i % 2 === 0 ? [...acc, new SkiaPoint(v, arr[i + 1])] : acc), []);

function Demo({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <SkiaStack Spacing={8} WidthRequest={150}>
      <SkiaShape WidthRequest={150} HeightRequest={110} BackgroundColor="#2B3035" CornerRadius={8}>
        {children}
      </SkiaShape>
      <SkiaLabel Text={title} FontSize={13} TextColor="#ADB5BD" HorizontalOptions="Center" />
    </SkiaStack>
  );
}

/** SkiaShape: every Type, fill + stroke, corner radii, children clipped to the shape. */
export function ShapesPage() {
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={20} Padding={new Thickness(16)}>
        <SkiaLabel Text="SkiaShape" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaLabel Text="Stroke is drawn inside the bounds; children are clipped to the shape." FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Center" />

        <SkiaLabel Text="Shadows" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={680}>
          <Demo title="Shadows=[{Y:4, Blur:6, Opacity:.5}]">
            <SkiaShape Type="Rectangle" CornerRadius={12} BackgroundColor="#FFFFFF" WidthRequest={100} HeightRequest={60} HorizontalOptions="Center" VerticalOptions="Center" Shadows={[{ X: 0, Y: 4, Blur: 6, Opacity: 0.5, Color: Colors.Black }]} />
          </Demo>
          <Demo title="Colored, offset X">
            <SkiaShape Type="Circle" BackgroundColor="#FFC107" WidthRequest={64} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" Shadows={[{ X: 6, Y: 6, Blur: 4, Opacity: 0.8, Color: "#6610F2" }]} />
          </Demo>
          <Demo title="Two shadows (glow + drop)">
            <SkiaShape Type="Rectangle" CornerRadius={30} BackgroundColor="#20C997" WidthRequest={110} HeightRequest={60} HorizontalOptions="Center" VerticalOptions="Center" Shadows={[{ X: 0, Y: 0, Blur: 10, Opacity: 0.9, Color: "#20C997" }, { X: 0, Y: 6, Blur: 4, Opacity: 0.6, Color: Colors.Black }]} />
          </Demo>
          <Demo title="ShadowOnly + hollow ClipBackgroundColor">
            <SkiaShape Type="Rectangle" CornerRadius={12} ClipBackgroundColor StrokeColor="#FFFFFF" StrokeWidth={2} WidthRequest={100} HeightRequest={60} HorizontalOptions="Center" VerticalOptions="Center" Shadows={[{ X: 0, Y: 5, Blur: 5, Opacity: 0.7, Color: Colors.Black }]} />
          </Demo>
        </SkiaWrap>

        <SkiaLabel Text="Types" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <SkiaWrap Spacing={16} HorizontalOptions="Center" MaximumWidthRequest={680}>
          <Demo title='Rectangle CornerRadius={16}'>
            <SkiaShape Type="Rectangle" CornerRadius={16} BackgroundColor="#0D6EFD" StrokeColor={Colors.White} StrokeWidth={3} WidthRequest={110} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="CornerRadius(24, 0, 0, 24)">
            <SkiaShape Type="Rectangle" CornerRadius={new CornerRadius(24, 0, 0, 24)} BackgroundColor="#20C997" WidthRequest={110} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Circle + stroke">
            <SkiaShape Type="Circle" BackgroundColor="#6610F2" StrokeColor="#FFD93D" StrokeWidth={4} WidthRequest={80} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Ellipse">
            <SkiaShape Type="Ellipse" BackgroundColor="#DC3545" WidthRequest={120} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Arc Value1=-90 Value2=270">
            <SkiaShape Type="Arc" Value1={-90} Value2={270} StrokeColor="#0DCAF0" StrokeWidth={8} WidthRequest={80} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Polygon (star, Points)">
            <SkiaShape Type="Polygon" Points={STAR} BackgroundColor="#FFC107" WidthRequest={90} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Line (Points)">
            <SkiaShape Type="Line" Points={[new SkiaPoint(0, 1), new SkiaPoint(0.33, 0.2), new SkiaPoint(0.66, 0.8), new SkiaPoint(1, 0)]} StrokeColor="#FD7E14" StrokeWidth={4} WidthRequest={120} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Path (SVG PathData)">
            <SkiaShape Type="Path" PathData={HEART} BackgroundColor="#E83E8C" WidthRequest={80} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Hollow: ClipBackgroundColor">
            <SkiaShape Type="Rectangle" CornerRadius={12} ClipBackgroundColor BackgroundColor="#0D6EFD" StrokeColor="#0D6EFD" StrokeWidth={3} WidthRequest={110} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Children clipped">
            <SkiaShape Type="Circle" BackgroundColor="#1F2937" StrokeColor="#67E8F9" StrokeWidth={3} WidthRequest={80} LockRatio={1} HorizontalOptions="Center" VerticalOptions="Center">
              <SkiaShape Type="Rectangle" BackgroundColor="#67E8F9" WidthRequest={100} HeightRequest={24} HorizontalOptions="Center" VerticalOptions="End" />
              <SkiaLabel Text="AB" FontSize={22} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" Margin={new Thickness(0, 0, 0, 10)} />
            </SkiaShape>
          </Demo>
          <Demo title="StrokeCap Butt, thin">
            <SkiaShape Type="Line" StrokeCap="Butt" Points={[new SkiaPoint(0, 0.5), new SkiaPoint(1, 0.5)]} StrokeColor={Colors.White} StrokeWidth={1} WidthRequest={120} HeightRequest={40} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
          <Demo title="Gradient fill">
            <SkiaShape Type="Rectangle" CornerRadius={35} FillGradient={{ Type: "Linear", Colors: ["#FF6B6B", "#FFD93D", "#4ECDC4"], EndXRatio: 1, EndYRatio: 0 }} WidthRequest={120} HeightRequest={70} HorizontalOptions="Center" VerticalOptions="Center" />
          </Demo>
        </SkiaWrap>
      </SkiaStack>
    </SkiaScroll>
  );
}
