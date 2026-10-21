/**
 * The pieces of the reading's editors: where in a value's rendering its text sits, the value shown
 * once an edit is made and before the host answers, a number's steps, and the popovers' content.
 */
import type { SourceCase } from "@nx-lang/language-protocol";

import { isNumericLiteral, type EditTarget } from "./edit.js";

/** A boolean's mark, as the reading draws it. */
export const BOOLEAN_MARKS = { true: "✓", false: "✗" } as const;

/**
 * Where a value's text sits in its rendering: the element holding it, the quotes drawn around it
 * there, and what else it holds that stays, as a color's swatch.
 */
export interface ValueHost {
  readonly host: HTMLElement;
  readonly lead: Node | undefined;
  readonly trail: Node | undefined;
  readonly keep: readonly Element[];
}

/**
 * Where the text of the value drawn by `span`, written as `written`, sits: in its words' box for UI
 * text, or in the span itself, with the quotes the reading draws, kept as they are drawn.
 */
export function valueHost(span: HTMLElement, written: string): ValueHost {
  const document = span.ownerDocument;
  const host = span.classList.contains("uitext-body") ? span : (span.querySelector<HTMLElement>(".uitext-body") ?? span);
  const keep = [...host.children].filter((child) => child.classList.contains("swatch"));
  // Which quotes the reading draws around the text is found once, from the rendering as drawn, and
  // kept on it, since a value shown since no longer reads as written.
  let quotes = span.dataset["quotes"];
  if (quotes === undefined) {
    const shown = host.textContent ?? "";
    const quoted = written.length >= 2 && written.startsWith('"') && written.endsWith('"');
    quotes = quoted && shown === written ? "both" : quoted && shown === written.slice(1) ? "trail" : "none";
    span.dataset["quotes"] = quotes;
  }
  const both = quotes === "both";
  const trailing = both || quotes === "trail";
  const quote = (node: ChildNode | null): Node =>
    node !== null && node.nodeType === 1 && /quote/.test((node as Element).className)
      ? node
      : document.createTextNode('"');
  return {
    host,
    lead: both ? quote(host.firstChild) : undefined,
    trail: trailing ? quote(host.lastChild) : undefined,
    keep,
  };
}

/**
 * Shows `value` in place of what `span` draws for `target`, muted as waiting for the host: a mark
 * for a boolean, a case's name after its qualifier, and otherwise the text between the quotes the
 * reading draws.
 */
export function showValue(span: HTMLElement, target: EditTarget, written: string, value: string): void {
  span.classList.add("pending");
  if (target.kind === "boolean") {
    const on = value === "true";
    span.textContent = on ? BOOLEAN_MARKS.true : BOOLEAN_MARKS.false;
    span.classList.toggle("true", on);
    span.classList.toggle("false", !on);
    span.title = value;
    span.setAttribute("aria-label", value);
    return;
  }
  if (target.kind === "case") {
    const shown = span.textContent ?? "";
    const dot = shown.lastIndexOf(".");
    span.replaceChildren(`${dot < 0 ? "" : shown.slice(0, dot + 1)}${value}`);
    return;
  }
  const { host, lead, trail, keep } = valueHost(span, written);
  for (const swatch of keep) {
    (swatch as HTMLElement).style.backgroundColor = value.trim();
  }
  host.replaceChildren(...keep, ...(lead === undefined ? [] : [lead]), value, ...(trail === undefined ? [] : [trail]));
}

/**
 * `text`, a numeric literal, moved by `steps`: an integer stays one, and a real keeps as many
 * decimals as it has; unless `signed`, it stops at zero. Nothing when `text` is not a decimal number.
 */
export function stepNumber(text: string, steps: number, signed = true): string | undefined {
  const trimmed = text.trim();
  if (!isNumericLiteral(trimmed) || /[xXeE]/.test(trimmed)) {
    return undefined;
  }
  const decimals = trimmed.includes(".") ? trimmed.length - trimmed.indexOf(".") - 1 : 0;
  // Where a number cannot be negative, it stops at zero.
  const next = signed ? Number(trimmed) + steps : Math.max(0, Number(trimmed) + steps);
  return decimals === 0 ? String(Math.round(next)) : next.toFixed(decimals);
}

/** A doc comment's first paragraph as plain text, for one line under a case's name. */
export function firstParagraph(doc: string | undefined): string {
  if (doc === undefined) {
    return "";
  }
  const paragraph = doc.trim().split(/\n\s*\n/)[0] ?? "";
  return paragraph.replace(/[`*_]/g, "").replace(/\s+/g, " ").trim();
}

/** The options of a case's popover: each case's name, its doc's first paragraph, the current marked. */
export function caseOptions(document: Document, cases: readonly SourceCase[], current: string): HTMLElement[] {
  return cases.map((option) => {
    const row = document.createElement("div");
    row.className = option.name === current ? "option current" : "option";
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(option.name === current));
    row.dataset["name"] = option.name;
    const name = document.createElement("span");
    name.className = "option-name";
    name.textContent = option.name;
    row.append(name);
    const words = firstParagraph(option.doc);
    if (words !== "") {
      const doc = document.createElement("span");
      doc.className = "option-doc";
      doc.textContent = words;
      row.append(doc);
    }
    return row;
  });
}

/**
 * The color `text` names as `#rrggbb`, for the browser's color picker: a hex color as it is, less
 * any alpha, or the color `fallback` is drawn in, or black.
 */
export function pickerColor(text: string, fallback: Element | undefined): string {
  const hex = text.trim();
  if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(hex)) {
    return hex.slice(0, 7).toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3,4}$/.test(hex)) {
    return `#${[...hex.slice(1, 4)].map((digit) => digit + digit).join("")}`.toLowerCase();
  }
  const drawn = fallback === undefined ? "" : getComputedStyle(fallback).backgroundColor;
  const rgb = /^rgba?\((\d+)[, ]+(\d+)[, ]+(\d+)/.exec(drawn);
  if (rgb !== null) {
    return `#${rgb.slice(1, 4).map((part) => Number(part).toString(16).padStart(2, "0")).join("")}`;
  }
  return "#000000";
}
