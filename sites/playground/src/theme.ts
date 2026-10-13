/**
 * The playground's light or dark theme, following the website's.
 *
 * <para>nxlang.org is one origin, so the website's theme choice is readable here: Starlight keeps
 * it in `localStorage` under `starlight-theme`, as `light`, `dark`, or nothing for the system's
 * preference. The playground follows the same choice, so moving between the docs and the
 * playground does not flip the page's colors.</para>
 */
import { useEffect, useState, useSyncExternalStore } from "react";
import type { BundledTheme } from "shiki";

export type Theme = "light" | "dark";

/** A light and a dark Shiki theme that go together. */
export interface EditorThemes {
  readonly dark: BundledTheme;
  readonly light: BundledTheme;
}

/**
 * The pairs of Shiki themes the source, the reading and the output can be colored with, while they
 * are compared. `vscode` is the long-standing default of VS Code, Dark+ and Light+, whose colors
 * tell an attribute's name from its value and leave punctuation quiet. `github` is GitHub's current
 * pair, whose comments are the more readable.
 */
export const THEME_FAMILIES = {
  vscode: { dark: "dark-plus", light: "light-plus" },
  github: { dark: "github-dark-default", light: "github-light-default" },
} as const satisfies Record<string, EditorThemes>;

export type ThemeFamily = keyof typeof THEME_FAMILIES;

export const THEME_FAMILY_LABELS: Record<ThemeFamily, string> = {
  vscode: "VS Code Dark+ / Light+ (default)",
  github: "GitHub",
};

/** Every theme of every family, for an editor to load up front. */
export const ALL_EDITOR_THEMES: readonly BundledTheme[] = Object.values(THEME_FAMILIES).flatMap((family) => [
  family.dark,
  family.light,
]);

let family: ThemeFamily = "vscode";
const familyListeners = new Set<() => void>();

/** Tells the stylesheet which family is in use, so the panes take that family's background. */
function markFamily(): void {
  if (typeof document !== "undefined") {
    document.documentElement.dataset["themeFamily"] = family;
  }
}
markFamily();

/** Chooses the family everything is colored with. A comparison, not a setting: it is not kept. */
export function setThemeFamily(next: ThemeFamily): void {
  if (next !== family) {
    family = next;
    markFamily();
    for (const listener of familyListeners) {
      listener();
    }
  }
}

function subscribeToFamily(listener: () => void): () => void {
  familyListeners.add(listener);
  return () => familyListeners.delete(listener);
}

/** The family everything is colored with now. */
export function useThemeFamily(): ThemeFamily {
  return useSyncExternalStore(subscribeToFamily, () => family);
}

/** The pair of themes everything is colored with now. */
export function useEditorThemes(): EditorThemes {
  return THEME_FAMILIES[useThemeFamily()];
}

const STORAGE_KEY = "starlight-theme";

function systemTheme(): Theme {
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/** The theme the website would show now. */
export function siteTheme(): Theme {
  let stored: string | null = null;
  try {
    stored = window.localStorage.getItem(STORAGE_KEY);
  } catch {
    // Storage can be unavailable (a private window, blocked site data); the system decides then.
  }
  return stored === "light" || stored === "dark" ? stored : systemTheme();
}

/** The site theme, kept current as the system preference or another tab's choice changes. */
export function useSiteTheme(): Theme {
  const [theme, setTheme] = useState<Theme>(siteTheme);

  useEffect(() => {
    const update = () => setTheme(siteTheme());
    const media = window.matchMedia("(prefers-color-scheme: light)");
    media.addEventListener("change", update);
    window.addEventListener("storage", update);
    return () => {
      media.removeEventListener("change", update);
      window.removeEventListener("storage", update);
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset["theme"] = theme;
  }, [theme]);

  return theme;
}
