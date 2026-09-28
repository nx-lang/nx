import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Worker } from "node:worker_threads";

import { describe, expect, it } from "vitest";

/**
 * Recursion under a browser-sized stack.
 *
 * <para>A browser worker's native stack is far smaller than Node's main thread, which is where the
 * other tests run, so a recursion limit that holds there can still trap in a browser. These run the
 * module in a Node worker whose stack is smaller than a browser worker's, and require every shape
 * the limit was measured with to end in the interpreter's error rather than a trap.</para>
 */

/**
 * The worker's stack, in megabytes. With the interpreter's default limit, a countdown recursion
 * traps about 300 calls deep in a 0.5 MB Node worker, against 517 in a Chromium dedicated worker, so
 * this is the stricter of the two.
 */
const STACK_MB = 0.5;
const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const entry = pathToFileURL(path.join(packageRoot, "dist", "src", "node.js")).href;

/** The deepest a call may nest in the module, from `MAX_RECURSION_DEPTH` in the native crate. */
const LIMIT = 200;

const shapes: Record<string, (depth: number) => string> = {
  countdown: (n) => `let f(n:int): int = { if n == 0 { 0 } else { f(n - 1) } }\nlet root() = { f(${n}) }`,
  sum: (n) => `let f(n:int): int = { if n == 0 { 0 } else { n + f(n - 1) } }\nlet root() = { f(${n}) }`,
  element: (n) =>
    `type Node = { label:string children?:Node+ }\nlet <Tree depth:int />: Node = { if depth == 0 { <Node label="leaf" /> } else { <Node label={"n" + depth} children={ <Tree depth={depth - 1} /> } /> } }\nlet root() = <Tree depth=${n} />`,
  forLoop: (n) => `let f(n:int): int* = { if n == 0 { {} } else { for i in 0..1 { f(n - 1) } } }\nlet root() = { f(${n}) }`
};

/** Evaluates each source on a fresh host in a worker with `stackSizeMb`, and says how each ended. */
function evaluateInWorker(sources: string[], stackSizeMb: number): Promise<string[]> {
  const code = `
    const { parentPort, workerData } = require("node:worker_threads");
    (async () => {
      const { createNxHost, loadNxModule } = await import(workerData.entry);
      const module = await loadNxModule();
      const results = [];
      for (const source of workerData.sources) {
        const host = createNxHost(module);
        try {
          const artifact = host.buildProgramArtifact(source);
          artifact.evaluateNx();
          results.push("value");
        } catch (error) {
          results.push(host.crashed ? "trap" : "error: " + error.message);
        }
      }
      parentPort.postMessage(results);
    })().catch((error) => parentPort.postMessage(["failed: " + error.message]));
  `;
  return new Promise((resolve, reject) => {
    const worker = new Worker(code, {
      eval: true,
      workerData: { entry, sources },
      resourceLimits: { stackSizeMb }
    });
    worker.once("message", (results: string[]) => {
      void worker.terminate();
      resolve(results);
    });
    worker.once("error", reject);
  });
}

describe("recursion under a browser worker's stack", () => {
  it("ends at the recursion limit with the interpreter's error, for every measured shape", async () => {
    const names = Object.keys(shapes);
    const results = await evaluateInWorker(
      names.flatMap((name) => [shapes[name]!(LIMIT - 1), shapes[name]!(LIMIT + 50)]),
      STACK_MB
    );
    names.forEach((name, index) => {
      expect(results[2 * index], `${name} within the limit`).toBe("value");
      expect(results[2 * index + 1], `${name} past the limit`).toMatch(/^error: Stack overflow: recursion depth 200 exceeded/);
    });
  });

  it("ends runaway recursion with the interpreter's error", async () => {
    const [result] = await evaluateInWorker(["let f(n:int): int = { f(n + 1) }\nlet root() = { f(0) }"], STACK_MB);
    expect(result).toMatch(/^error: Stack overflow/);
  });
});
