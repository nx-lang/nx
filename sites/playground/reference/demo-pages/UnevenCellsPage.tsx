import { useCallback, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaLabel, SkiaLayer, SkiaScroll, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaLayout as SkiaLayoutCtrl, SkiaScroll as SkiaScrollCtrl } from "drawnui-react/core";
import { useCanvasView } from "./canvasView";
import { FeedCell, type FeedItem } from "./FeedCell";

const WORDS = "drawn ui renders every pixel itself skia canvas recycled cells measure visible estimates the rest and refines in idle time uneven rows news feed social timeline product catalog".split(" ");
const COLORS = ["#0D6EFD", "#6610F2", "#D63384", "#FD7E14", "#20C997", "#0DCAF0", "#FFC107"];
// deterministic pseudo-random bodies (1..6 lines at phone width); ids can go negative for prepended "history"
function makeItem(id: number): FeedItem {
  let seed = (id + 100_000) * 2654435761 >>> 0;
  const rnd = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };
  const words = 4 + Math.floor(rnd() * 56);
  const body = Array.from({ length: words }, () => WORDS[Math.floor(rnd() * WORDS.length)]).join(" ");
  return { Id: id, Title: `Post ${id}`, Body: body[0].toUpperCase() + body.slice(1) + ".", Color: COLORS[((id % COLORS.length) + COLORS.length) % COLORS.length] };
}
const range = (from: number, count: number) => Array.from({ length: count }, (_, i) => makeItem(from + i));
const INITIAL = range(1, 200);
const STATUS_HEIGHT = 36;

/** Uneven rows: MeasureItemsStrategy="MeasureVisible" — visible cells measured on demand, the rest estimated and measured in idle time. */
export function UnevenCellsPage() {
  const [items, setItems] = useState<FeedItem[]>(INITIAL);
  const [loading, setLoading] = useState("");
  const [debug, setDebug] = useState("");
  const scroll = useRef<SkiaScrollCtrl>(null);
  const feed = useRef<SkiaLayoutCtrl>(null);
  const view = useCanvasView();
  const template = useCallback(() => new FeedCell(), []);
  const jump = (index: number, option: "Start" | "End" = "Start") => scroll.current?.ScrollToIndex(index, true, option);
  const refresh = () => setDebug(`${feed.current?.DebugString ?? ""} · ${view?.FrameTime.toFixed(1) ?? "?"} ms · ${view?.FPS ?? "?"} fps`);
  // LoadMoreCommand: "fetch" the next page (append keeps every measured height); LoadMoreTopCommand: older history (prepend keeps the visible rows in place)
  const loadMore = () => {
    setLoading("loading next page…");
    setTimeout(() => setItems((cur) => { const last = cur[cur.length - 1].Id; return [...cur, ...range(last + 1, 100)]; }), 400);
    setTimeout(() => setLoading(""), 450);
  };
  const loadMoreTop = () => {
    setLoading("loading history…");
    setTimeout(() => setItems((cur) => { const first = cur[0].Id; return [...range(first - 30, 30), ...cur]; }), 400);
    setTimeout(() => setLoading(""), 450);
  };

  return (
    <SkiaLayer VerticalOptions="Fill">
      <SkiaLabel Text={`${items.length} uneven cells · MeasureVisible · LoadMore at both ends ${loading ? "· " + loading : ""}`} FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Center" Margin={new Thickness(0, 10, 0, 0)} />

      <SkiaScroll ref={scroll} Orientation="Vertical" Margin={new Thickness(0, STATUS_HEIGHT, 0, 0)} Scrolled={refresh} LoadMoreCommand={loadMore} LoadMoreOffset={300} LoadMoreTopCommand={loadMoreTop} LoadMoreTopOffset={100}>
        <SkiaStack
          ref={feed}
          ItemsSource={items}
          ItemTemplate={template}
          RecyclingTemplate="Enabled"
          MeasureItemsStrategy="MeasureVisible"
          Spacing={8}
          Padding={new Thickness(16, 8)}
        />
      </SkiaScroll>

      <SkiaWrap Spacing={6} Margin={new Thickness(8, 0, 8, 36)} HorizontalOptions="Center" VerticalOptions="End">
        <SkiaButton Text="HOME" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(0)} />
        <SkiaButton Text="MIDDLE" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(items.length / 2)} />
        <SkiaButton Text="END" FontSize={12} BackgroundColor="#0D6EFD" WidthRequest={104} Tapped={() => jump(items.length, "End")} />
        <SkiaButton Text="STATS" FontSize={12} BackgroundColor="#20C997" WidthRequest={104} Tapped={refresh} />
      </SkiaWrap>

      <SkiaLabel Text={debug} FontSize={11} TextColor="#00FF00" BackgroundColor="#DD000000" InputTransparent Margin={new Thickness(8, 4)} HorizontalOptions="Center" VerticalOptions="End" MaxLines={1} />
    </SkiaLayer>
  );
}
