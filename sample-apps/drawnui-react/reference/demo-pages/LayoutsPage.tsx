import { Colors, SkiaGrid, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";

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

/** A colored cell with a centered caption, sized by its grid cell. */
function Cell({ text, color, ...place }: { text: string; color: string; Column?: number; Row?: number; ColumnSpan?: number; RowSpan?: number }) {
  return (
    <SkiaShape Type="Rectangle" CornerRadius={6} BackgroundColor={color} HorizontalOptions="Fill" VerticalOptions="Fill" {...place}>
      <SkiaLabel Text={text} FontSize={13} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" Padding={new Thickness(8, 6)} />
    </SkiaShape>
  );
}

/** A small labelled box used by the stack/row/layer demos. */
function Box({ text, color, w, h, ...rest }: { text: string; color: string; w?: number; h?: number; HorizontalOptions?: "Start" | "Center" | "End" | "Fill"; VerticalOptions?: "Start" | "Center" | "End" | "Fill"; Margin?: Thickness }) {
  return (
    <SkiaShape Type="Rectangle" CornerRadius={6} BackgroundColor={color} WidthRequest={w ?? -1} HeightRequest={h ?? -1} {...rest}>
      <SkiaLabel Text={text} FontSize={12} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" Padding={new Thickness(10, 6)} />
    </SkiaShape>
  );
}

/** Every SkiaLayout type: Absolute (SkiaLayer), Column (SkiaStack), Row (SkiaRow), Wrap (SkiaWrap), Grid (SkiaGrid). */
export function LayoutsPage() {
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="SkiaLayout types" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaLabel Text="One class, five Type values; the aliases preset Type (+ HorizontalOptions=Fill for SkiaStack / SkiaLayer / SkiaWrap / SkiaGrid). Children position themselves with HorizontalOptions / VerticalOptions / Margin, the container adds Spacing and Padding." FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Fill" HorizontalTextAlignment="Center" />

        <SkiaLabel Text="Absolute · SkiaLayer" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title="Children overlap in one cell; alignment + Margin place them (WPF-style, no X/Y)">
          <SkiaLayer HeightRequest={150} BackgroundColor="#1F2937">
            <Box text="Start/Start" color="#0D6EFD" />
            <Box text="Center/Start" color="#6610F2" HorizontalOptions="Center" />
            <Box text="End/Start" color="#D63384" HorizontalOptions="End" />
            <Box text="Start/Center" color="#FD7E14" VerticalOptions="Center" />
            <Box text="Center/Center" color="#20C997" HorizontalOptions="Center" VerticalOptions="Center" />
            <Box text="End/Center" color="#0DCAF0" HorizontalOptions="End" VerticalOptions="Center" />
            <Box text="Start/End" color="#6EA8FE" VerticalOptions="End" />
            <Box text="Margin(0,0,0,12)" color="#FFC107" HorizontalOptions="Center" VerticalOptions="End" Margin={new Thickness(0, 0, 0, 12)} />
            <Box text="End/End" color="#DC3545" HorizontalOptions="End" VerticalOptions="End" />
          </SkiaLayer>
        </Card>
        <Card title="Icon + text with an Absolute layer instead of a grid (cheaper): label gets the icon's width as Margin">
          <SkiaLayer>
            <SkiaShape Type="Circle" BackgroundColor="#6EA8FE" WidthRequest={36} LockRatio={1} />
            <SkiaLabel Text="Margin={new Thickness(48, 0, 0, 0)} VerticalOptions=Center — no second column to measure." FontSize={14} TextColor="#DEE2E6" Margin={new Thickness(48, 0, 0, 0)} VerticalOptions="Center" HorizontalOptions="Fill" />
          </SkiaLayer>
        </Card>

        <Card title="IsClippedToBounds — a child larger than its parent">
          <SkiaRow Spacing={24}>
            <SkiaLayer WidthRequest={140} HeightRequest={70} BackgroundColor="#1F2937">
              <SkiaShape Type="Circle" BackgroundColor="#D63384" WidthRequest={110} LockRatio={1} HorizontalOptions="End" VerticalOptions="End" Margin={new Thickness(0, 0, -30, -30)} />
              <SkiaLabel Text="overflows (default)" FontSize={11} TextColor={Colors.White} Padding={new Thickness(6)} />
            </SkiaLayer>
            <SkiaLayer WidthRequest={140} HeightRequest={70} BackgroundColor="#1F2937" IsClippedToBounds>
              <SkiaShape Type="Circle" BackgroundColor="#D63384" WidthRequest={110} LockRatio={1} HorizontalOptions="End" VerticalOptions="End" Margin={new Thickness(0, 0, -30, -30)} />
              <SkiaLabel Text="IsClippedToBounds" FontSize={11} TextColor={Colors.White} Padding={new Thickness(6)} />
            </SkiaLayer>
            <SkiaLayer WidthRequest={140} HeightRequest={70} BackgroundColor="#1F2937" IsClippedToBounds ClipEffects={false}>
              <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#20C997" WidthRequest={100} HeightRequest={40} HorizontalOptions="Center" VerticalOptions="Center" Shadows={[{ X: 0, Y: 0, Blur: 12, Opacity: 1, Color: "#20C997" }]} />
              <SkiaLabel Text="ClipEffects={false}" FontSize={11} TextColor={Colors.White} Padding={new Thickness(6)} />
            </SkiaLayer>
          </SkiaRow>
        </Card>

        <SkiaLabel Text="Column · SkiaStack" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title="Vertical stack, Spacing between children, each child aligns horizontally on its own">
          <SkiaStack Spacing={6} BackgroundColor="#1F2937" Padding={new Thickness(8)}>
            <Box text="Start (default)" color="#0D6EFD" />
            <Box text="HorizontalOptions=Center" color="#6610F2" HorizontalOptions="Center" />
            <Box text="HorizontalOptions=End" color="#D63384" HorizontalOptions="End" />
            <Box text="HorizontalOptions=Fill" color="#20C997" HorizontalOptions="Fill" />
            <Box text="WidthRequest={200}" color="#FD7E14" w={200} />
          </SkiaStack>
        </Card>

        <SkiaLabel Text="Row · SkiaRow" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title="Horizontal stack; the row is as tall as its tallest child, children align vertically">
          <SkiaRow Spacing={8} BackgroundColor="#1F2937" Padding={new Thickness(8)} HorizontalOptions="Fill">
            <Box text="Start" color="#0D6EFD" h={70} />
            <Box text="Center" color="#6610F2" VerticalOptions="Center" />
            <Box text="End" color="#D63384" VerticalOptions="End" />
            <Box text="Fill" color="#20C997" VerticalOptions="Fill" />
            <Box text="Margin(16,0,0,0)" color="#FD7E14" VerticalOptions="Center" Margin={new Thickness(16, 0, 0, 0)} />
          </SkiaRow>
          <SkiaLabel Text="A Row gives children an infinite width: Fill on the main axis auto-sizes (MAUI stack semantics). Use SkiaGrid with a * column when something must take the remaining width." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <SkiaLabel Text="Grid · SkiaGrid" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />

        <Card title='ColumnDefinitions="*, 2*, Auto" RowDefinitions="Auto, 60" · ColumnSpacing/RowSpacing 8'>
          <SkiaGrid ColumnDefinitions="*, 2*, Auto" RowDefinitions="Auto, 60" ColumnSpacing={8} RowSpacing={8}>
            <Cell text="*" color="#0D6EFD" Column={0} Row={0} />
            <Cell text="2*" color="#6610F2" Column={1} Row={0} />
            <Cell text="Auto (this label)" color="#D63384" Column={2} Row={0} />
            <Cell text="Row 1 = 60pt" color="#20C997" Column={0} Row={1} />
            <Cell text="Column 1" color="#FD7E14" Column={1} Row={1} />
            <Cell text="Auto" color="#DC3545" Column={2} Row={1} />
          </SkiaGrid>
        </Card>

        <Card title="ColumnSpan / RowSpan">
          <SkiaGrid ColumnDefinitions="*, *, *" RowDefinitions="48, 48, 48" ColumnSpacing={6} RowSpacing={6}>
            <Cell text="ColumnSpan=2" color="#0D6EFD" Column={0} Row={0} ColumnSpan={2} />
            <Cell text="RowSpan=2" color="#6610F2" Column={2} Row={0} RowSpan={2} />
            <Cell text="0,1" color="#20C997" Column={0} Row={1} />
            <Cell text="1,1" color="#FD7E14" Column={1} Row={1} />
            <Cell text="ColumnSpan=3" color="#D63384" Column={0} Row={2} ColumnSpan={3} />
          </SkiaGrid>
        </Card>

        <Card title="Implicit tracks: no definitions, children reference Column/Row (DefaultColumnDefinition = Auto)">
          <SkiaGrid ColumnSpacing={12} RowSpacing={4} HorizontalOptions="Start">
            <SkiaLabel Text="Name" FontSize={14} TextColor="#ADB5BD" Column={0} Row={0} />
            <SkiaLabel Text="DrawnUI for React" FontSize={14} TextColor={Colors.White} Column={1} Row={0} />
            <SkiaLabel Text="Renderer" FontSize={14} TextColor="#ADB5BD" Column={0} Row={1} />
            <SkiaLabel Text="CanvasKit (Skia WASM) + react-reconciler" FontSize={14} TextColor={Colors.White} Column={1} Row={1} />
            <SkiaLabel Text="License" FontSize={14} TextColor="#ADB5BD" Column={0} Row={2} />
            <SkiaLabel Text="MIT" FontSize={14} TextColor={Colors.White} Column={1} Row={2} />
          </SkiaGrid>
        </Card>

        <Card title="Icon + text pattern: Auto column for the icon, * for wrapping text">
          <SkiaGrid ColumnDefinitions="Auto, *" ColumnSpacing={12}>
            <SkiaShape Type="Circle" BackgroundColor="#6EA8FE" WidthRequest={40} LockRatio={1} VerticalOptions="Start" Column={0} />
            <SkiaLabel Column={1} FontSize={14} TextColor="#DEE2E6" HorizontalOptions="Fill" Text="The star column takes whatever the Auto column leaves, and this label wraps inside it. The row is Auto, so it grows with the text — the same layout a MAUI Grid would produce." />
          </SkiaGrid>
        </Card>

        <SkiaLabel Text="Wrap · SkiaWrap" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title="Type=Wrap · Spacing 8 · resize the window">
          <SkiaWrap Spacing={8}>
            {["Absolute", "Column", "Row", "Wrap", "Grid", "SkiaStack", "SkiaRow", "SkiaLayer", "SkiaWrap", "SkiaGrid", "Spacing", "Padding", "Margin"].map((t) => (
              <SkiaShape key={t} Type="Rectangle" CornerRadius={14} BackgroundColor="#373B3E">
                <SkiaLabel Text={t} FontSize={13} TextColor="#DEE2E6" Padding={new Thickness(12, 6)} />
              </SkiaShape>
            ))}
          </SkiaWrap>
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
