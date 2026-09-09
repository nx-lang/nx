import type { SkiaControl } from "./SkiaControl";

/**
 * Mirrors DrawnUi.Net `Style` (SharedNet/Internals/Core/Style.cs): a set of property defaults for one control type,
 * registered once at startup through `Super.UseDrawnUi().ConfigureStyles(...)`.
 *
 * The C# `Setters` list of `Setter { Property, Value }` becomes a plain object here, because React controls are
 * classes with plain properties instead of BindableProperty: `Setters: { FontFamily: "FontText", FontSize: 14 }`.
 */
export interface Style {
  /** The control class the style targets, e.g. `SkiaLabel`. */
  TargetType: Function;
  /** Also style classes deriving from TargetType (C# Style.ApplyToDerivedTypes). Default false. */
  ApplyToDerivedTypes?: boolean;
  /** Property defaults, by property name. Anything the app sets itself wins over these. */
  Setters: Record<string, unknown>;
}

/** Mirrors DrawnUi.Net `IStylesCollection` / `StylesCollection`: the startup registry, static like the C# one. */
export class StylesCollection {
  /** Every style registered through ConfigureStyles, in registration order (C# StylesCollection.Styles). */
  static readonly Styles: Style[] = [];

  AddStyle(style: Style): StylesCollection {
    StylesCollection.Styles.push(style);
    SkiaStyles.InvalidateStylesCache();
    return this;
  }
}

/** Resolved per control class: the styles that target it, nearest base last so the nearest wins (C# ResolveStylesForType). */
interface ResolvedStyles { Setters: Record<string, unknown>; Empty: boolean }

/**
 * Applies registered styles to controls. Port of the C# `SkiaControl` styles partial
 * (SharedNet/Draw/SkiaControl.NetShared.cs) with one difference forced by the language: C# knows which properties the
 * app set explicitly (BindableProperty + ExplicitPropertiesSet) and skips those; here a property counts as "not set by
 * the app" while it still equals the value the control's own constructor left, captured once per class.
 */
export class SkiaStyles {
  private static readonly resolved = new Map<Function, ResolvedStyles>();
  private static readonly defaults = new Map<Function, Record<string, unknown>>();

  /** True while no style is registered: lets Measure skip the whole feature at zero cost. */
  static get IsEmpty(): boolean { return StylesCollection.Styles.length === 0; }

  /** C# InvalidateStylesCache: drop the per-type resolution after the registry changed. */
  static InvalidateStylesCache(): void { SkiaStyles.resolved.clear(); SkiaStyles.defaults.clear(); }

  /**
   * Setters that apply to a control class: the exact-type style plus every ApplyToDerivedTypes style of a base class,
   * furthest base first so a nearer style overrides it, exact type last (C# sorts by inheritance distance).
   */
  private static For(type: Function): ResolvedStyles {
    let hit = SkiaStyles.resolved.get(type);
    if (hit) return hit;

    const inherited: { Distance: number; Style: Style }[] = [];
    let exact: Style | undefined;
    for (const style of StylesCollection.Styles) {
      if (style.TargetType === type) { exact = style; continue; }
      if (!style.ApplyToDerivedTypes) continue;
      let distance = 0;
      for (let base = Object.getPrototypeOf(type); base; base = Object.getPrototypeOf(base)) {
        distance++;
        if (base === style.TargetType) { inherited.push({ Distance: distance, Style: style }); break; }
      }
    }
    inherited.sort((a, b) => b.Distance - a.Distance);

    const setters: Record<string, unknown> = {};
    for (const i of inherited) Object.assign(setters, i.Style.Setters);
    if (exact) Object.assign(setters, exact.Setters);

    hit = { Setters: setters, Empty: Object.keys(setters).length === 0 };
    SkiaStyles.resolved.set(type, hit);
    return hit;
  }

  /** The values a freshly constructed control of this class carries, for the properties a style touches. */
  private static SnapshotDefaults(type: Function, sample: SkiaControl, keys: string[]): void {
    if (SkiaStyles.defaults.has(type)) return;
    const known: Record<string, unknown> = {};
    for (const key of keys) known[key] = (sample as unknown as Record<string, unknown>)[key];
    SkiaStyles.defaults.set(type, known);
  }

  /**
   * C# ApplyInitialStyles: applies the registered defaults to one control. `atConstruction` is true when nothing has
   * touched the control yet (the reconciler calls it between `new` and the JSX props), so every setter applies and the
   * pre-style values are kept as this class's defaults; a control styled later only gets the properties that still
   * hold those defaults, which is how an explicit assignment wins — the C# ExplicitPropertiesSet rule without
   * property accessors.
   */
  static Apply(control: SkiaControl, atConstruction: boolean): void {
    const type = control.constructor;
    const resolved = SkiaStyles.For(type);
    if (resolved.Empty) return;

    const keys = Object.keys(resolved.Setters);
    const target = control as unknown as Record<string, unknown>;
    if (atConstruction) SkiaStyles.SnapshotDefaults(type, control, keys);

    const defaults = atConstruction ? undefined : SkiaStyles.defaults.get(type);
    for (const key of keys) {
      // no snapshot for this class (nothing of it ever went through the reconciler): apply as a plain default
      if (!defaults || Object.is(target[key], defaults[key])) target[key] = resolved.Setters[key];
    }
  }
}
