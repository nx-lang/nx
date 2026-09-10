import { createContext, forwardRef, type ReactNode, useCallback, useContext, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { Colors, Thickness } from "../core/Types";
import { Super } from "../core/Super";
import { Easing } from "../core/Easing";
import type { SkiaControl } from "../core/SkiaControl";
import type { SkiaDrawer as SkiaDrawerCtrl } from "../controls/SkiaDrawer";
import type { ControlTappedEventArgs } from "../core/Gestures";
import { SkiaBackdrop, SkiaButton, SkiaDrawer, SkiaGrid, SkiaLabel, SkiaLayer, SkiaRichLabel, SkiaShape } from "./index";

/** Arguments passed to a page (C# GoToAsync arguments / query string), e.g. `GoToAsync("detail", true, { id: 42 })` or `GoToAsync("detail?id=42")`. */
export type ShellArguments = Record<string, unknown>;
/** Route name -> page factory receiving the navigation arguments. Pages are plain JSX, rendered inside the shell when navigated to. */
export type ShellRoutes = Record<string, (args: ShellArguments) => ReactNode>;
/** C# NavigationSource. */
export type NavigationSource = "Push" | "Pop";
/** C# SkiaShellNavigatingArgs: set Cancel to stop the navigation. */
export interface SkiaShellNavigatingArgs { Route: string; Source: NavigationSource; Cancel: boolean; View?: SkiaControl; Previous?: SkiaControl }
/** C# SkiaShellNavigatedArgs. */
export interface SkiaShellNavigatedArgs { Route: string; Source: NavigationSource; View?: SkiaControl }

/** "name?a=1&b=2" -> name + arguments (query values are strings). */
export function SplitRoute(route: string): { name: string; args: ShellArguments } {
  const q = route.indexOf("?");
  if (q < 0) return { name: route, args: {} };
  const args: ShellArguments = {};
  new URLSearchParams(route.slice(q + 1)).forEach((v, k) => { args[k] = v; });
  return { name: route.slice(0, q), args };
}
/** name + arguments -> "name?a=1" (only string / number / boolean values travel in the route, the rest stays in memory). */
export function BuildRoute(name: string, args?: ShellArguments): string {
  if (!args) return name;
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(args)) if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") p.set(k, String(v));
  const q = p.toString();
  return q ? `${name}?${q}` : name;
}

/** C# OpenPopupAsync parameters. */
export interface PopupOptions {
  animated?: boolean;
  /** A tap outside the content closes the popup (default true). */
  closeWhenBackgroundTapped?: boolean;
  /** Dim + blur the page behind (default true). */
  showOverlay?: boolean;
  backgroundColor?: string;
}

/** C# PushModalAsync parameters. */
export interface ModalOptions {
  /** The modal can be dragged down to close (default false). */
  useGestures?: boolean;
  animated?: boolean;
  /** Dim + blur the page behind (C# freezeBackground; the frozen screenshot is not ported). */
  freezeBackground?: boolean;
}

/** A shell tab: its root route and the label shown in the tab bar. */
export interface ShellTab { route: string; title: string }

