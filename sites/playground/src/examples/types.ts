/** One example: a short program that shows one part of the language, and where the docs explain it. */
export interface Example {
  /** The id in its address, `/play/<id>`. */
  readonly id: string;
  /** What the examples drop-down calls it. */
  readonly title: string;
  /** The group it is listed under. */
  readonly topic: string;
  /** The website page that explains it, as a path under the docs root with an optional anchor. */
  readonly docs: string;
  /** The NX source. */
  readonly source: string;
}

/** Examples grouped by topic, in the order they are listed. */
export interface ExampleGroup {
  readonly topic: string;
  readonly examples: readonly Example[];
}
