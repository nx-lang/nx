import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaDecoratedGrid, SkiaGrid, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import { SkiaDynamicDrawnCell, type SkiaLayout as SkiaLayoutCtrl, SkiaLabel as SkiaLabelCtrl, SkiaShape as SkiaShapeCtrl, Thickness as ThicknessCtrl } from "drawnui-react/core";

/** Recycled chip cell for the templated layouts below: visuals once, SetContent per bind. */
class ChipCell extends SkiaDynamicDrawnCell {
  private readonly shape = new SkiaShapeCtrl();
  private readonly label = new SkiaLabelCtrl();
  constructor() {
    super();
    this.shape.Type = "Rectangle"; this.shape.CornerRadius = 10; this.shape.HorizontalOptions = "Fill";
    this.label.FontSize = 13; this.label.TextColor = "#DEE2E6"; this.label.Padding = new ThicknessCtrl(12, 8); this.label.HorizontalOptions = "Center";
    this.shape.AddSubView(this.label);
    this.AddSubView(this.shape);
    this.HorizontalOptions = "Fill";
  }
  protected override SetContent(ctx: unknown): void {
    const item = ctx as { text: string; color: string };
    this.label.Text = item.text; this.shape.BackgroundColor = item.color;
  }
}
const PALETTE = ["#0F3460", "#533483", "#1B4332", "#7B2D26", "#495057", "#0D6EFD", "#D63384", "#2D6A4F"];

