/**
 * The examples: short programs, one or two per topic, that give a visitor something to start from.
 *
 * <para>Each is `nx/<id>.nx`, with what its `root` prints committed beside it as `nx/<id>.out.nx`.
 * `scripts/check-examples.mjs` evaluates every one through the module the site ships and fails when
 * an output drifts, so an example cannot rot.</para>
 */
import metadata from "./examples.json";
import type { Example, ExampleGroup } from "./types";

const sources = import.meta.glob<string>(["./nx/*.nx", "!./nx/*.out.nx"], {
  query: "?raw",
  import: "default",
  eager: true
});

/** Every example, in the order the drop-down lists them. */
export const EXAMPLES: readonly Example[] = metadata.map((entry) => {
  const source = sources[`./nx/${entry.id}.nx`];
  if (source === undefined) {
    throw new Error(`Example '${entry.id}' has metadata but no NX source.`);
  }
  return { ...entry, source };
});

/** The example the playground opens with. */
export const DEFAULT_EXAMPLE: Example = EXAMPLES.find((example) => example.id === "hello") ?? EXAMPLES[0]!;

/** The examples grouped by topic, each group in the order its first example appears. */
export const EXAMPLE_GROUPS: readonly ExampleGroup[] = EXAMPLES.reduce<ExampleGroup[]>((groups, example) => {
  const group = groups.find((candidate) => candidate.topic === example.topic);
  if (group === undefined) {
    groups.push({ topic: example.topic, examples: [example] });
  } else {
    (group.examples as Example[]).push(example);
  }
  return groups;
}, []);

export function exampleById(id: string): Example | undefined {
  return EXAMPLES.find((example) => example.id === id);
}

/** The example whose source is exactly `source`, if any. */
export function exampleWithSource(source: string): Example | undefined {
  return EXAMPLES.find((example) => example.source === source);
}

/** The website address of an example's docs page. */
export function docsUrl(example: Example): string {
  const [page, anchor] = example.docs.split("#");
  return `/${page}/${anchor === undefined ? "" : `#${anchor}`}`;
}

export type { Example, ExampleGroup } from "./types";
