import { useEffect, useRef } from "react";
import { SkiaLabel, SkiaLayer, Thickness } from "drawnui-react";
import type { SkiaLayout as SkiaLayoutCtrl } from "drawnui-react/core";
import { PongGame } from "./pong/PongGame";
import { RescalingLayout } from "./pong/RescalingLayout";

const HINT_MARGIN = new Thickness(12, 8, 12, 0);
const FIELD_MARGIN = new Thickness(0, 36, 0, 0);

/**
 * Pong, the DrawnUi Pong.Shared sample (MAUI, WPF, OpenTK, Blazor and pure-WASM heads) on this engine: a DrawnGame
 * subclass with a game loop, sprites moved by Left / Top, an AI paddle, keyboard and touch input. The 360x640 field
 * is fitted to the page by a RescalingLayout (rendering scale, not a transform).
 */
export function PongPage() {
  const host = useRef<SkiaLayoutCtrl>(null);

  // the game is code-behind (like WarriorSprite): built once, mounted into the host, disposed on unmount
  useEffect(() => {
    const layer = host.current;
    if (!layer) return;
    const field = new RescalingLayout();
    field.LogicalWidth = PongGame.WIDTH;
    field.LogicalHeight = PongGame.HEIGHT;
    const game = new PongGame();
    game.HorizontalOptions = "Center";
    game.VerticalOptions = "Center";
    field.AddSubView(game);
    layer.AddSubView(field);
    // a hidden tab gets no frames: pause, and Resume() restarts the frame clock, else the first frame back
    // carries the whole hidden time as its delta (what MAUI apps do from OnSleep / OnResume)
    const onVisibility = () => { if (document.hidden) game.Pause(); else game.Resume(); };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      layer.RemoveSubView(field);
      field.Dispose();
    };
  }, []);

  return (
    <SkiaLayer BackgroundColor="#0A0F1E" VerticalOptions="Fill">
      <SkiaLabel Text="← → or drag to move, tap / Space to serve · first to 7" FontFamilyFallback="FontSymbols,FontSymbols2" FontSize={13} TextColor="#ADB5BD" HorizontalOptions="Center" Margin={HINT_MARGIN} />
      <SkiaLayer ref={host} VerticalOptions="Fill" Margin={FIELD_MARGIN} />
    </SkiaLayer>
  );
}
