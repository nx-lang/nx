import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import type { NxJsonSchema } from "../src/types.js";

const corpusDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures", "schema");

/**
 * What the Rust export answers for one corpus function: `crates/nx-api` writes these with
 * `NX_UPDATE_SCHEMA_CORPUS=1`.
 */
export interface GoldenFunction {
  readonly inputSchema?: NxJsonSchema;
  readonly outputSchema?: NxJsonSchema;
  readonly diagnostics: readonly string[];
}

/**
 * Hand-written arguments for one corpus function, each valid against its input schema, with what
 * the call is expected to return when the result is worth pinning.
 */
export interface ArgumentCase {
  readonly args: Record<string, unknown>;
  readonly result?: unknown;
}

export interface CorpusModule {
  readonly identity: string;
  readonly source: string;
  readonly golden: Readonly<Record<string, GoldenFunction>>;
  readonly args: Readonly<Record<string, readonly ArgumentCase[]>>;
}

/**
 * The schema corpus: one NX source per mapping, with its golden documents and argument cases.
 */
export const corpus: readonly CorpusModule[] = readdirSync(corpusDir)
  .filter((file) => file.endsWith(".nx"))
  .sort()
  .map((identity) => {
    const base = identity.slice(0, -".nx".length);
    const argsPath = path.join(corpusDir, `${base}.args.json`);
    return {
      identity,
      source: readFileSync(path.join(corpusDir, identity), "utf8"),
      golden: JSON.parse(readFileSync(path.join(corpusDir, `${base}.schema.json`), "utf8")) as Record<
        string,
        GoldenFunction
      >,
      args: readdirSync(corpusDir).includes(`${base}.args.json`)
        ? (JSON.parse(readFileSync(argsPath, "utf8")) as Record<string, ArgumentCase[]>)
        : {}
    };
  });
