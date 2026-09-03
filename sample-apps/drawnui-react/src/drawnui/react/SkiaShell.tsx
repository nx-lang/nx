import { createContext, forwardRef, type ReactNode, useCallback, useContext, useImperativeHandle, useMemo, useState } from "react";
import { Colors, Thickness } from "../core/Types";
import { SkiaButton, SkiaLabel, SkiaLayer } from "./index";

/** Route name -> page factory. Pages are plain JSX, rendered inside the shell when navigated to. */
export type ShellRoutes = Record<string, () => ReactNode>;

/** Navigation API, same verbs as DrawnUi SkiaShell. */
export interface ShellNavigation {
  GoToAsync(route: string): Promise<void>;
  GoBackAsync(): Promise<void>;
  /** Routes below the current page, root excluded. */
  NavigationStack: readonly string[];
  CanGoBack: boolean;
  /** Current route, "" for the root content. */
  Route: string;
}

const ShellContext = createContext<ShellNavigation | null>(null);

/** Navigation for pages and buttons rendered inside a SkiaShell. */
export function useShell(): ShellNavigation {
  const shell = useContext(ShellContext);
  if (!shell) throw new Error("DrawnUi: useShell() must be used inside <SkiaShell>");
  return shell;
}

export interface SkiaShellProps {
  Routes: ShellRoutes;
  /** Root content shown when the stack is empty. */
  children?: ReactNode;
  /** Nav bar height in points when a page is open. */
  NavBarHeight?: number;
  NavBarColor?: string;
  /** Title shown in the nav bar; defaults to the route name. */
  Titles?: Record<string, string>;
}

/**
 * Mirrors DrawnUi SkiaShell at the React level: a root page plus a stack of routed pages drawn inside one
 * SkiaLayer, with a nav bar and back button. Pages are JSX (route factories), navigation via ref or useShell().
 * Not ported yet: transitions, modals, popups, toasts, tabs, hardware back.
 */
export const SkiaShell = forwardRef<ShellNavigation, SkiaShellProps>(function SkiaShell(
  { Routes, children, NavBarHeight = 56, NavBarColor = "#212529", Titles }, ref,
) {
  const [stack, setStack] = useState<string[]>([]);
  const route = stack[stack.length - 1] ?? "";

  const GoToAsync = useCallback(async (r: string) => {
    if (!Routes[r]) throw new Error(`DrawnUi: route '${r}' is not registered in SkiaShell.Routes`);
    setStack((s) => [...s, r]);
  }, [Routes]);
  const GoBackAsync = useCallback(async () => setStack((s) => s.slice(0, -1)), []);

  const nav = useMemo<ShellNavigation>(() => ({
    GoToAsync, GoBackAsync, NavigationStack: stack, CanGoBack: stack.length > 0, Route: route,
  }), [GoToAsync, GoBackAsync, stack, route]);
  useImperativeHandle(ref, () => nav, [nav]);

  return (
    <ShellContext.Provider value={nav}>
      <SkiaLayer VerticalOptions="Fill">
        {route
          ? <SkiaLayer VerticalOptions="Fill" Margin={new Thickness(0, NavBarHeight, 0, 0)}>{Routes[route]()}</SkiaLayer>
          : children}
        {route && (
          <SkiaLayer HeightRequest={NavBarHeight} BackgroundColor={NavBarColor}>
            <SkiaButton Text="‹  Back" BackgroundColor="#00000000" TextColor="#6EA8FE" FontSize={16} VerticalOptions="Center" Margin={new Thickness(8, 0)} ApplyEffect="Ripple" Tapped={() => void GoBackAsync()} AccessibilityRole="button" AccessibilityLabel="Back" />
            <SkiaLabel Text={Titles?.[route] ?? route} FontSize={18} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Fill" HorizontalTextAlignment="Center" VerticalOptions="Center" MaxLines={1} Margin={new Thickness(96, 0)} AccessibilityRole="heading" />
            <SkiaLayer HeightRequest={1} VerticalOptions="End" BackgroundColor="#343A40" />
          </SkiaLayer>
        )}
      </SkiaLayer>
    </ShellContext.Provider>
  );
});
