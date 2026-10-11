/**
 * The question-flow conformance program, built from its sources with the compiler, and the
 * lifecycle, results and origins its `program.json` and `expected/` files record. Only the tests
 * use the compiler: the package itself takes prepared programs.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { NxIrRuntimeError, type NxCanonicalValue, type NxHostValue, type NxOriginEntry, type NxPreparedProgram } from "@nx-lang/ir-runtime";
import { compileNxModule, createNxHost } from "@nx-lang/sdk-wasm";

import { programFromImages, type PreviewImage } from "../src/index.js";

const host = createNxHost(await compileNxModule(readFileSync(fileURLToPath(import.meta.resolve("@nx-lang/sdk-wasm/nx.wasm")))));

const programRoot = new URL("../../../../specs/ir-conformance/question-flow/", import.meta.url);

function readJson(path: string): unknown {
  return JSON.parse(readFileSync(new URL(path, programRoot), "utf8"));
}

interface Manifest {
  readonly entry: string;
  readonly emit: readonly string[];
  readonly lifecycles: readonly {
    readonly module: string;
    readonly component: string;
    readonly props: { readonly [key: string]: NxHostValue };
    readonly batches: readonly (readonly NxHostValue[])[];
  }[];
}

const manifest = readJson("program.json") as Manifest;

/** The identity of the program's entry module, `main.nx`. */
export const entry = manifest.entry;

/** The `Flow` lifecycle: its props, `{ respondent: "friend" }`, and 30 batches, one per question. */
export const lifecycle = manifest.lifecycles[0]!;

const results = readJson("expected/results.json") as {
  readonly [key: string]: {
    readonly initial: NxCanonicalValue;
    readonly batches: readonly { readonly rendered: NxCanonicalValue; readonly effects: readonly NxCanonicalValue[] }[];
  };
};

const origins = readJson("expected/origins.json") as {
  readonly [key: string]: { readonly initial: readonly NxOriginEntry[]; readonly batches: readonly (readonly NxOriginEntry[])[] };
};

/** The rendered output and effects the lifecycle records for its first render and each batch. */
export const expected = results[`${lifecycle.module}::${lifecycle.component}`]!;

/** The origin entries the lifecycle records for its first render and each batch. */
export const expectedOrigins = origins[`${lifecycle.module}::${lifecycle.component}`]!;

/** The source text of each module the program emits, by identity. */
export function sources(): Map<string, string> {
  return new Map(manifest.emit.map((identity) => [identity, readFileSync(new URL(identity, programRoot), "utf8")]));
}

/**
 * The program's images, with their debug sections, built from its sources after `edit` has
 * changed them, if given.
 */
export function images(edit?: (sources: Map<string, string>) => void): PreviewImage[] {
  const modules = sources();
  edit?.(modules);
  const artifact = host.buildWorkspaceArtifact({
    modules: [...modules].map(([identity, source]) => ({ identity, source })),
    entry,
  });
  try {
    return artifact.generateNxIr({ modules: [], debug: true }).map(({ identity, bytes }) => ({ identity, bytes }));
  } finally {
    artifact.dispose();
  }
}

/** The program, linked from {@link images}. */
export function program(edit?: (sources: Map<string, string>) => void): NxPreparedProgram {
  return programFromImages(images(edit), entry);
}

/** Replaces the one occurrence of `from` in a module's source, failing when there is not exactly one. */
export function replaceOnce(modules: Map<string, string>, identity: string, from: string | RegExp, to: string): void {
  const source = modules.get(identity)!;
  const count = typeof from === "string" ? source.split(from).length - 1 : (source.match(new RegExp(from, "g")) ?? []).length;
  if (count !== 1) {
    throw new Error(`Expected one ${String(from)} in ${identity}, found ${count}.`);
  }
  modules.set(identity, source.replace(from, to));
}

/** The JSON pointer of the first `ActionHandler` record for `action` in a rendered output. */
export function handlerFor(rendered: NxCanonicalValue, action: string, pointer = ""): string | undefined {
  if (Array.isArray(rendered)) {
    for (const [index, item] of (rendered as readonly NxCanonicalValue[]).entries()) {
      const found = handlerFor(item, action, `${pointer}/${index}`);
      if (found !== undefined) {
        return found;
      }
    }
  } else if (rendered !== null && typeof rendered === "object") {
    const record = rendered as { readonly [key: string]: NxCanonicalValue };
    if (record.$type === "ActionHandler" && record.action === action) {
      return pointer;
    }
    for (const [key, item] of Object.entries(record)) {
      const found = handlerFor(item, action, `${pointer}/${key}`);
      if (found !== undefined) {
        return found;
      }
    }
  }
  return undefined;
}

/** The action record of a lifecycle batch's only entry. */
export function actionOf(batch: number): { readonly $type: string; readonly [key: string]: NxHostValue } {
  const entry = lifecycle.batches[batch]![0] as { readonly action: { readonly $type: string } };
  return entry.action as { readonly $type: string; readonly [key: string]: NxHostValue };
}

/** The program with `Flow`'s `rating` state field renamed to `score`, which the old state does not fit. */
export function renamedRating(): NxPreparedProgram {
  return program((modules) => {
    const source = modules.get("main.nx")!;
    const at = source.indexOf("component <Flow");
    modules.set("main.nx", source.slice(0, at) + source.slice(at).replace(/\brating\b/g, "score"));
  });
}

/** The error a call fails with, which must be the runtime's error type. */
export function failure(call: () => unknown): NxIrRuntimeError {
  try {
    call();
  } catch (error) {
    assert.ok(error instanceof NxIrRuntimeError, `expected an NxIrRuntimeError, got ${String(error)}`);
    return error;
  }
  assert.fail("expected the call to fail");
}
