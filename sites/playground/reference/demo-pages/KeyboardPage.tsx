import { useEffect, useState } from "react";
import { Colors, KeyboardManager, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, Thickness } from "drawnui-react";

const WAITING = "Waiting for input";

/** Port of the Blazor sandbox KeyboardProbe page: KeyboardManager.KeyDown / KeyUp with modifier state and a history. */
export function KeyboardPage() {
  const [hero, setHero] = useState("Keyboard input ready");
  const [last, setLast] = useState("Last key: waiting");
  const [modifiers, setModifiers] = useState("Modifiers: shift false, ctrl false, alt false");
  const [history, setHistory] = useState<string[]>([WAITING, WAITING, WAITING, WAITING, WAITING]);
  const [chars, setChars] = useState("");

  useEffect(() => {
    const apply = (phase: string, key: string) => {
      setHero("Keyboard probe live");
      setLast(`Last key: ${phase} ${key || "Unknown"}`);
      setModifiers(`Modifiers: shift ${KeyboardManager.IsShiftPressed}, ctrl ${KeyboardManager.IsControlPressed}, alt ${KeyboardManager.IsAltPressed}`);
      setHistory((h) => [`${phase} ${key || "Unknown"}`, ...h].slice(0, 5));
    };
    const down = (key: string) => apply("down", key);
    const up = (key: string) => apply("up", key);
    const char = (ch: string) => setChars((c) => (c + ch).slice(-40));
    KeyboardManager.Subscribe(down, char, up);
    return () => KeyboardManager.Unsubscribe(down, char, up);
  }, []);

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Keyboard Input" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />
        <SkiaLabel Text="KeyboardManager attaches window-level keydown / keyup listeners like the DrawnUi.Blazor and Wasm heads (drawnui-keyboard.js): shortcuts, game input, drawn editors. Click the page, then press letters, arrows, modifiers or function keys." FontSize={13} TextColor="#ADB5BD" HorizontalOptions="Fill" HorizontalTextAlignment="Center" />

        {/* the Blazor probe canvas: cream card, blue banner, last key, modifiers, recent events */}
        <SkiaStack Spacing={14} Padding={new Thickness(20)} BackgroundColor="#FEFDF6" HorizontalOptions="Fill">
          <SkiaLabel Text={hero} FontSize={28} FontFamily="FontTextBold" TextColor="#252B37" HorizontalOptions="Fill" />
          <SkiaShape Type="Rectangle" CornerRadius={18} BackgroundColor="#3C639F" HeightRequest={92} HorizontalOptions="Fill" Padding={new Thickness(16)}>
            <SkiaLabel Text="Press letters, arrows, modifiers, or function keys" FontSize={18} TextColor={Colors.White} HorizontalOptions="Fill" VerticalOptions="Center" />
          </SkiaShape>
          <SkiaLabel Text={last} FontSize={16} TextColor="#41495A" HorizontalOptions="Fill" />
          <SkiaLabel Text={modifiers} FontSize={14} TextColor="#636F80" HorizontalOptions="Fill" />
          <SkiaLabel Text={`KeyChar (printable, no Ctrl/Alt): "${chars}"`} FontSize={14} TextColor="#636F80" HorizontalOptions="Fill" />
          <SkiaStack Spacing={8} Padding={new Thickness(14)} BackgroundColor="#F2EDE0" HorizontalOptions="Fill">
            <SkiaLabel Text="Recent events" FontSize={18} FontFamily="FontTextBold" TextColor="#47321C" HorizontalOptions="Fill" />
            {history.map((h, i) => <SkiaLabel key={i} Text={h} FontSize={14} TextColor="#5C4A35" HorizontalOptions="Fill" />)}
          </SkiaStack>
        </SkiaStack>
      </SkiaStack>
    </SkiaScroll>
  );
}