/** Navigation API, same verbs as DrawnUi SkiaShell. */
export interface ShellNavigation {
  /** Pushes a registered route; `route` may carry a query ("detail?id=42") and/or explicit arguments (C# GoToAsync(route, animate, arguments)). */
  GoToAsync(route: string, animated?: boolean, args?: ShellArguments): Promise<void>;
  /** C# GoBack: closes the top popup, else the top modal, else pops the page. */
  GoBackAsync(animated?: boolean): Promise<void>;
  PopToRootAsync(): Promise<void>;
  /** Routes below the current page of the current tab, root excluded. */
  NavigationStack: readonly string[];
  CanGoBack: boolean;
  /** Current route (with its query), "" for the root content. */
  Route: string;
  /** Arguments of the current page (query + explicit). */
  Arguments: ShellArguments;
  OpenPopupAsync(content: ReactNode, options?: PopupOptions): Promise<void>;
  ClosePopupAsync(animated?: boolean): Promise<void>;
  CloseAllPopups(): Promise<void>;
  PushModalAsync(content: ReactNode, options?: ModalOptions): Promise<void>;
  PopModalAsync(animated?: boolean): Promise<void>;
  ShowToast(content: string | ReactNode, msShowTime?: number): void;
  CloseAllToasts(): Promise<void>;
  PopupsCount: number;
  ModalsCount: number;
  ToastsCount: number;
  /** Tabs (C# SkiaViewSwitcher.SelectedIndex): each tab keeps its own page stack. */
  SelectedTab: number;
  SelectTabAsync(index: number): Promise<void>;
  /** Pops every page of the current tab (C# PopTabToRoot). */
  PopTabToRootAsync(): Promise<void>;
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
  /** Root content shown when the stack is empty (ignored when Tabs are set: the selected tab's route is the root). */
  children?: ReactNode;
  /** Nav bar height in points when a page is open. */
  NavBarHeight?: number;
  NavBarColor?: string;
  /** Title shown in the nav bar; defaults to the route name. */
  Titles?: Record<string, string>;
  /** Page push/pop slide duration in ms (C# SkiaViewSwitcher.PagesAnimationSpeed). */
  PagesAnimationSpeed?: number;
  /** Opaque background of pushed pages (they slide over the page below). */
  PageBackgroundColor?: string;
  /** Bottom tab bar: one root route per tab, per-tab navigation stacks (C# SkiaViewSwitcher tabs). */
  Tabs?: ShellTab[];
  TabBarHeight?: number;
  TabBarColor?: string;
  /** Mirror pages in the URL hash (#/route/route) and honour the browser back button (React extension, default true). */
  UseBrowserHistory?: boolean;
  /** Slide + fade between tabs (C# SkiaViewSwitcher.AnimateTabs, default false). */
  AnimateTabs?: boolean;
  /** Tab switch duration in ms (C# TabsAnimationSpeed). */
  TabsAnimationSpeed?: number;
  /** Before a page / popup / modal is pushed or popped; set e.Cancel = true to stop it (C# Navigating). */
  Navigating?: (e: SkiaShellNavigatingArgs) => void;
  /** After a page / popup / modal went upfront or was removed (C# Navigated). */
  Navigated?: (e: SkiaShellNavigatedArgs) => void;
  /** The current route changed (C# RouteChanged). */
  RouteChanged?: (route: string) => void;
  /** Safe-area insets (points) reserved above the nav bar and below the tab bar; default Super.Insets (C# Super.Insets). */
  Insets?: Thickness;
}

/** C# SkiaViewSwitcher.Custom easing (back-ease, side coefficient 0.55) used for tab switches. */
const TabsEasing = new Easing((x) => (x - 1) * (x - 1) * ((0.55 + 1) * (x - 1) + 0.55) + 1);

/** C# SkiaShell static defaults (mutable). */
export const ShellDefaults = {
  ToastBackgroundColor: "#CC000000",
  ToastTextColor: Colors.White,
  ToastTextFont: "",
  ToastTextSize: 16,
  ToastTextMargins: 24,
  /** Default overlay tint behind popups / modals. */
  PopupBackgroundColor: "#66000000",
  /** Backdrop blur behind popups / modals, points. */
  PopupsBackgroundBlur: 6,
  PopupsAnimationSpeed: 250,
  PopupsCancelAnimationsAfterMs: 1500,
  ZIndexModals: 1000,
  ZIndexPopups: 2000,
  ZIndexToasts: 3000,
  /** Accent of the selected tab. */
  TabSelectedColor: "#6EA8FE",
  TabColor: "#ADB5BD",
};

interface Overlay { id: number; node: ReactNode; ctrl?: SkiaControl; drawer?: SkiaDrawerCtrl; content?: SkiaControl; closing?: boolean; history?: boolean; animated?: boolean }
type HistoryKind = "page" | "popup" | "modal";

/** Canvas width in points for a control (page slide distance); falls back to the window. */
function CanvasWidthPts(ctrl: SkiaControl): number {
  const el = (ctrl.Superview as unknown as { Element?: HTMLCanvasElement } | undefined)?.Element;
  return el ? el.clientWidth : window.innerWidth;
}

/** Resolves once pred() is truthy (a ref set by the React commit) or after timeout ms. */
async function WaitFor<T>(pred: () => T | undefined | null | false, timeout = 1000): Promise<T | undefined> {
  const started = performance.now();
  for (;;) {
    const v = pred();
    if (v) return v;
    if (performance.now() - started > timeout) return undefined;
    await new Promise((r) => setTimeout(r, 16));
  }
}

function AnimateWithTimeout(promise: Promise<unknown>, ms = ShellDefaults.PopupsCancelAnimationsAfterMs): Promise<void> {
  return Promise.race([promise.catch(() => undefined), new Promise<void>((r) => setTimeout(r, ms))]).then(() => undefined);
}

const HISTORY_KEY = "drawnui-shell";

/**
 * Mirrors DrawnUi SkiaShell at the React level: a root page (or tabs) plus per-tab stacks of routed pages drawn
 * inside one SkiaLayer with a nav bar (pages slide in/out like SkiaViewSwitcher), popups (blurred + dimmed
 * backdrop, scale + fade, tap outside to close), modals (full-screen SkiaDrawer from the bottom, optionally
 * draggable), toasts (bottom banner, slide + fade, auto close). Pages live in the URL hash and the browser back
 * button behaves like C# GoBack (popup, then modal, then page). Navigation via ref or useShell().
 */
