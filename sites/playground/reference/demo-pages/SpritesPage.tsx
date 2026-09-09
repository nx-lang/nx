import { useEffect, useRef, useState } from "react";
import { Colors, Easing, KeyboardManager, SkiaButton, SkiaLabel, SkiaLayer, SkiaRow, SkiaScroll, SkiaShape, SkiaSprite, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaLayout as SkiaLayoutCtrl, SkiaSprite as SkiaSpriteCtrl } from "drawnui-react/core";
import { WarriorSprite } from "./WarriorSprite";

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

const TILE = 64, COLS = 9, ROWS = 4;

/** SkiaSprite (FastRepro SpriteTestPage) and a SkiaSpriteSet warrior on a tile board moved with the keyboard (SpriteSnappedBoardPage). */
export function SpritesPage() {
  const sprite = useRef<SkiaSpriteCtrl>(null);
  const [info, setInfo] = useState("loading…");
  const [fps, setFps] = useState(15);
  const [playing, setPlaying] = useState(true);
  const board = useRef<SkiaLayoutCtrl>(null);
  const [pos, setPos] = useState({ col: 1, row: 1 });
  const [state, setState] = useState("IdleRight");
  const player = useRef<WarriorSprite | null>(null);
  const moving = useRef(false);
  const facing = useRef<"Right" | "Left">("Right");

  // the warrior is a code-behind SkiaSpriteSet subclass: mounted into an empty host layer, disposed on unmount
  useEffect(() => {
    const host = board.current;
    if (!host) return;
    const p = new WarriorSprite("Blue");
    p.WidthRequest = TILE; p.HeightRequest = TILE; p.ZIndex = 10;
    p.TranslationX = 1 * TILE; p.TranslationY = 1 * TILE;
    host.AddSubView(p);
    player.current = p;
    return () => { host.RemoveSubView(p); p.Dispose(); player.current = null; };
  }, []);

  const move = async (dx: number, dy: number) => {
    const p = player.current;
    if (!p || moving.current) return;
    const target = { col: Math.max(0, Math.min(COLS - 1, pos.col + dx)), row: Math.max(0, Math.min(ROWS - 1, pos.row + dy)) };
    if (dx !== 0) facing.current = dx > 0 ? "Right" : "Left";
    if (target.col === pos.col && target.row === pos.row) { p.WState = `Idle${facing.current}`; setState(p.WState); return; }
    moving.current = true;
    p.WState = `Walk${facing.current}`; setState(p.WState);
    await p.TranslateToAsync(target.col * TILE, target.row * TILE, 220, Easing.Linear);
    setPos(target);
    moving.current = false;
    p.WState = `Idle${facing.current}`; setState(p.WState);
  };
  const attack = () => {
    const p = player.current;
    if (!p || moving.current) return;
    p.WState = `War${facing.current}`; setState(p.WState);
    setTimeout(() => { if (player.current) { player.current.WState = `Idle${facing.current}`; setState(player.current.WState); } }, 500);
  };
  const moveRef = useRef(move); moveRef.current = move;
  const attackRef = useRef(attack); attackRef.current = attack;

  useEffect(() => {
    const down = (key: string, e: KeyboardEvent) => {
      const map: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1], KeyA: [-1, 0], KeyD: [1, 0], KeyW: [0, -1], KeyS: [0, 1] };
      if (map[key]) { e.preventDefault(); void moveRef.current(...map[key]); }
      else if (key === "Space") { e.preventDefault(); attackRef.current(); }
    };
    const char = () => {};
    KeyboardManager.Subscribe(down, char);
    return () => KeyboardManager.Unsubscribe(down, char);
  }, []);

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Sprites" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title={`SkiaSprite — Source="anims/BlueWarrior/Warrior_Idle.png" Columns={8} Rows={1} · ${info}`}>
          <SkiaRow Spacing={16}>
            <SkiaSprite ref={sprite} Source="anims/BlueWarrior/Warrior_Idle.png" Columns={8} Rows={1} FramesPerSecond={fps} Repeat={-1} WidthRequest={160} HeightRequest={160} BackgroundColor="#212529" UseCache="Image"
              Success={(s) => setInfo(`${s.TotalFrames} frames · ${s.FrameWidth}×${s.FrameHeight} px · ${Math.round(s.DurationMs)} ms`)} Error={(_, e) => setInfo(`error: ${e.message}`)} />
            <SkiaSprite Source="anims/RedWarrior/Warrior_Attack1.png" Columns={4} Rows={1} FramesPerSecond={8} Repeat={-1} WidthRequest={160} HeightRequest={160} BackgroundColor="#212529" UseCache="Image" />
            <SkiaSprite Source="anims/Trees/Tree1.png" Columns={8} Rows={1} FramesPerSecond={6} Repeat={-1} WidthRequest={160} HeightRequest={160} BackgroundColor="#212529" UseCache="Image" />
            <SkiaStack Spacing={8} VerticalOptions="Center" HorizontalOptions="Fill">
              <SkiaWrap Spacing={8}>
                <SkiaButton Text={playing ? "Pause" : "Play"} BackgroundColor="#0D6EFD" FontSize={13} Tapped={() => { const s = sprite.current; if (!s) return; if (s.IsPlaying) { s.Stop(); setPlaying(false); } else { s.Start(); setPlaying(true); } }} />
                {[5, 15, 30].map((v) => <SkiaButton key={v} Text={`${v} fps`} BackgroundColor={fps === v ? "#533483" : "#495057"} FontSize={13} Tapped={() => setFps(v)} />)}
                <SkiaButton Text="Seek(0)" BackgroundColor="#495057" FontSize={13} Tapped={() => { sprite.current?.Stop(); sprite.current?.Seek(0); setPlaying(false); }} />
              </SkiaWrap>
              <SkiaLabel Text="Frames are cut from the sheet by Columns × Rows, transparent borders trimmed per frame (C# SpriteFrameImage), nearest sampling; the animator runs 0..DurationMs and picks the frame by time." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
            </SkiaStack>
          </SkiaRow>
        </Card>

        <Card title={`SkiaSpriteSet warrior on a tile board — arrows / WASD move, Space attacks · tile ${pos.col},${pos.row} · ${state}`}>
          <SkiaLayer WidthRequest={COLS * TILE} HeightRequest={ROWS * TILE} BackgroundColor="#1B4332" HorizontalOptions="Center" IsClippedToBounds>
            {Array.from({ length: COLS * ROWS }, (_, i) => (
              <SkiaShape key={i} Type="Rectangle" WidthRequest={TILE} HeightRequest={TILE} BackgroundColor={((i % COLS) + Math.floor(i / COLS)) % 2 === 0 ? "#2D6A4F" : "#40916C"} Margin={new Thickness((i % COLS) * TILE, Math.floor(i / COLS) * TILE, 0, 0)} />
            ))}
            <SkiaSprite Source="anims/Trees/Tree1.png" Columns={8} Rows={1} FramesPerSecond={6} Repeat={-1} WidthRequest={TILE} HeightRequest={TILE} Margin={new Thickness(5 * TILE, 0, 0, 0)} UseCache="Image" />
            <SkiaSprite Source="anims/Trees/Tree2.png" Columns={8} Rows={1} FramesPerSecond={5} Repeat={-1} WidthRequest={TILE} HeightRequest={TILE} Margin={new Thickness(7 * TILE, 2 * TILE, 0, 0)} UseCache="Image" />
            <SkiaSprite Source="anims/RedWarrior/Warrior_Idle.png" Columns={8} Rows={1} FramesPerSecond={15} Repeat={-1} WidthRequest={TILE} HeightRequest={TILE} Margin={new Thickness(6 * TILE, 3 * TILE, 0, 0)} ScaleX={-1} UseCache="Image" ZIndex={9} />
            {/* the player (WarriorSprite) is added to this layer from code */}
            <SkiaLayer ref={board} WidthRequest={COLS * TILE} HeightRequest={ROWS * TILE} />
          </SkiaLayer>
          <SkiaWrap Spacing={8} HorizontalOptions="Center">
            <SkiaButton Text="← Left" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => void move(-1, 0)} />
            <SkiaButton Text="↑ Up" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => void move(0, -1)} />
            <SkiaButton Text="↓ Down" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => void move(0, 1)} />
            <SkiaButton Text="Right →" FontFamilyFallback="FontSymbols,FontSymbols2" BackgroundColor="#0F3460" FontSize={13} Tapped={() => void move(1, 0)} />
            <SkiaButton Text="Attack (Space)" BackgroundColor="#D63384" FontSize={13} Tapped={attack} />
          </SkiaWrap>
          <SkiaLabel Text="WarriorSprite extends SkiaSpriteSet: Define(0 idle, 1 run, 2 attack) with the FastRepro sheets; WState maps to State and mirrors CurrentSprite.ScaleX; the move is a TranslateToAsync to the target tile while the walk state plays." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
