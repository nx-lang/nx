import type { SkiaEditor } from "../controls/SkiaEditor";

/**
 * Hidden DOM textarea standing in for the focused SkiaEditor so IME composition, mobile soft keyboards,
 * autocorrect, native copy / cut / paste and undo reach the drawn editor. Not part of DrawnUi.Blazor (which has no
 * DOM input): the textarea mirrors the editor's text + selection, and every `input` event is diffed against the
 * editor and applied through the same stub methods the physical keyboard uses (`StubSelectRange` + `StubTypeText`
 * / `StubBackspace`, a line break through `StubPressEnter`). Keys that the textarea turns into input events
 * (characters, Backspace, Delete, Enter, clipboard shortcuts) are skipped by the KeyboardManager path while the
 * proxy is active; navigation keys keep going through `KeyDown`.
 */
export class TextInputProxy {
  private static element?: HTMLTextAreaElement;
  private static owner?: SkiaEditor;
  private static composing = false;
  private static applying = false;

  /** The textarea (created on first use); KeyboardManager recognizes it through `data-drawnui-text-input`. */
  static get Element(): HTMLTextAreaElement | undefined { return TextInputProxy.element; }
  static IsActive(editor: SkiaEditor): boolean { return TextInputProxy.owner === editor; }
  static IsProxyTarget(target: EventTarget | null): boolean { return !!target && target === TextInputProxy.element; }

  private static Create(): HTMLTextAreaElement | undefined {
    if (typeof document === "undefined") return undefined;
    const el = document.createElement("textarea");
    el.dataset.drawnuiTextInput = "1";
    el.setAttribute("aria-hidden", "true");
    el.tabIndex = -1;
    el.autocomplete = "off";
    el.setAttribute("autocorrect", "off");
    el.setAttribute("autocapitalize", "off");
    el.rows = 1;
    // fixed 1x1 box over the caret: invisible, still focusable, 16px font keeps iOS from zooming the page
    el.style.cssText = "position:fixed;left:0;top:0;width:1px;height:1px;padding:0;border:0;margin:0;opacity:0.01;font-size:16px;line-height:1;resize:none;overflow:hidden;outline:none;background:transparent;color:transparent;caret-color:transparent;pointer-events:none;z-index:-1;";
    el.addEventListener("compositionstart", () => { TextInputProxy.composing = true; });
    el.addEventListener("compositionend", () => { TextInputProxy.composing = false; TextInputProxy.ApplyFromElement(); });
    el.addEventListener("input", () => TextInputProxy.ApplyFromElement());
    el.addEventListener("blur", () => {
      // focus moved to another DOM text field (a page input): the drawn editor loses focus like C# BlurExternalTextInput
      queueMicrotask(() => {
        const a = document.activeElement as HTMLElement | null;
        const owner = TextInputProxy.owner;
        if (owner && a && a !== el && (a.tagName === "INPUT" || a.tagName === "TEXTAREA" || a.isContentEditable)) owner.IsFocused = false;
      });
    });
    // a tap on the canvas moves DOM focus to the body: give it back to the proxy synchronously (keeps the soft keyboard)
    window.addEventListener("pointerup", (e) => {
      const owner = TextInputProxy.owner;
      if (owner && owner.IsFocused && (e.target as HTMLElement | null)?.tagName === "CANVAS") el.focus({ preventScroll: true });
    }, true);
    document.body.appendChild(el);
    return el;
  }

  /** The editor took focus: the textarea mirrors it and gets DOM focus (soft keyboard, IME). */
  static Attach(editor: SkiaEditor): void {
    const el = TextInputProxy.element ?? (TextInputProxy.element = TextInputProxy.Create());
    if (!el) return;
    TextInputProxy.owner = editor;
    TextInputProxy.composing = false;
    TextInputProxy.Configure(editor, el);
    TextInputProxy.Sync(editor);
    el.focus({ preventScroll: true });
  }

  static Detach(editor: SkiaEditor): void {
    if (TextInputProxy.owner !== editor) return;
    TextInputProxy.owner = undefined;
    const el = TextInputProxy.element;
    if (el && document.activeElement === el) el.blur();
  }

  private static Configure(editor: SkiaEditor, el: HTMLTextAreaElement): void {
    const modes: Record<string, string> = { Default: "text", Numeric: "numeric", Decimal: "decimal", Phone: "tel", Email: "email" };
    el.inputMode = modes[editor.KeyboardType] ?? "text";
    const hints: Record<string, string> = { Done: "done", Go: "go", Next: "next", Search: "search", Send: "send" };
    el.enterKeyHint = hints[editor.ReturnType] ?? (editor.IsMultiline ? "enter" : "done");
    el.spellcheck = editor.IsSpellCheckEnabled;
  }

  /** Editor text / selection changed: mirror it (never while an IME composition is in flight). */
  static Sync(editor: SkiaEditor): void {
    const el = TextInputProxy.element;
    if (!el || TextInputProxy.owner !== editor || TextInputProxy.composing || TextInputProxy.applying) return;
    if (el.value !== editor.Text) el.value = editor.Text;
    const start = editor.CursorPosition, end = start + editor.SelectionLength;
    if (el.selectionStart !== start || el.selectionEnd !== end) el.setSelectionRange(start, end);
    TextInputProxy.Place(editor, el);
  }

  /** Puts the textarea over the editor so mobile browsers scroll the right spot into view. */
  private static Place(editor: SkiaEditor, el: HTMLTextAreaElement): void {
    const canvas = editor.Superview?.Element;
    if (!canvas) return;
    const b = canvas.getBoundingClientRect();
    const scale = editor.RenderingScale || 1;
    const r = editor.DrawingRect;
    el.style.left = `${Math.round(b.left + r.Left / scale)}px`;
    el.style.top = `${Math.round(b.top + r.Top / scale)}px`;
  }

  /** The textarea changed (typing, IME commit, paste, undo): diff against the editor and replay through the stubs. */
  private static ApplyFromElement(): void {
    const el = TextInputProxy.element, editor = TextInputProxy.owner;
    if (!el || !editor || TextInputProxy.composing) return;
    const next = el.value, prev = editor.Text;
    if (next === prev) { TextInputProxy.Sync(editor); return; }
    let start = 0;
    const max = Math.min(prev.length, next.length);
    while (start < max && prev.charCodeAt(start) === next.charCodeAt(start)) start++;
    let endPrev = prev.length, endNext = next.length;
    while (endPrev > start && endNext > start && prev.charCodeAt(endPrev - 1) === next.charCodeAt(endNext - 1)) { endPrev--; endNext--; }
    const inserted = next.slice(start, endNext);
    TextInputProxy.applying = true;
    try {
      editor.StubSelectRange(start, endPrev - start);
      if (inserted.includes("\n") && (!editor.IsMultiline || editor.ShouldSubmitOnEnter)) {
        // Enter on a single-line editor (or a Send editor): submit instead of inserting the break
        const text = inserted.replace(/\n/g, "");
        if (text) editor.StubTypeText(text); else if (endPrev > start) editor.StubBackspace();
        editor.StubPressEnter(false, false);
      } else if (inserted) editor.StubTypeText(inserted);
      else editor.StubBackspace();
    } finally { TextInputProxy.applying = false; }
    TextInputProxy.Sync(editor);
  }
}