export const SkiaShell = forwardRef<ShellNavigation, SkiaShellProps>(function SkiaShell(
  { Routes, children, NavBarHeight = 56, NavBarColor = "#212529", Titles, PagesAnimationSpeed = 200, PageBackgroundColor = "#212529", Tabs, TabBarHeight = 56, TabBarColor = "#212529", UseBrowserHistory = true, AnimateTabs = false, TabsAnimationSpeed = 150, Navigating, Navigated, RouteChanged, Insets }, ref,
) {
  const hasTabs = !!Tabs && Tabs.length > 0;
  const [systemInsets, setSystemInsets] = useState(() => Super.Insets);
  useEffect(() => Super.OnInsetsChanged(() => setSystemInsets(Super.Insets)), []);
  const insets = Insets ?? systemInsets;
  const argsInMemory = useRef(new Map<string, ShellArguments>()); // arguments that cannot travel in the route string
  const RenderRoute = useCallback((r: string): ReactNode => {
    const { name, args } = SplitRoute(r);
    const factory = Routes[name];
    return factory ? factory({ ...args, ...argsInMemory.current.get(r) }) : null;
  }, [Routes]);
  const RouteName = (r: string) => SplitRoute(r).name;
  const events = useRef({ Navigating, Navigated, RouteChanged });
  events.current = { Navigating, Navigated, RouteChanged };
  /** C# NotifyAndCheckCanNavigate: raises Navigating, false when cancelled. */
  const canNavigate = useCallback((r: string, source: NavigationSource, view?: SkiaControl, previous?: SkiaControl): boolean => {
    const e: SkiaShellNavigatingArgs = { Route: r ?? "", Source: source, Cancel: false, View: view, Previous: previous };
    events.current.Navigating?.(e);
    return !e.Cancel;
  }, []);
  const navigated = useCallback((r: string, source: NavigationSource, view?: SkiaControl) => { events.current.Navigated?.({ Route: r ?? "", Source: source, View: view }); }, []);
  const [selectedTab, setSelectedTab] = useState(0);
  /** The tab root leaving during an animated switch (C# PreviousVisibleView) and the slide direction. */
  const [tabTransition, setTabTransition] = useState<{ from: number; dir: 1 | -1 } | null>(null);
  const tabRoots = useRef(new Map<number, SkiaControl>());
  const [stacks, setStacks] = useState<string[][]>(() => [[]]);
  const stack = stacks[selectedTab] ?? [];
  const setStack = useCallback((update: (s: string[]) => string[]) => setStacks((all) => { const next = all.slice(); next[selectedTab] = update(next[selectedTab] ?? []); return next; }), [selectedTab]);
  const [leaving, setLeaving] = useState<string | null>(null); // page sliding out during a pop
  const [transitions, setTransitions] = useState(0); // pages sliding in: the page below stays visible meanwhile
  const [popups, setPopups] = useState<Overlay[]>([]);
  const [modals, setModals] = useState<Overlay[]>([]);
  const [toasts, setToasts] = useState<Overlay[]>([]);
  const nextId = useRef(1);
  const pageCtrls = useRef(new Map<string, SkiaControl>());
  const route = stack[stack.length - 1] ?? "";
  useEffect(() => { events.current.RouteChanged?.(route); }, [route]);

  // latest values for callbacks fired from history events
  const live = useRef({ stack, popups, modals, selectedTab });
  live.current = { stack, popups, modals, selectedTab };

  // ---- browser history: one entry per pushed page / popup / modal, popped in C# GoBack order ----
  const history = useRef<HistoryKind[]>([]);
  const historyOn = UseBrowserHistory && typeof window !== "undefined";
  const hashFor = (s: string[]) => (s.length ? "#/" + s.map(encodeURIComponent).join("/") : window.location.pathname + window.location.search);
  const pushHistory = useCallback((kind: HistoryKind, s: string[]) => {
    if (!historyOn) return;
    history.current.push(kind);
    window.history.pushState({ [HISTORY_KEY]: history.current.length }, "", kind === "page" ? hashFor(s) : undefined);
  }, [historyOn]);
  /** A programmatic close of something that has a history entry goes through history.back(), so the URL stays in sync. */
  const backThroughHistory = useCallback((kind: HistoryKind): boolean => {
    if (!historyOn || history.current[history.current.length - 1] !== kind) return false;
    window.history.back();
    return true;
  }, [historyOn]);

  // ---- pages ----
  const slideOut = useCallback(async (current: string, animated: boolean) => {
    const ctrl = pageCtrls.current.get(current);
    if (animated && ctrl) {
      setLeaving(current);
      await AnimateWithTimeout(ctrl.TranslateToAsync(CanvasWidthPts(ctrl), 0, PagesAnimationSpeed), PagesAnimationSpeed + 500);
      setLeaving(null);
    }
  }, [PagesAnimationSpeed]);

  /** Pops the top page; false when Navigating cancelled it (C# NavigationSource.Pop). */
  const popPage = useCallback(async (animated: boolean): Promise<boolean> => {
    const s = live.current.stack;
    const current = s[s.length - 1];
    if (!current) return false;
    const below = s[s.length - 2] ?? "";
    if (!canNavigate(below, "Pop", pageCtrls.current.get(current), pageCtrls.current.get(current))) return false;
    await slideOut(current, animated);
    setStack((x) => x.slice(0, -1));
    argsInMemory.current.delete(current);
    navigated(below, "Pop", pageCtrls.current.get(below));
    return true;
  }, [slideOut, setStack, canNavigate, navigated]);

  const GoToAsync = useCallback(async (r: string, animated = true, args?: ShellArguments) => {
    const { name, args: fromRoute } = SplitRoute(r);
    if (!Routes[name]) throw new Error(`DrawnUi: route '${name}' is not registered in SkiaShell.Routes`);
    const full = BuildRoute(name, { ...fromRoute, ...args });
    if (!canNavigate(full, "Push", undefined, pageCtrls.current.get(live.current.stack[live.current.stack.length - 1] ?? ""))) return;
    if (args) argsInMemory.current.set(full, { ...fromRoute, ...args });
    const next = [...live.current.stack, full];
    setStack(() => next);
    pushHistory("page", next);
    if (animated) { setTransitions((t) => t + 1); await new Promise((res) => setTimeout(res, PagesAnimationSpeed + 50)); setTransitions((t) => t - 1); }
    navigated(full, "Push", await WaitFor(() => pageCtrls.current.get(full)));
  }, [Routes, PagesAnimationSpeed, setStack, pushHistory, canNavigate, navigated]);

  // ---- popups ----
  const closePopupNow = useCallback(async (p: Overlay, animated: boolean): Promise<boolean> => {
    if (p.closing) return false;
    if (!canNavigate(live.current.stack[live.current.stack.length - 1] ?? "", "Pop", p.content, p.content)) return false;
    p.closing = true;
    if (animated && p.animated !== false && p.ctrl && p.content) {
      await AnimateWithTimeout(Promise.all([p.ctrl.FadeToAsync(0, ShellDefaults.PopupsAnimationSpeed), p.content.ScaleToAsync(0, 0, ShellDefaults.PopupsAnimationSpeed)]));
    }
    setPopups((list) => list.filter((x) => x !== p));
    navigated(live.current.stack[live.current.stack.length - 1] ?? "", "Pop");
    return true;
  }, [canNavigate, navigated]);
  const closePopup = useCallback(async (p: Overlay, animated: boolean) => {
    if (p.closing) return;
    if (p.history && backThroughHistory("popup")) { p.history = false; return; } // popstate closes it
    await closePopupNow(p, animated);
  }, [backThroughHistory, closePopupNow]);

  const OpenPopupAsync = useCallback(async (content: ReactNode, options?: PopupOptions) => {
    const animated = options?.animated ?? true;
    if (!canNavigate(live.current.stack[live.current.stack.length - 1] ?? "", "Push")) return;
    const entry: Overlay = { id: nextId.current++, node: null, animated };
    const closeWhenBackgroundTapped = options?.closeWhenBackgroundTapped ?? true;
    const showOverlay = options?.showOverlay ?? true;
    const overlay = options?.backgroundColor ?? ShellDefaults.PopupBackgroundColor;
    entry.node = (
      <SkiaLayer key={entry.id} VerticalOptions="Fill" HorizontalOptions="Fill" ZIndex={ShellDefaults.ZIndexPopups + entry.id} BlockGesturesBelow Opacity={animated ? 0.1 : 1}
        ref={(c: SkiaControl | null) => { if (c) entry.ctrl = c; }}
        Tapped={(_, e: ControlTappedEventArgs) => {
          // C# PopupWrapper: a tap outside the content closes it
          const c = entry.content;
          const l = e.ProcessingInfo.MappedLocation, o = e.ProcessingInfo.ChildOffset;
          if (closeWhenBackgroundTapped && c && !c.HitIsInside(l.X + o.X, l.Y + o.Y)) void closePopup(entry, animated);
        }}>
        {showOverlay && <SkiaBackdrop Blur={ShellDefaults.PopupsBackgroundBlur} BackgroundColor={overlay} InputTransparent />}
        <SkiaLayer HorizontalOptions="Center" VerticalOptions="Center" Scale={animated ? 0.5 : 1} ref={(c: SkiaControl | null) => { if (c) entry.content = c; }}>
          {content}
        </SkiaLayer>
      </SkiaLayer>
    );
    setPopups((list) => [...list, entry]);
    entry.history = true;
    pushHistory("popup", live.current.stack);
    await WaitFor(() => entry.ctrl && entry.content); // mounted after the React commit
    if (animated && entry.ctrl && entry.content) await AnimateWithTimeout(Promise.all([entry.ctrl.FadeToAsync(1, ShellDefaults.PopupsAnimationSpeed), entry.content.ScaleToAsync(1, 1, ShellDefaults.PopupsAnimationSpeed)]));
    navigated(live.current.stack[live.current.stack.length - 1] ?? "", "Push", entry.content);
  }, [closePopup, pushHistory, canNavigate, navigated]);

  const ClosePopupAsync = useCallback(async (animated = true) => { const list = live.current.popups; const top = list[list.length - 1]; if (top) await closePopup(top, animated); }, [closePopup]);
  const CloseAllPopups = useCallback(async () => { for (const p of [...live.current.popups].reverse()) await closePopup(p, false); }, [closePopup]);

  // ---- modals ----
  const removeModal = useCallback((m: Overlay) => setModals((list) => list.filter((x) => x !== m)), []);

  const popModalNow = useCallback(async (top: Overlay, animated: boolean): Promise<boolean> => {
    if (top.closing) return false;
    if (!canNavigate(live.current.stack[live.current.stack.length - 1] ?? "", "Pop", top.drawer, top.drawer)) return false;
    top.closing = true;
    if (top.drawer && top.drawer.IsOpen) {
      top.drawer.Animated = animated;
      const done = new Promise<void>((r) => { const prev = top.drawer!.StateTransitionComplete; top.drawer!.StateTransitionComplete = (d, o) => { prev?.(d, o); if (!o) r(); }; });
      top.drawer.IsOpen = false;
      if (animated) await AnimateWithTimeout(done);
    }
    removeModal(top);
    navigated(live.current.stack[live.current.stack.length - 1] ?? "", "Pop");
    return true;
  }, [removeModal, canNavigate, navigated]);

  const PushModalAsync = useCallback(async (content: ReactNode, options?: ModalOptions) => {
    const animated = options?.animated ?? true;
    if (!canNavigate(live.current.stack[live.current.stack.length - 1] ?? "", "Push")) return;
    const entry: Overlay = { id: nextId.current++, node: null };
    const overlay = (options?.freezeBackground ?? true) ? ShellDefaults.PopupBackgroundColor : undefined;
    let opened: () => void = () => {};
    const openedPromise = new Promise<void>((r) => { opened = r; });
    // the user closed it (drag down / IsOpen = false): leave through history when it owns an entry, else remove
    const userClosed = () => { if (entry.closing) return; if (entry.history && backThroughHistory("modal")) entry.history = false; else { entry.closing = true; removeModal(entry); } };
    entry.node = (
      <SkiaLayer key={entry.id} VerticalOptions="Fill" HorizontalOptions="Fill" ZIndex={ShellDefaults.ZIndexModals + entry.id} BlockGesturesBelow>
        {overlay && <SkiaBackdrop Blur={ShellDefaults.PopupsBackgroundBlur} BackgroundColor={overlay} InputTransparent />}
        <SkiaDrawer ref={(d: SkiaDrawerCtrl | null) => { if (d) entry.drawer = d; }} Direction="FromBottom" HeaderSize={0} HorizontalOptions="Fill" VerticalOptions="Fill"
          RespondsToGestures={options?.useGestures ?? false} Animated={animated} Bounces={false} BlockGesturesBelow
          StateTransitionComplete={(d, isOpen) => { if (isOpen) opened(); else if (!d.IsOpen) userClosed(); }}
          IsOpenChanged={(_, isOpen) => { if (!isOpen && !animated) userClosed(); }}>
          {content}
        </SkiaDrawer>
      </SkiaLayer>
    );
    setModals((list) => [...list, entry]);
    entry.history = true;
    pushHistory("modal", live.current.stack);
    const drawer = await WaitFor(() => entry.drawer);
    await WaitFor(() => drawer && drawer.DrawingRect.Height > 0); // first draw: the drawer measured its travel
    if (drawer) { drawer.IsOpen = true; if (!animated) opened(); }
    await AnimateWithTimeout(openedPromise);
    navigated(live.current.stack[live.current.stack.length - 1] ?? "", "Push", drawer);
  }, [removeModal, pushHistory, backThroughHistory, canNavigate, navigated]);

  const PopModalAsync = useCallback(async (animated = true) => {
    const list = live.current.modals; const top = list[list.length - 1];
    if (!top || top.closing) return;
    if (top.history && backThroughHistory("modal")) { top.history = false; return; }
    await popModalNow(top, animated);
  }, [popModalNow, backThroughHistory]);

  // ---- toasts ----
  const closeToast = useCallback(async (t: Overlay, animated: boolean) => {
    if (t.closing) return;
    t.closing = true;
    const c = t.ctrl;
    if (animated && c) await AnimateWithTimeout(Promise.all([c.TranslateToAsync(0, c.DrawingRect.Height / c.RenderingScale, 250), c.FadeToAsync(0, 250)]));
    setToasts((list) => list.filter((x) => x !== t));
  }, []);
  const toastsRef = useRef<Overlay[]>([]);
  toastsRef.current = toasts;
  const CloseAllToasts = useCallback(async () => { await Promise.all(toastsRef.current.map((t) => closeToast(t, false))); }, [closeToast]);

  const ShowToast = useCallback((content: string | ReactNode, msShowTime = 4000) => {
    void (async () => {
      await CloseAllToasts();
      const entry: Overlay = { id: nextId.current++, node: null };
      const body = typeof content === "string"
        ? <SkiaRichLabel Text={content} TextColor={ShellDefaults.ToastTextColor} FontFamily={ShellDefaults.ToastTextFont || undefined} FontSize={ShellDefaults.ToastTextSize} Margin={new Thickness(ShellDefaults.ToastTextMargins)} HorizontalOptions="Fill" />
        : content;
      entry.node = (
        <SkiaLayer key={entry.id} HorizontalOptions="Fill" VerticalOptions="End" ZIndex={ShellDefaults.ZIndexToasts + entry.id} BackgroundColor={ShellDefaults.ToastBackgroundColor} BlockGesturesBelow UseCache="Operations" Opacity={0}
          ref={(c: SkiaControl | null) => { if (c) entry.ctrl = c; }}>
          <SkiaLayer HorizontalOptions="Fill">{body}</SkiaLayer>
        </SkiaLayer>
      );
      setToasts((list) => [...list, entry]);
      const c = await WaitFor(() => entry.ctrl);
      await WaitFor(() => c && c.DrawingRect.Height > 0); // laid out: height known
      if (c) {
        c.TranslationY = c.DrawingRect.Height / c.RenderingScale;
        void c.TranslateToAsync(0, 0, 300);
        void c.FadeToAsync(1, 300);
      }
      setTimeout(() => void closeToast(entry, true), msShowTime);
    })();
  }, [CloseAllToasts, closeToast]);

  // ---- back (C# GoBackDefault order) ----
  const GoBackAsync = useCallback(async (animated = true) => {
    const { popups: ps, modals: ms, stack: s } = live.current;
    if (ps.length) { await closePopup(ps[ps.length - 1], animated); return; }
    if (ms.length) { await PopModalAsync(animated); return; }
    if (!s.length) return;
    if (backThroughHistory("page")) { await WaitFor(() => live.current.stack.length < s.length, PagesAnimationSpeed + 800); return; }
    await popPage(animated);
  }, [closePopup, PopModalAsync, backThroughHistory, popPage, PagesAnimationSpeed]);

  const PopToRootAsync = useCallback(async () => {
    setLeaving(null);
    if (historyOn) { const pages = history.current.filter((k) => k === "page").length; history.current = history.current.filter((k) => k !== "page"); if (pages) window.history.replaceState({ [HISTORY_KEY]: history.current.length }, "", hashFor([])); }
    setStacks((all) => all.map(() => []));
  }, [historyOn]);

  const PopTabToRootAsync = useCallback(async () => { setLeaving(null); setStack(() => []); }, [setStack]);
  const SelectTabAsync = useCallback(async (index: number) => {
    if (!hasTabs) return;
    const to = Math.max(0, Math.min(Tabs!.length - 1, index));
    const from = live.current.selectedTab;
    setLeaving(null);
    if (to === from) return;
    if (!AnimateTabs) { setSelectedTab(to); return; }
    // C# SelectRightTab / SelectLeftTab: the new root slides in from 0.75 width with a fade, the old one slides out
    const dir: 1 | -1 = to > from ? 1 : -1;
    setTabTransition({ from, dir });
    setSelectedTab(to);
    const next = await WaitFor(() => tabRoots.current.get(to));
    const prev = tabRoots.current.get(from);
    if (next) {
      const w = CanvasWidthPts(next);
      await AnimateWithTimeout(Promise.all([
        next.TranslateToAsync(0, 0, TabsAnimationSpeed, TabsEasing),
        next.FadeToAsync(1, TabsAnimationSpeed, Easing.Linear),
        prev ? prev.TranslateToAsync(-dir * w, 0, TabsAnimationSpeed, TabsEasing) : Promise.resolve(),
      ]), TabsAnimationSpeed + 500);
    }
    setTabTransition(null);
  }, [hasTabs, Tabs, AnimateTabs, TabsAnimationSpeed]);

  // popstate: the browser popped OUR last entry (back) — or moved forward to a hash we can rebuild
  useEffect(() => {
    if (!historyOn) return;
    const parseHash = () => window.location.hash.replace(/^#\/?/, "").split("/").filter(Boolean).map(decodeURIComponent).filter((r) => !!Routes[RouteName(r)]);
    // initial deep link
    const initial = parseHash();
    if (initial.length) { setStacks((all) => { const next = all.slice(); next[live.current.selectedTab] = initial; return next; }); history.current = initial.map(() => "page" as HistoryKind); window.history.replaceState({ [HISTORY_KEY]: history.current.length }, "", hashFor(initial)); }
    const onPop = () => {
      const depth = (window.history.state as Record<string, number> | null)?.[HISTORY_KEY] ?? 0;
      if (depth < history.current.length) {
        // back: unwind our entries down to depth
        void (async () => {
          while (history.current.length > depth) {
            const kind = history.current.pop();
            const { popups: ps, modals: ms } = live.current;
            let done = true;
            if (kind === "popup") { const top = ps[ps.length - 1]; if (top) { top.history = false; done = await closePopupNow(top, true); if (!done) top.history = true; } }
            else if (kind === "modal") { const top = ms[ms.length - 1]; if (top) { top.history = false; done = await popModalNow(top, true); if (!done) top.history = true; } }
            else if (kind === "page") done = await popPage(true);
            if (!done && kind) { // Navigating cancelled: put the browser entry back
              history.current.push(kind);
              window.history.pushState({ [HISTORY_KEY]: history.current.length }, "", kind === "page" ? hashFor(live.current.stack) : undefined);
              break;
            }
          }
        })();
      } else {
        // forward (depth grew), or the same depth with another hash = a plain <a href="#/route"> link or a typed hash:
        // the hash names the pages, rebuild the stack without animation (overlays cannot be restored)
        const target = parseHash();
        if (depth === history.current.length && target.join("/") === live.current.stack.join("/")) return;
        history.current = target.map(() => "page" as HistoryKind);
        window.history.replaceState({ [HISTORY_KEY]: history.current.length }, "", hashFor(target));
        setStacks((all) => { const next = all.slice(); next[live.current.selectedTab] = target; return next; });
      }
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, [historyOn, Routes, closePopupNow, popModalNow, popPage]);

  const nav = useMemo<ShellNavigation>(() => ({
    GoToAsync, GoBackAsync, PopToRootAsync, NavigationStack: stack, CanGoBack: stack.length > 0 || popups.length > 0 || modals.length > 0, Route: route, Arguments: { ...SplitRoute(route).args, ...argsInMemory.current.get(route) },
    OpenPopupAsync, ClosePopupAsync, CloseAllPopups, PushModalAsync, PopModalAsync, ShowToast, CloseAllToasts,
    PopupsCount: popups.length, ModalsCount: modals.length, ToastsCount: toasts.length,
    SelectedTab: selectedTab, SelectTabAsync, PopTabToRootAsync,
  }), [GoToAsync, GoBackAsync, PopToRootAsync, stack, route, OpenPopupAsync, ClosePopupAsync, CloseAllPopups, PushModalAsync, PopModalAsync, ShowToast, CloseAllToasts, popups.length, modals.length, toasts.length, selectedTab, SelectTabAsync, PopTabToRootAsync]);
  useImperativeHandle(ref, () => nav, [nav]);

  const visiblePages = leaving && !stack.includes(leaving) ? [...stack, leaving] : stack;
  const topRoute = visiblePages[visiblePages.length - 1] ?? "";
  const transitioning = transitions > 0 || leaving !== null;
  const rootBottom = hasTabs ? TabBarHeight + insets.Bottom : 0;
  const navTop = NavBarHeight + insets.Top;
  const rootNode = hasTabs ? RenderRoute(Tabs![selectedTab]?.route ?? "") : children;
  const entering = tabTransition ? { TranslationX: tabTransition.dir * 0.75 * (typeof window !== "undefined" ? window.innerWidth : 0), Opacity: 0.001, ZIndex: 1 } : {};

  return (
    <ShellContext.Provider value={nav}>
      <SkiaLayer VerticalOptions="Fill">
        {tabTransition && (
          <SkiaLayer key={`tab-leaving-${tabTransition.from}`} VerticalOptions="Fill" Margin={new Thickness(0, 0, 0, rootBottom)} ZIndex={0} BackgroundColor={PageBackgroundColor}
            ref={(c: SkiaControl | null) => { if (c) tabRoots.current.set(tabTransition.from, c); }}>
            {RenderRoute(Tabs![tabTransition.from]?.route ?? "")}
          </SkiaLayer>
        )}
        <SkiaLayer key={hasTabs ? `tab-${selectedTab}` : "root"} VerticalOptions="Fill" Margin={new Thickness(0, 0, 0, rootBottom)} IsVisible={visiblePages.length === 0 || transitioning} BackgroundColor={hasTabs ? PageBackgroundColor : undefined} {...entering}
          ref={(c: SkiaControl | null) => { if (c) tabRoots.current.set(selectedTab, c); }}>
          {rootNode}
        </SkiaLayer>
        {visiblePages.map((r, i) => (
          <PageHost key={`${selectedTab}:${r}#${i}`} route={r} speed={PagesAnimationSpeed} visible={r === topRoute || transitioning} background={PageBackgroundColor} bottom={rootBottom}
            register={(c) => { if (c) pageCtrls.current.set(r, c); else pageCtrls.current.delete(r); }}>
            <SkiaLayer VerticalOptions="Fill" Margin={new Thickness(0, navTop, 0, 0)}>{RenderRoute(r)}</SkiaLayer>
            <SkiaLayer HeightRequest={navTop} BackgroundColor={NavBarColor} Padding={new Thickness(0, insets.Top, 0, 0)}>
              <SkiaButton Text="‹  Back" BackgroundColor="#00000000" TextColor="#6EA8FE" FontSize={16} VerticalOptions="Center" Margin={new Thickness(8, 0)} ApplyEffect="Ripple" Tapped={() => void GoBackAsync()} AccessibilityRole="button" AccessibilityLabel="Back" />
              <SkiaLabel Text={Titles?.[RouteName(r)] ?? RouteName(r)} FontSize={18} FontFamily="FontTextBold" TextColor={Colors.White} HorizontalOptions="Fill" HorizontalTextAlignment="Center" VerticalOptions="Center" MaxLines={1} Margin={new Thickness(96, 0)} AccessibilityRole="heading" />
              <SkiaButton Text="Home" BackgroundColor="#00000000" TextColor="#6EA8FE" FontSize={16} HorizontalOptions="End" VerticalOptions="Center" Margin={new Thickness(0, 0, 8, 0)} ApplyEffect="Ripple" Tapped={() => void PopToRootAsync()} AccessibilityRole="button" AccessibilityLabel="Home" />
              <SkiaLayer HeightRequest={1} VerticalOptions="End" BackgroundColor="#343A40" />
            </SkiaLayer>
          </PageHost>
        ))}
        {hasTabs && (
          <SkiaLayer HeightRequest={TabBarHeight + insets.Bottom} VerticalOptions="End" BackgroundColor={TabBarColor} ZIndex={ShellDefaults.ZIndexModals - 1} BlockGesturesBelow Padding={new Thickness(0, 0, 0, insets.Bottom)}>
            <SkiaLayer HeightRequest={1} BackgroundColor="#343A40" />
            <SkiaGrid ColumnDefinitions={Tabs!.map(() => "*").join(",")} HorizontalOptions="Fill" VerticalOptions="Fill">
              {Tabs!.map((t, i) => (
                <SkiaLayer key={t.route} Column={i} HorizontalOptions="Fill" VerticalOptions="Fill" AnimationTapped="Ripple" Tapped={() => void SelectTabAsync(i)} AccessibilityRole="button" AccessibilityLabel={t.title}>
                  <SkiaLabel Text={t.title} FontSize={13} FontFamily={i === selectedTab ? "FontTextBold" : undefined} TextColor={i === selectedTab ? ShellDefaults.TabSelectedColor : ShellDefaults.TabColor} HorizontalOptions="Center" VerticalOptions="Center" AccessibilityRole="presentation" />
                  {i === selectedTab && <SkiaShape Type="Rectangle" HeightRequest={3} WidthRequest={36} CornerRadius={2} BackgroundColor={ShellDefaults.TabSelectedColor} HorizontalOptions="Center" VerticalOptions="End" />}
                </SkiaLayer>
              ))}
            </SkiaGrid>
          </SkiaLayer>
        )}
        {modals.map((m) => m.node)}
        {popups.map((p) => p.node)}
        {toasts.map((t) => t.node)}
      </SkiaLayer>
    </ShellContext.Provider>
  );
});

/** A pushed page: slides in from the right on mount (C# SkiaViewSwitcher PushView), covers what is below. */
function PageHost({ route, speed, visible, background, bottom, register, children }: { route: string; speed: number; visible: boolean; background: string; bottom: number; register: (c: SkiaControl | null) => void; children: ReactNode }) {
  const ctrl = useRef<SkiaControl | null>(null);
  useEffect(() => {
    const c = ctrl.current;
    if (!c) return;
    register(c);
    if (speed > 0) { c.TranslationX = CanvasWidthPts(c); void c.TranslateToAsync(0, 0, speed); }
    return () => register(null);
  }, [route]); // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <SkiaLayer ref={(c: SkiaControl | null) => { ctrl.current = c; }} VerticalOptions="Fill" HorizontalOptions="Fill" Margin={new Thickness(0, 0, 0, bottom)} BackgroundColor={background} IsVisible={visible} BlockGesturesBelow>
      {children}
    </SkiaLayer>
  );
}
