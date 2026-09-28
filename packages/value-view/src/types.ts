/**
 * A value spelled as NX text, with a node for each value, property and sequence in it: what
 * `NxProgramArtifact.evaluateNx()` in `@nx-lang/sdk-wasm` returns.
 *
 * <para>Declared here structurally, rather than imported, so a host can show a value without
 * depending on the compiler package. The SDK's type is assignable to it.</para>
 */
export interface NxValueText {
  /** The value's NX spelling. */
  readonly text: string;
  /** What each part of `text` is, in text order, each parent before its children. */
  readonly nodes: readonly NxValueNode[];
}

/**
 * What kind of thing an {@link NxValueNode} is: a record, one `name=value` property, a non-empty
 * sequence, a constant union case, a number, string or boolean, a function value, or `{}`.
 */
export type NxValueRole =
  | "record"
  | "property"
  | "sequence"
  | "case"
  | "scalar"
  | "function"
  | "empty";

/** What one range of an {@link NxValueText}'s text is. */
export interface NxValueNode {
  /** First UTF-16 code unit of the node, so `text.slice(start, end)` is the node. */
  readonly start: number;
  /** UTF-16 code unit one past the node. */
  readonly end: number;
  /** Index of the enclosing node, absent at the top. */
  readonly parent?: number;
  /** What kind of thing the node is. */
  readonly role: NxValueRole;
  /** The type in NX, `Task` or `string` or `Task*`; a property's declared type. */
  readonly type?: string;
  /** A property's name, or a function value's. */
  readonly name?: string;
  /** Whether a property is declared optional. */
  readonly optional?: boolean;
  /** A sequence's length. */
  readonly count?: number;
  /** Where the program declares what the node is, if it does. */
  readonly declaration?: NxValueSpan;
}

/**
 * A span of the program's source: byte offsets, and 1-based lines and columns counted in Unicode
 * scalar values, as NX diagnostics give them.
 */
export interface NxValueSpan {
  readonly startByte: number;
  readonly endByte: number;
  readonly startLine: number;
  readonly startColumn: number;
  readonly endLine: number;
  readonly endColumn: number;
}
