/** DrawnUi PrebuiltControlStyle: the look a styled control builds its default content with. */
export type PrebuiltControlStyle = "Unset" | "Platform" | "Cupertino" | "Material" | "Material3" | "Windows";

/** Resolved look: `Platform` picks from the browser's OS (C# picks from the running platform). */
export type ResolvedControlStyle = Exclude<PrebuiltControlStyle, "Platform">;

let platformStyle: ResolvedControlStyle | undefined;

/** C# UsingControlStyle: Platform -> iOS/macOS = Cupertino, Android = Material, Windows = Windows, else the DrawnUi default look. */
export function ResolveControlStyle(style: PrebuiltControlStyle): ResolvedControlStyle {
  if (style !== "Platform") return style;
  if (!platformStyle) {
    const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
    platformStyle = /iPhone|iPad|iPod|Macintosh/.test(ua) ? "Cupertino" : /Android/.test(ua) ? "Material" : /Windows/.test(ua) ? "Windows" : "Unset";
  }
  return platformStyle;
}
