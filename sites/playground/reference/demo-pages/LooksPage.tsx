import { useState } from "react";
import { SkiaButton, SkiaCheckbox, SkiaLabel, SkiaProgress, SkiaRadioButton, SkiaRow, SkiaScroll, SkiaShape, SkiaSlider, SkiaStack, SkiaSwitch, Thickness } from "drawnui-react";
import type { PrebuiltControlStyle } from "drawnui-react/core";

/** Port of the Fiddle "Looks" preset: one card per prebuilt style, same controls, only ControlStyle changes. */
function Card({ title, style, log }: { title: string; style: PrebuiltControlStyle; log: (s: string) => void }) {
  return (
    <SkiaShape Type="Rectangle" CornerRadius={16} BackgroundColor="#F5F5F5" Padding={new Thickness(18, 14)} HorizontalOptions="Fill" UseCache="Image">
      <SkiaStack Spacing={14} HorizontalOptions="Fill">
        <SkiaLabel Text={title} FontSize={16} FontAttributes="Bold" TextColor="#111827" />

        <SkiaRow Spacing={16} HorizontalOptions="Fill">
          <SkiaSwitch ControlStyle={style} IsToggled VerticalOptions="Center" Toggled={(_, v) => log(`${title} switch: ${v}`)} />
          <SkiaCheckbox ControlStyle={style} IsToggled VerticalOptions="Center" Toggled={(_, v) => log(`${title} checkbox: ${v}`)} />
          <SkiaRadioButton ControlStyle={style} Text="One" IsToggled GroupName={title} VerticalOptions="Center" Toggled={(_, v) => v && log(`${title} radio: One`)} />
          <SkiaRadioButton ControlStyle={style} Text="Two" GroupName={title} VerticalOptions="Center" Toggled={(_, v) => v && log(`${title} radio: Two`)} />
        </SkiaRow>

        <SkiaButton Text="Button" ControlStyle={style} HorizontalOptions="Start" Tapped={() => log(`${title} button tapped`)} />

        <SkiaProgress ControlStyle={style} Value={65} HorizontalOptions="Fill" />

        <SkiaSlider ControlStyle={style} End={65} HorizontalOptions="Fill" EndChanged={(_, v) => log(`${title} slider: ${v.toFixed(0)}`)} />

        {/* range mode: two thumbs */}
        <SkiaSlider ControlStyle={style} EnableRange Start={20} End={80} HorizontalOptions="Fill" StartChanged={(_, v) => log(`${title} range start: ${v.toFixed(0)}`)} EndChanged={(_, v) => log(`${title} range end: ${v.toFixed(0)}`)} />
      </SkiaStack>
    </SkiaShape>
  );
}

export function LooksPage() {
  const [last, setLast] = useState("interact with any control");
  return (
    <SkiaScroll Orientation="Vertical">
      <SkiaStack Spacing={16} Padding={new Thickness(16)} HorizontalOptions="Center" MaximumWidthRequest={720}>
        <SkiaLabel Text="Platform Looks" FontSize={24} TextColor="#FFFFFF" HorizontalOptions="Center" />
        <SkiaLabel Text="SkiaSwitch, SkiaCheckbox, SkiaRadioButton, SkiaButton, SkiaProgress, SkiaSlider — the same tree per card, only ControlStyle changes (the Fiddle 'Looks' snippet)." FontSize={13} TextColor="#ADB5BD" HorizontalOptions="Fill" HorizontalTextAlignment="Center" />
        <SkiaLabel Text={`Last: ${last}`} FontSize={13} TextColor="#6EA8FE" HorizontalOptions="Center" />
        <Card title="Default" style="Unset" log={setLast} />
        <Card title="Windows — Fluent" style="Windows" log={setLast} />
        <Card title="Cupertino — iOS" style="Cupertino" log={setLast} />
        <Card title="Material — Android" style="Material" log={setLast} />
        <Card title="Material3 — Android" style="Material3" log={setLast} />
      </SkiaStack>
    </SkiaScroll>
  );
}
