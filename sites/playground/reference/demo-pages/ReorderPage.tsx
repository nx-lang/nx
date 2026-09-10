import { useCallback, useMemo, useRef, useState } from "react";
import { Colors, SkiaButton, SkiaLabel, SkiaLayer, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaLabel as SkiaLabelCtrl, SkiaLayout as SkiaLayoutCtrl, SkiaScroll as SkiaScrollCtrl, SkiaShape as SkiaShapeCtrl } from "drawnui-react/core";
import { ReorderCell, type DragHost, type ReorderItem } from "./ReorderCell";

const COLORS = ["#0D6EFD", "#6610F2", "#D63384", "#FD7E14", "#20C997", "#0DCAF0", "#FFC107"];
/** Android's language preferences, in their own name and locale tag. Latin, Cyrillic and Greek only: the demo
 *  registers OpenSans and nothing else, and a script it has no glyphs for would draw a row of tofu. */
const LANGUAGES: [string, string][] = [
  ["English (United States)", "en-US"], ["Español (España)", "es-ES"], ["Français (France)", "fr-FR"],
  ["Deutsch (Deutschland)", "de-DE"], ["Italiano (Italia)", "it-IT"], ["Português (Brasil)", "pt-BR"],
  ["Nederlands (Nederland)", "nl-NL"], ["Svenska (Sverige)", "sv-SE"], ["Norsk bokmål (Norge)", "nb-NO"],
  ["Dansk (Danmark)", "da-DK"], ["Suomi (Suomi)", "fi-FI"], ["Íslenska (Ísland)", "is-IS"],
  ["Polski (Polska)", "pl-PL"], ["Čeština (Česko)", "cs-CZ"], ["Slovenčina (Slovensko)", "sk-SK"],
  ["Magyar (Magyarország)", "hu-HU"], ["Română (România)", "ro-RO"], ["Hrvatski (Hrvatska)", "hr-HR"],
  ["Slovenščina (Slovenija)", "sl-SI"], ["Bosanski (Bosna i Hercegovina)", "bs-BA"], ["Shqip (Shqipëri)", "sq-AL"],
  ["Lietuvių (Lietuva)", "lt-LT"], ["Latviešu (Latvija)", "lv-LV"], ["Eesti (Eesti)", "et-EE"],
  ["Русский (Россия)", "ru-RU"],
  ["Українська (Україна)", "uk-UA"],
  ["Беларуская (Беларусь)", "be-BY"],
  ["Български (България)", "bg-BG"],
  ["Македонски (Македонија)", "mk-MK"],
  ["Српски (Србија)", "sr-RS"],
  ["Қазақша (Қазақстан)", "kk-KZ"],
  ["Кыргызча (Кыргызстан)", "ky-KG"],
  ["Монгол (Монгол)", "mn-MN"],
  ["Ελληνικά (Ελλάδα)", "el-GR"],
  ["Türkçe (Türkiye)", "tr-TR"], ["Azərbaycan (Azərbaycan)", "az-AZ"],
  ["Oʻzbekcha (Oʻzbekiston)", "uz-UZ"], ["Català (Espanya)", "ca-ES"], ["Galego (España)", "gl-ES"],
  ["Euskara (Espainia)", "eu-ES"], ["Gaeilge (Éire)", "ga-IE"], ["Gàidhlig (Alba)", "gd-GB"],
  ["Cymraeg (Cymru)", "cy-GB"], ["Malti (Malta)", "mt-MT"], ["Bahasa Indonesia (Indonesia)", "id-ID"],
  ["Bahasa Melayu (Malaysia)", "ms-MY"], ["Filipino (Pilipinas)", "fil-PH"], ["Tiếng Việt (Việt Nam)", "vi-VN"],
  ["Kiswahili (Kenya)", "sw-KE"], ["Afrikaans (Suid-Afrika)", "af-ZA"],
];
const INITIAL: ReorderItem[] = LANGUAGES.map(([Title, Tag], i) => ({ Id: i + 1, Title, Tag, Color: COLORS[i % COLORS.length] }));
const SPACING = 6;
const STATUS_HEIGHT = 58;
/** How long the released ghost takes to glide into its slot. */
const DROP_SECONDS = 0.14;
// hoisted: a new object per render would re-assign the prop and remeasure the ghost on every step of a drag
const GHOST_SHADOW = [{ Blur: 12, Opacity: 0.5, X: 0, Y: 6, Color: Colors.Black }];
const GRIP_MARGIN = new Thickness(10, 0, 0, 0);
const TITLE_MARGIN = new Thickness(46, 0, 60, 0);
const BADGE_MARGIN = new Thickness(0, 0, 14, 0);

