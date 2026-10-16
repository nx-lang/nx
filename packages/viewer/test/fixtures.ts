/**
 * Source trees as the language service answers them: each entry of `test/fixtures/sources.json`
 * through `@nx-lang/sdk-wasm`'s language snapshot, computed when the tests load, so the tests
 * exercise the trees the service really produces. An entry names repository files, the first being
 * the document shown, or gives a source of its own.
 */
import { readFileSync } from "node:fs";

import type { SourceTree } from "@nx-lang/language-protocol";
import {
  createNxHost,
  loadNxModule,
  type NxHost,
  type NxLanguageDocumentInput,
  type SourceTreeOptions
} from "@nx-lang/sdk-wasm";

/** The repository's root, from `dist/test`. */
export const repository = new URL("../../../../", import.meta.url);

/** A document's tree and its text. */
export interface Fixture {
  readonly text: string;
  readonly tree: SourceTree;
}

interface Entry {
  readonly files?: readonly string[];
  readonly source?: string;
  /** Further documents analyzed with the first, by identity. */
  readonly with?: Readonly<Record<string, string>>;
  /** The host visualizer modules to resolve visualizers with; visualizers are resolved when this is set. */
  readonly visualizers?: readonly string[];
}

const entries = JSON.parse(
  readFileSync(new URL("../../test/fixtures/sources.json", import.meta.url), "utf8")
) as Record<string, Entry>;

/** The name of every fixture. */
export const fixtureNames: readonly string[] = Object.keys(entries);

let host: NxHost | undefined;

/** The wasm host every fixture is computed with, created once. */
export async function nxHost(): Promise<NxHost> {
  host ??= createNxHost(await loadNxModule());
  return host;
}

/** The tree of the first of `documents`, analyzed together, asked for with `options`. */
export async function treeOf(
  documents: readonly NxLanguageDocumentInput[],
  options?: SourceTreeOptions
): Promise<Fixture> {
  const snapshot = (await nxHost()).createLanguageSnapshot(documents);
  try {
    const first = documents[0]!;
    return { text: first.source, tree: snapshot.sourceTree(first.uri, options) };
  } finally {
    snapshot.dispose();
  }
}

const computed = new Map<string, Promise<Fixture>>();

/** The fixture named `name` in `test/fixtures/sources.json`, computed once. */
export function fixture(name: string): Promise<Fixture> {
  let found = computed.get(name);
  if (found === undefined) {
    found = compute(name);
    computed.set(name, found);
  }
  return found;
}

async function compute(name: string): Promise<Fixture> {
  const entry = entries[name];
  if (entry === undefined) {
    throw new Error(`No fixture '${name}' in test/fixtures/sources.json.`);
  }
  const options = entry.visualizers === undefined ? undefined : { visualizers: { modules: [...entry.visualizers] } };
  const others = Object.entries(entry.with ?? {}).map(([identity, source]) => ({
    uri: `nx:///${identity}`,
    identity,
    source
  }));
  if (entry.source !== undefined) {
    return treeOf([{ uri: `nx:///${name}.nx`, identity: `${name}.nx`, source: entry.source }, ...others], options);
  }
  return treeOf(
    [
      ...(entry.files ?? []).map((path) => ({
        uri: `nx:///${path}`,
        identity: path,
        source: readFileSync(new URL(path, repository), "utf8")
      })),
      ...others
    ],
    options
  );
}