/** Text cell for the decorated grid: a title and a value, padded, no background — the grid lines separate the cells. */
class TextCell extends SkiaDynamicDrawnCell {
  private readonly title = new SkiaLabelCtrl();
  private readonly value = new SkiaLabelCtrl();
  constructor() {
    super();
    this.Type = "Column";
    this.Padding = new ThicknessCtrl(12, 10);
    this.HorizontalOptions = "Fill";
    this.title.FontSize = 11; this.title.TextColor = "#8B95A1"; this.title.TextTransform = "Uppercase";
    this.value.FontSize = 15; this.value.TextColor = "#DEE2E6"; this.value.FontFamily = "FontTextBold";
    this.AddSubView(this.title); this.AddSubView(this.value);
  }
  protected override SetContent(ctx: unknown): void {
    const item = ctx as { title: string; value: string };
    this.title.Text = item.title; this.value.Text = item.value;
  }
}
const FACTS = [
  { title: "Engine", value: "Skia" }, { title: "Language", value: "TypeScript" }, { title: "Layouts", value: "5 types" }, { title: "Cache", value: "4 kinds" },
  { title: "Gestures", value: "Unified" }, { title: "Shaders", value: "SkSL" }, { title: "Fonts", value: "Any TTF" }, { title: "Animations", value: "60 fps" },
  { title: "Navigation", value: "SkiaShell" }, { title: "Accessibility", value: "ARIA overlay" }, { title: "License", value: "MIT" }, { title: "Docs", value: "drawnui.net" },
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
  const [count, setCount] = useState(10);
  const [split, setSplit] = useState(3);
  const [dynamic, setDynamic] = useState(false);
  const items = useMemo(() => Array.from({ length: count }, (_, i) => ({ text: `Item ${i + 1}`, color: PALETTE[i % PALETTE.length] })), [count]);
  const template = useCallback(() => new ChipCell(), []);
  const textTemplate = useCallback(() => new TextCell(), []);
  // ImageComposite: one child spins, the layer re-records only it (+ what it overlaps)
  const composite = useRef<SkiaLayoutCtrl>(null);
  const spinner = useRef<SkiaShapeCtrl>(null);
  const [compositeInfo, setCompositeInfo] = useState("");
  const [redrawn, setRedrawn] = useState<number[]>([]); // indexes of the children the last composite record repainted
  // stable elements: a new Margin object on every render would remeasure every shape (a full composite record)
  const compositeShapes = useMemo(() => Array.from({ length: 24 }, (_, i) => (
    <SkiaShape key={i} Type={i % 3 === 0 ? "Circle" : "Rectangle"} CornerRadius={6} WidthRequest={40} HeightRequest={40} BackgroundColor={PALETTE[i % PALETTE.length]} Margin={new Thickness(12 + (i % 12) * 52, 12 + Math.floor(i / 12) * 70, 0, 0)} UseCache="Operations" />
  )), []);
  const spinnerMargin = useMemo(() => new Thickness(12 + 5 * 52 + 46 - 22, 12 + 40 + 15 - 22, 0, 0), []);
  const compositeMargins = useMemo(() => [...Array.from({ length: 24 }, (_, i) => new Thickness(12 + (i % 12) * 52, 12 + Math.floor(i / 12) * 70, 0, 0)), new Thickness(12 + 5 * 52 + 46 - 22, 12 + 40 + 15 - 22, 0, 0)], []);
  useEffect(() => {
    let angle = 0;
    const id = setInterval(() => {
      const sp = spinner.current, layer = composite.current;
      if (!sp || !layer) return;
      angle = (angle + 6) % 360;
      sp.Rotation = angle; sp.RepaintComposition();
      const rec = layer.LastCompositeRecord;
      setCompositeInfo(`last record: ${rec.Mode} · ${rec.Children} of ${layer.Children.length} children redrawn`);
      setRedrawn(rec.Dirty.map((c) => layer.Children.indexOf(c)).filter((i) => i >= 0));
    }, 40);
    return () => clearInterval(id);
  }, []);
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
        <Card title="ZIndex draws later (on top); HorizontalFillRatio/VerticalFillRatio = fraction of the box; Left/Top nudge the drawn output">
          <SkiaLayer HeightRequest={120} BackgroundColor="#212529" HorizontalOptions="Fill">
            <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#0D6EFD" HorizontalOptions="Fill" VerticalOptions="Fill" HorizontalFillRatio={0.5} VerticalFillRatio={0.75} ZIndex={2}>
              <SkiaLabel Text="ZIndex=2 · FillRatio 0.5 × 0.75" FontSize={12} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaShape>
            <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#D63384" WidthRequest={220} HeightRequest={70} Margin={new Thickness(120, 30, 0, 0)} ZIndex={1}>
              <SkiaLabel Text="ZIndex=1, declared second" FontSize={12} TextColor={Colors.White} HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaShape>
            <SkiaShape Type="Rectangle" CornerRadius={8} BackgroundColor="#20C997" WidthRequest={160} HeightRequest={50} HorizontalOptions="End" VerticalOptions="End" Left={-20} Top={-10} ZIndex={3}>
              <SkiaLabel Text="Left=-20 Top=-10 · ZIndex=3" FontSize={11} TextColor="#1A1A2E" HorizontalOptions="Center" VerticalOptions="Center" />
            </SkiaShape>
          </SkiaLayer>
        </Card>

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
            <SkiaLabel Text="DrawnUI" FontSize={14} TextColor={Colors.White} Column={1} Row={0} />
            <SkiaLabel Text="Renderer" FontSize={14} TextColor="#ADB5BD" Column={0} Row={1} />
            <SkiaLabel Text="CanvasKit (Skia WASM)" FontSize={14} TextColor={Colors.White} Column={1} Row={1} />
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

        <SkiaLabel Text="Caching · UseCache=ImageComposite" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title={`SkiaLayer UseCache="ImageComposite" · 24 shapes + 1 rotating · ${compositeInfo || "…"}`}>
          <SkiaLayer HeightRequest={150} HorizontalOptions="Fill">
            <SkiaLayer ref={composite} UseCache="ImageComposite" HeightRequest={150} HorizontalOptions="Fill" BackgroundColor="#212529">
              {compositeShapes}
              <SkiaShape ref={spinner} Type="Rectangle" CornerRadius={4} WidthRequest={44} HeightRequest={44} BackgroundColor="#FFC107" Margin={spinnerMargin} UseCache="Operations" ZIndex={5} />
            </SkiaLayer>
            {/* overlay OUTSIDE the composite: white outline = repainted by the last record, everything else was kept as is */}
            <SkiaLayer HeightRequest={150} HorizontalOptions="Fill" InputTransparent>
              {redrawn.map((i) => <SkiaShape key={i} Type="Rectangle" CornerRadius={i === 24 ? 4 : 6} WidthRequest={i === 24 ? 44 : 40} HeightRequest={i === 24 ? 44 : 40} StrokeColor="#FFFFFF" StrokeWidth={2} Margin={compositeMargins[i]} />)}
            </SkiaLayer>
          </SkiaLayer>
          <SkiaLabel Text="White outlines = the children the last record repainted (the rotating one + every sibling its old and new bounds overlap); the others are kept in the cache surface untouched. RepaintComposition() from the spinning child marks it dirty in the composite parent (C# DirtyChildrenTracker); own content / measure changes record fully." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>
        <SkiaLabel Text="ItemsSource + ItemTemplate for Wrap / Row / Grid · Split" FontSize={20} TextColor={Colors.White} HorizontalOptions="Center" Margin={new Thickness(0, 8, 0, 0)} />
        <Card title={`SkiaWrap ItemsSource (${count} recycled ChipCell) · Split=${split} · DynamicColumns=${dynamic}`}>
          <SkiaWrap Spacing={8} ItemsSource={items} ItemTemplate={template} Split={split} DynamicColumns={dynamic} />
          <SkiaWrap Spacing={6}>
            {[0, 2, 3, 4].map((v) => <SkiaButton key={v} Text={v === 0 ? "Split 0 (flow)" : `Split ${v}`} BackgroundColor={split === v ? "#533483" : "#495057"} FontSize={12} Tapped={() => setSplit(v)} />)}
            <SkiaButton Text={`DynamicColumns ${dynamic ? "on" : "off"}`} BackgroundColor={dynamic ? "#533483" : "#495057"} FontSize={12} Tapped={() => setDynamic((d) => !d)} />
            <SkiaButton Text="+ item" BackgroundColor="#0D6EFD" FontSize={12} Tapped={() => setCount((c) => c + 1)} />
            <SkiaButton Text="- item" BackgroundColor="#0D6EFD" FontSize={12} Tapped={() => setCount((c) => Math.max(1, c - 1))} />
          </SkiaWrap>
        </Card>
        <Card title="SkiaRow ItemsSource (same cells, laid out horizontally, every item realized)">
          <SkiaRow Spacing={8} ItemsSource={items.slice(0, 5)} ItemTemplate={template} />
        </Card>
        <Card title={`SkiaDecoratedGrid ItemsSource · Split=4 · ColumnSpacing / RowSpacing 1 · gradient lines in the spacing`}>
          <SkiaDecoratedGrid ItemsSource={FACTS} ItemTemplate={textTemplate} Split={4} ColumnDefinitions="*,*,*,*" ColumnSpacing={1} RowSpacing={1} BackgroundColor="#212529" />
        </Card>
        <Card title="SkiaGrid ItemsSource · Split=3 · Invert (column-major)">
          <SkiaGrid ItemsSource={items} ItemTemplate={template} Split={3} Invert ColumnDefinitions="*,*,*" ColumnSpacing={8} RowSpacing={8} />
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
