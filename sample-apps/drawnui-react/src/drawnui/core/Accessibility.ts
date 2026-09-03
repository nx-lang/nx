import type { SkiaControl } from "./SkiaControl";
import { SKRect } from "./Types";

/** ARIA role constants (DrawnUi.Models.Aria) for `AccessibilityRole`. */
export const Aria = {
  // interactive widgets
  RoleButton: "button", RoleLink: "link", RoleCheckbox: "checkbox", RoleRadio: "radio", RoleSwitch: "switch",
  RoleSlider: "slider", RoleSpinButton: "spinbutton", RoleTextBox: "textbox", RoleSearchBox: "searchbox",
  RoleComboBox: "combobox", RoleListBox: "listbox", RoleOption: "option", RoleTab: "tab", RoleTabPanel: "tabpanel",
  RoleTabList: "tablist", RoleMenu: "menu", RoleMenuItem: "menuitem", RoleMenuItemCheckbox: "menuitemcheckbox",
  RoleMenuItemRadio: "menuitemradio", RoleScrollBar: "scrollbar",
  // structure & landmarks
  RoleText: "text", RoleHeading: "heading", RoleImg: "img", RoleList: "list", RoleListItem: "listitem",
  RoleSeparator: "separator", RoleProgressBar: "progressbar", RoleTooltip: "tooltip", RoleDialog: "dialog",
  RoleAlertDialog: "alertdialog", RoleStatus: "status", RoleAlert: "alert", RoleGroup: "group", RoleRegion: "region",
  RoleNavigation: "navigation", RoleMain: "main",
  /** Removes the control from the accessibility tree even when a default role would apply (inner label of a button). */
  RolePresentation: "presentation",
  // live regions
  LivePolite: "polite", LiveAssertive: "assertive",
} as const;

/** One entry of the accessibility snapshot (DrawnUi AccessibilityNode). Rect is CSS pixels relative to the canvas. */
export interface AccessibilityNode {
  Id: number;
  Label?: string;
  Hint?: string;
  Role: string;
  Rect: SKRect;
  CanInteract: boolean;
  IsPressed?: boolean;
  Live?: string;
  Source: SkiaControl;
}

/**
 * Mirrors DrawnUi SkiaAccessibilityManager: registry of accessible controls + a rate-limited snapshot
 * (at most one rebuild per MinUpdateIntervalMs, taken at frame end) that the DOM overlay renders.
 * Detached / hidden / far-off-canvas controls drop out of the snapshot on the next rebuild, so nothing has
 * to unregister explicitly; positions follow scrolling because rects are read from the arranged DrawingRect.
 */
export class SkiaAccessibilityManager {
  private readonly nodes = new Set<SkiaControl>();
  private dirty = false;
  private lastRebuild = -Infinity;
  private pending = 0;
  private readonly changed = new Set<() => void>();
  private readonly liveUpdated = new Set<(node: SkiaControl) => void>();

  /** Minimum milliseconds between snapshot rebuilds. */
  MinUpdateIntervalMs = 1000;
  Snapshot: AccessibilityNode[] = [];
  FocusedNode?: SkiaControl;

  /** Subscribes to snapshot changes; returns the unsubscribe function. */
  OnChanged(cb: () => void): () => void { this.changed.add(cb); return () => this.changed.delete(cb); }
  /** Fired immediately (bypassing the rate limit) when a live-region node's value changes. */
  OnLiveRegionUpdated(cb: (node: SkiaControl) => void): () => void { this.liveUpdated.add(cb); return () => this.liveUpdated.delete(cb); }

  NotifyFocused(node?: SkiaControl): void { if (this.FocusedNode !== node) this.FocusedNode = node; }

  Register(node: SkiaControl): void { this.nodes.add(node); this.dirty = true; }

  NotifyUpdated(node: SkiaControl): void {
    if (!this.nodes.has(node)) return;
    this.dirty = true;
    if (node.AccessibilityLive) for (const cb of this.liveUpdated) cb(node);
  }

  ForceRebuildOnNextFrame(): void { this.lastRebuild = 0; this.dirty = true; }

  Unregister(node: SkiaControl): void {
    if (this.nodes.delete(node)) { node.OnAccessibilityUnregistered(); this.dirty = true; }
  }

  UnregisterSubtree(root: SkiaControl): void {
    for (const n of Array.from(this.nodes)) {
      let c: SkiaControl | undefined = n;
      while (c && c !== root) c = c.Parent;
      if (c) this.Unregister(n);
    }
  }

  /**
   * Called by Canvas at the end of every drawn frame. Rebuilds when something was invalidated, and at most once
   * per MinUpdateIntervalMs otherwise (keeps rects in sync with scrolling); notifies only when the snapshot differs.
   */
  OnFrameEnd(scale: number, canvasWidthPx: number, canvasHeightPx: number, requestFrame: () => void): void {
    if (this.nodes.size === 0 && this.Snapshot.length === 0) return;
    const now = performance.now();
    const wait = this.MinUpdateIntervalMs - (now - this.lastRebuild);
    if (wait > 0) {
      // rate-limited: rendering is on demand, so make sure a frame comes back once the interval has passed
      if (!this.pending) this.pending = window.setTimeout(() => { this.pending = 0; requestFrame(); }, wait);
      return;
    }
    this.dirty = false;
    this.lastRebuild = now;

    const list: AccessibilityNode[] = [];
    for (const n of this.nodes) {
      if (!n.Superview) { this.nodes.delete(n); n.OnAccessibilityUnregistered(); continue; } // detached from the tree
      if (!n.IsVisible || !n.IsAccessibilityElement || n.AccessibilityRole === Aria.RolePresentation) continue;
      const px = n.GetAccessibilityPixelRect();
      if (px.Width <= 0 || px.Height <= 0) continue;
      // nodes beyond the canvas stay in the tree (scroll content reachable with Tab, the overlay scrolls them into view);
      // only what is entirely outside a clipped canvas by more than a screen is dropped to keep the DOM small
      if (px.Right < -canvasWidthPx || px.Bottom < -canvasHeightPx || px.Left > 2 * canvasWidthPx || px.Top > 2 * canvasHeightPx) continue;
      list.push({
        Id: n.AccessibilityId, Label: n.AccessibilityLabel, Hint: n.AccessibilityHint, Role: n.AccessibilityRole!,
        Rect: new SKRect(px.Left / scale, px.Top / scale, px.Right / scale, px.Bottom / scale),
        CanInteract: n.AccessibilityCanInteract, IsPressed: n.AccessibilityIsPressed, Live: n.AccessibilityLive, Source: n,
      });
    }
    list.sort((a, b) => a.Rect.Top - b.Rect.Top || a.Rect.Left - b.Rect.Left);
    if (SkiaAccessibilityManager.Same(list, this.Snapshot)) return;
    this.Snapshot = list;
    for (const cb of this.changed) cb();
  }

  private static Same(a: AccessibilityNode[], b: AccessibilityNode[]): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      const x = a[i], y = b[i];
      if (x.Source !== y.Source || x.Label !== y.Label || x.Hint !== y.Hint || x.Role !== y.Role || x.CanInteract !== y.CanInteract
        || x.IsPressed !== y.IsPressed || x.Live !== y.Live
        || Math.abs(x.Rect.Left - y.Rect.Left) > 0.5 || Math.abs(x.Rect.Top - y.Rect.Top) > 0.5
        || Math.abs(x.Rect.Right - y.Rect.Right) > 0.5 || Math.abs(x.Rect.Bottom - y.Rect.Bottom) > 0.5) return false;
    }
    return true;
  }
}
