/**
 * The playground's light or dark theme, following the website's.
 *
 * <para>nxlang.org is one origin, so the website's theme choice is readable here: Starlight keeps
 * it in `localStorage` under `starlight-theme`, as `light`, `dark`, or nothing for the system's
 * preference. The playground follows the same choice, so moving between the docs and the
 * playground does not flip the page's colors.</para>
 */
import { useEffect, useState } from "react";

export type Theme = "light" | "dark";

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
