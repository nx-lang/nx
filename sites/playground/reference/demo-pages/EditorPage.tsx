import { useRef, useState } from "react";
import { Colors, SkiaButton, SkiaEditor, SkiaLabel, SkiaScroll, SkiaShape, SkiaStack, SkiaWrap, Thickness } from "drawnui-react";
import type { SkiaEditor as SkiaEditorCtrl } from "drawnui-react/core";

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

/** SkiaEditor: drawn text input with caret, selection, placeholder, password, multiline and the platform looks. */
export function EditorPage() {
  const editor = useRef<SkiaEditorCtrl>(null);
  const [text, setText] = useState("");
  const [state, setState] = useState("");
  const [submitted, setSubmitted] = useState("");
  const [focused, setFocused] = useState(false);
  const [chat, setChat] = useState<string[]>([]);
  const [password, setPassword] = useState("");
  const describe = (e: SkiaEditorCtrl) => setState(`cursor ${e.CursorPosition} · selection ${e.SelectionLength} · ${e.Text.length} chars`);

  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16, 16, 16, 120)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="SkiaEditor" FontSize={24} TextColor={Colors.White} HorizontalOptions="Center" />

        <Card title={`Single line — Text="${text}" · IsFocused=${focused} · ${state} · submitted: "${submitted}"`}>
          <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" ref={editor} PlaceholderText="Type here, Enter submits" FontSize={16} TextChanged={(e, t) => { setText(t); describe(e); }} CursorMoved={describe}
            FocusChanged={(_, f) => setFocused(f)} TextSubmitted={(_, t) => setSubmitted(t)} />
          <SkiaWrap Spacing={8}>
            <SkiaButton Text="Focus" BackgroundColor="#0D6EFD" FontSize={13} Tapped={() => { if (editor.current) editor.current.IsFocused = true; }} />
            <SkiaButton Text="SelectAll()" BackgroundColor="#495057" FontSize={13} Tapped={() => editor.current?.SelectAll()} />
            <SkiaButton Text="InsertAtCursor('🙂')" BackgroundColor="#495057" FontSize={13} FontFamilyFallback="FontSymbols,FontSymbols2" Tapped={() => editor.current?.InsertAtCursor("🙂")} />
            <SkiaButton Text="Set Text" BackgroundColor="#495057" FontSize={13} Tapped={() => { if (editor.current) editor.current.Text = "Hello from code"; }} />
            <SkiaButton Text="Clear" BackgroundColor="#495057" FontSize={13} Tapped={() => { if (editor.current) editor.current.Text = ""; }} />
          </SkiaWrap>
          <SkiaLabel Text="A focused editor is mirrored by a hidden textarea (TextInputProxy): IME composition, the mobile soft keyboard, autocorrect, native paste / cut / undo land in the drawn editor through the same stub methods the physical keyboard uses." FontSize={12} TextColor="#ADB5BD" HorizontalOptions="Fill" />
        </Card>

        <Card title="ControlStyle — Cupertino, Material, Material3, Windows (C# ApplyControlStyleVisuals palettes)">
          <SkiaWrap Spacing={10}>
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" ControlStyle="Cupertino" PlaceholderText="Cupertino" WidthRequest={200} />
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" ControlStyle="Material" PlaceholderText="Material" WidthRequest={200} />
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" ControlStyle="Material3" PlaceholderText="Material3" WidthRequest={200} />
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" ControlStyle="Windows" PlaceholderText="Windows" WidthRequest={200} />
          </SkiaWrap>
        </Card>

        <Card title={`IsPassword — ${password.length} chars hidden behind bullets, KeyboardType Numeric / Email input modes`}>
          <SkiaWrap Spacing={10}>
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" IsPassword PlaceholderText="Password" WidthRequest={220} TextChanged={(_, t) => setPassword(t)} />
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" KeyboardType="Numeric" PlaceholderText="Numeric (inputmode)" WidthRequest={220} />
            <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" KeyboardType="Email" PlaceholderText="Email" WidthRequest={220} />
          </SkiaWrap>
        </Card>

        <Card title="Multiline — MaxLines={4}: Enter inserts a line, the box scrolls to the caret">
          <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" MaxLines={4} PlaceholderText="Write a few lines… wrapping, arrows, Shift+arrows select, double tap / long press selects a word" FontSize={15} />
        </Card>

        <Card title="Multiline + AutoHeight — MaxLines={-1}: the editor grows with the text">
          <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" MaxLines={-1} AutoHeight PlaceholderText="Grows as you type" FontSize={15} Text={"First line\nSecond line"} />
        </Card>

        <Card title={`Chat input — MaxLines={3} ReturnType="Send": Enter submits and keeps focus, Shift+Enter breaks the line · ${chat.length} sent`}>
          <SkiaStack Spacing={6}>
            {chat.slice(-4).map((m, i) => <SkiaLabel key={i} Text={m} FontSize={13} TextColor="#DEE2E6" BackgroundColor="#0F3460" Padding={new Thickness(10, 6)} HorizontalOptions="End" />)}
          </SkiaStack>
          <SkiaEditor FontFamilyFallback="FontSymbols,FontSymbols2,FontEmoji" MaxLines={3} ReturnType="Send" PlaceholderText="Message" FontSize={15} HorizontalOptions="Fill" TextSubmitted={(e, t) => { if (t.trim()) { setChat((c) => [...c, t]); e.Text = ""; } }} />
        </Card>
      </SkiaStack>
    </SkiaScroll>
  );
}
