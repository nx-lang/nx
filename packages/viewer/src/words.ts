/**
 * The viewer's wording: kinds in words, handler names as events, and the operator list. Each
 * reading stands for one construct, so the reading stays one-to-one with the source.
 */
import type { SourceDeclarationKind } from "@nx-lang/language-protocol";

/**
 * A name split at its capitals into sentence case: `SingleChoice` reads "Single choice",
 * `URLField` reads "URL field", and each segment of `User.Update` is split the same way. A run of
 * capitals stays as it is, so an acronym keeps its case.
 */
export function inWords(name: string): string {
  const words = name
    .split(".")
    .flatMap((segment) => segment.match(/[A-Z]+(?![a-z])|[A-Z]?[a-z]+|\d+|[^A-Za-z\d]+/g) ?? [segment]);
  return words
    .map((word, position) => {
      if (word.length > 1 && word === word.toUpperCase()) {
        return word;
      }
      return position === 0 ? word.charAt(0).toUpperCase() + word.slice(1) : word.toLowerCase();
    })
    .join(" ");
}

/** A handler's name as an event: `onIntegerAnswered` reads "integer answered". */
export function eventWords(name: string): string {
  const event = /^on[A-Z]/.test(name) ? name.slice(2) : name;
  return inWords(event).toLowerCase();
}

/** How a declaration kind is named in a declaration's header. */
export const KIND_LABELS: Readonly<Record<SourceDeclarationKind, string>> = {
  record: "Type",
  action: "Action",
  union: "One of",
  alias: "Type",
  component: "Component",
  function: "Function",
  value: "Value",
};

/** A binary operator's reading: the symbol between its operands, or a phrase around them. */
export type BinaryReading =
  | { readonly kind: "infix"; readonly symbol: string }
  | { readonly kind: "around"; readonly before: string; readonly between: string };

/** The binary operators the viewer reads, by token. */
export const BINARY_READINGS: ReadonlyMap<string, BinaryReading> = new Map<string, BinaryReading>([
  ["==", { kind: "infix", symbol: "=" }],
  ["!=", { kind: "infix", symbol: "≠" }],
  ["<", { kind: "infix", symbol: "<" }],
  [">", { kind: "infix", symbol: ">" }],
  ["<=", { kind: "infix", symbol: "≤" }],
  [">=", { kind: "infix", symbol: "≥" }],
  ["+", { kind: "infix", symbol: "+" }],
  ["-", { kind: "infix", symbol: "−" }],
  ["*", { kind: "infix", symbol: "×" }],
  ["/", { kind: "infix", symbol: "÷" }],
  ["%", { kind: "around", before: "remainder of", between: "÷" }],
  ["&&", { kind: "infix", symbol: "and" }],
  ["||", { kind: "infix", symbol: "or" }],
  ["??", { kind: "infix", symbol: ", otherwise" }],
  ["..", { kind: "infix", symbol: "up to" }],
  ["..=", { kind: "infix", symbol: "through" }],
]);

/** The prefix operators the viewer reads, by token. */
export const PREFIX_READINGS: ReadonlyMap<string, string> = new Map([
  ["-", "−"],
  ["!", "not"],
]);

/** The postfix operators the viewer reads, by token: `x?` reads "x is given". */
export const POSTFIX_READINGS: ReadonlyMap<string, string> = new Map([["?", "is given"]]);