/**
 * Drag to reorder: a templated list reordered in place while the row being dragged floats over it.
 *
 * The grabbed row is lifted into a ghost — a copy of the row that lives above the scroll and follows the pointer —
 * while the real row draws blank, so the gap that travels through the list is the slot the row will land in. Every
 * step writes a permuted array, which the layout recognises as a reorder of the same items: it rebinds the cells it
 * already has and moves their measured heights with them instead of rebuilding the structure, so the list does not
 * jump under the pointer and the scroll keeps its offset. On release the ghost glides into the slot and hands the
 * row back. Hold near the top or bottom edge and the list keeps moving, which is how a row reaches a position that
 * was off screen when the drag started.
 */
export function ReorderPage() {
  const [items, setItems] = useState<ReorderItem[]>(INITIAL);
  const [status, setStatus] = useState("drag a language by its grip, or use the buttons");
  const scroll = useRef<SkiaScrollCtrl>(null);
  const rows = useRef<SkiaLayoutCtrl>(null);
  const overlay = useRef<SkiaLayoutCtrl>(null);
  const ghost = useRef<SkiaShapeCtrl>(null);
  const ghostTitle = useRef<SkiaLabelCtrl>(null);
  const ghostBadge = useRef<SkiaLabelCtrl>(null);
  const live = useRef<ReorderItem[]>(items);
  const dragging = useRef<ReorderItem | undefined>(undefined);
  const drop = useRef({ raf: 0, at: 0, index: -1, from: 0, to: 0, progress: 0 });
  const grabOffset = useRef(0);
  live.current = items;

  const report = useCallback((what: string) => {
    const offset = scroll.current?.ViewportOffsetY ?? 0;
    setStatus(`${what} · offset ${offset.toFixed(0)} pt · ${live.current.slice(0, 5).map((i) => i.Tag).join(", ")}…`);
  }, []);

  const move = useCallback((from: number, to: number) => {
    const cur = live.current;
    if (to < 0 || to >= cur.length || from < 0 || from >= cur.length) return false;
    const next = cur.slice();
    next.splice(to, 0, next.splice(from, 1)[0]);
    live.current = next;
    setItems(next);
    return true;
  }, []);

  // one object for the whole page: cells are recycled, so the template must not rebuild the host per row
  const host = useMemo<DragHost>(() => {
    /** Where a row sits on screen right now, in canvas pixels, while that row is realized. */
    const rowRect = (index: number) => rows.current?.ChildrenFactory.GetExistingViewAtIndex(index)?.DrawingRect;

    /** The ghost is hidden between drags, so it is the overlay that always carries the current scale. */
    const scale = () => overlay.current?.RenderingScale || 1;

    /** Point coordinates inside the overlay, which is where the ghost is laid out. */
    const toGhostSpace = (pixels: number, axis: "Left" | "Top") => {
      const origin = overlay.current?.DrawingRect;
      return (pixels - (origin ? origin[axis] : 0)) / scale();
    };

    /** Rebinds nothing, just re-applies each row's look: used when the list did not change but a row's state did. */
    const refresh = () => { for (const view of rows.current?.Views ?? []) (view as ReorderCell).Refresh?.(); };

    const finish = () => {
      if (drop.current.raf) cancelAnimationFrame(drop.current.raf);
      drop.current.raf = 0;
      drop.current.index = -1;
      dragging.current = undefined;
      const g = ghost.current;
      if (g) { g.IsVisible = false; g.Update(); }
      refresh(); // the real row draws itself again
      report("dropped");
    };

    /** One frame of the settle, eased out so it lands rather than stops. The slot is re-read every frame because
     *  the list is still catching up with the last move when the pointer is released. */
    const pump = (now: number) => {
      const d = drop.current, g = ghost.current;
      if (!g) return finish();
      d.progress += Math.min(0.05, (now - d.at) / 1000) / DROP_SECONDS;
      d.at = now;
      if (d.progress >= 1) return finish();
      const rect = rowRect(d.index);
      if (rect) d.to = toGhostSpace(rect.Top, "Top");
      g.Top = d.from + (d.to - d.from) * (1 - Math.pow(1 - d.progress, 3));
      g.RepaintComposition();
      d.raf = requestAnimationFrame(pump);
    };

    return {
      Scroll: () => scroll.current ?? undefined,
      IndexOf: (item) => live.current.indexOf(item),
      Move: (from, to) => { const ok = move(from, to); if (ok) report("dragging"); return ok; },
      Dragging: () => dragging.current,
      Spacing: SPACING,

      Lift: (item, index, pointerY) => {
        const g = ghost.current, rect = rowRect(index);
        if (!g || !rect) return;
        if (drop.current.raf) { cancelAnimationFrame(drop.current.raf); drop.current.raf = 0; }
        grabOffset.current = (pointerY - rect.Top) / scale();
        dragging.current = item;
        if (ghostTitle.current) ghostTitle.current.Text = item.Title;
        if (ghostBadge.current) ghostBadge.current.Text = item.Tag;
        g.StrokeColor = item.Color;
        g.WidthRequest = rect.Width / scale();
        g.HeightRequest = rect.Height / scale();
        g.Left = toGhostSpace(rect.Left, "Left");
        g.Top = toGhostSpace(rect.Top, "Top");
        g.IsVisible = true;
        g.Update();
        refresh(); // blanks the row the ghost is now standing in for
        report("picked up");
      },

      Carry: (pointerY) => {
        const g = ghost.current;
        if (!g || !dragging.current || drop.current.raf) return;
        g.Top = toGhostSpace(pointerY, "Top") - grabOffset.current;
        g.RepaintComposition();
      },

      Drop: (index) => {
        const g = ghost.current;
        if (!g || !dragging.current) return finish();
        drop.current.index = index;
        drop.current.from = g.Top;
        drop.current.to = g.Top;
        drop.current.progress = 0;
        drop.current.at = performance.now();
        drop.current.raf = requestAnimationFrame(pump);
      },
    };
  }, [move, report]);

  const template = useCallback(() => new ReorderCell(host), [host]);

  return (
    <SkiaLayer VerticalOptions="Fill">
      <SkiaStack Spacing={2} Margin={new Thickness(0, 8, 0, 0)}>
        <SkiaLabel Text={`${items.length} languages in order of preference · drag by the grip, hold at an edge to keep going`} FontSize={13} TextColor={Colors.LightGray} HorizontalOptions="Center" />
        <SkiaLabel Text={status} FontSize={12} TextColor="#6EA8FE" HorizontalOptions="Center" />
      </SkiaStack>

      <SkiaScroll ref={scroll} Orientation="Vertical" Margin={new Thickness(0, STATUS_HEIGHT, 0, 44)} Scrolled={() => report("scrolled")}>
        <SkiaStack
          ref={rows}
          ItemsSource={items}
          ItemTemplate={template}
          RecyclingTemplate="Enabled"
          MeasureItemsStrategy="MeasureFirst"
          Spacing={SPACING}
          Padding={new Thickness(12, 8)}
        />
      </SkiaScroll>

      {/* The lifted row lives here, above the scroll that clips the list, and never eats the drag it is showing. */}
      <SkiaLayer ref={overlay} HorizontalOptions="Fill" VerticalOptions="Fill" InputTransparent>
        <SkiaShape
          ref={ghost}
          Type="Rectangle"
          CornerRadius={8}
          BackgroundColor="#1D4ED8"
          StrokeWidth={2}
          HorizontalOptions="Start"
          VerticalOptions="Start"
          IsVisible={false}
          Shadows={GHOST_SHADOW}
        >
          <SkiaLayer WidthRequest={26} HorizontalOptions="Start" VerticalOptions="Fill" Margin={GRIP_MARGIN}>
            <SkiaStack Spacing={3} VerticalOptions="Center" HorizontalOptions="Fill">
              {[0, 1, 2].map((i) => (
                <SkiaShape key={i} Type="Rectangle" CornerRadius={1} HeightRequest={2} WidthRequest={16} BackgroundColor="#BFDBFE" HorizontalOptions="Center" />
              ))}
            </SkiaStack>
          </SkiaLayer>
          <SkiaLabel ref={ghostTitle} FontSize={14} TextColor={Colors.White} VerticalOptions="Center" Margin={TITLE_MARGIN} />
          <SkiaLabel ref={ghostBadge} FontSize={12} TextColor="#BFDBFE" HorizontalOptions="End" VerticalOptions="Center" Margin={BADGE_MARGIN} />
        </SkiaShape>
      </SkiaLayer>

      <SkiaWrap Spacing={6} Margin={new Thickness(8, 0, 8, 8)} HorizontalOptions="Center" VerticalOptions="End">
        <SkiaButton Text="1st below 10th" FontSize={13} BackgroundColor="#495057" Tapped={() => { move(0, 9); report("moved 1st below 10th"); }} />
        <SkiaButton Text="Reverse" FontSize={13} BackgroundColor="#495057" Tapped={() => { live.current = live.current.slice().reverse(); setItems(live.current); report("reversed"); }} />
        <SkiaButton Text="Reset" FontSize={13} BackgroundColor="#495057" Tapped={() => { live.current = INITIAL; setItems(INITIAL); report("reset"); }} />
      </SkiaWrap>
    </SkiaLayer>
  );
}
