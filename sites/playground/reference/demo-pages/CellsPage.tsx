import { useCallback, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaLabel, SkiaLayer, SkiaScroll, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaLayout as SkiaLayoutCtrl, SkiaScroll as SkiaScrollCtrl } from "drawnui-react/core";
import { useCanvasView } from "./canvasView";
import { ContactCell } from "./ContactCell";

// Huge data source, like the "Cells" fiddle: 100 000 items, only the visible cells exist.
const ITEMS = Array.from({ length: 100_000 }, (_, i) => i + 1);
const STATUS_HEIGHT = 36;

/** Recycled cells: the templated layout is the scroll's ONLY content, like DrawnUi. */
export function CellsPage() {
  const [lastTapped, setLastTapped] = useState(0);
  const [debug, setDebug] = useState("");
  const scroll = useRef<SkiaScrollCtrl>(null);
  const feed = useRef<SkiaLayoutCtrl>(null);
  const view = useCanvasView();
  // ItemTemplate must be a stable reference: a new function on every render would rebuild the whole cell pool.
  const template = useCallback(() => new ContactCell(setLastTapped), []);
  const jump = (index: number, option: "Start" | "End" = "Start") => scroll.current?.ScrollToIndex(index, true, option);

  return (
    <SkiaLayer VerticalOptions="Fill">
      <SkiaLabel Text={`100 000 recycled cells · last tapped: ${lastTapped || "-"}`} FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Center" Margin={new Thickness(0, 10, 0, 0)} />

      <SkiaScroll ref={scroll} Orientation="Vertical" Margin={new Thickness(0, STATUS_HEIGHT, 0, 0)} Scrolled={() => setDebug(`${feed.current?.DebugString ?? ""} · ${view?.FrameTime.toFixed(1) ?? "?"} ms · ${view?.FPS ?? "?"} fps`)}>
        <SkiaStack
          ref={feed}
          ItemsSource={ITEMS}
          ItemTemplate={template}
          RecyclingTemplate="Enabled"
          MeasureItemsStrategy="MeasureFirst"
          Spacing={8}
          Padding={new Thickness(16, 8)}
        />
      </SkiaScroll>

      {/* jump toolbar: wraps on narrow screens */}
      <SkiaWrap Spacing={6} Margin={new Thickness(8, 0, 8, 36)} HorizontalOptions="Center" VerticalOptions="End">
        <SkiaButton Text="HOME" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(0)} />
        <SkiaButton Text="BACKWARD" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump((feed.current?.FirstVisibleIndex ?? 0) - 5)} />
        <SkiaButton Text="MIDDLE" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(ITEMS.length / 2)} />
        <SkiaButton Text="FORWARD" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump((feed.current?.FirstVisibleIndex ?? 0) + 5)} />
        <SkiaButton Text="END" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(ITEMS.length, "End")} />
      </SkiaWrap>

      <SkiaLabel Text={debug} FontSize={11} TextColor="#00FF00" BackgroundColor="#DD000000" InputTransparent Margin={new Thickness(8, 4)} HorizontalOptions="Center" VerticalOptions="End" MaxLines={1} />
    </SkiaLayer>
  );
}
