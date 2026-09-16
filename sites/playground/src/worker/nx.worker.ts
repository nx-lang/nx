/**
 * The worker the editor compiles and answers language queries in.
 *
 * <para>The module is a hashed, immutable asset like CanvasKit's, streamed and compiled by the
 * browser as it arrives. The catalog is bundled as text, so a compile costs no network at all, and
 * a compile answers with the visitor's module alone: the catalog's own artifact is bundled on the
 * main thread, where the renderer prepares it once. Everything with behaviour is elsewhere and
 * driven by Node tests against the same module: the load and its budget in `module.ts`, the
 * compiler and its recovery in `session.ts`.</para>
 */
import nxModuleUrl from "@nx-lang/sdk-wasm/nx.wasm?url";

import catalog from "../../catalog/skia.nx?raw";
import { loadNxModule } from "./module.ts";
import { toWorkerError, type WorkerMessage, type WorkerRequest } from "./protocol.ts";
import { createNxSession, type NxSession } from "./session.ts";

/** Started at once, so the module is compiling while the editor is still mounting. */
const ready: Promise<NxSession> = loadNxModule(nxModuleUrl).then((module) =>
  createNxSession({ module, catalog })
);

// The main thread holds a request sent before this without a deadline, because the wait is the
// module's download rather than the compiler's work. A load that failed — or that never finished,
// which `module.ts` bounds — needs no notice: the rejection answers every request, the one waiting
// and the ones after it.
void ready.then(
  () => post({ kind: "ready" }),
  () => {}
);

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  void ready.then(
    async (session) => {
      try {
        const value = await session.answer(request);
        // A compiled image is the artifact's own buffer, so it is moved to the main thread rather
        // than copied.
        post({ kind: "ok", id: request.id, value }, transferables(value));
      } catch (error) {
        post({ kind: "error", id: request.id, error: toWorkerError(error) });
      }
    },
    (error: unknown) => {
      post({ kind: "error", id: request.id, error: toWorkerError(error) });
    }
  );
};

function post(message: WorkerMessage, transfer: Transferable[] = []): void {
  self.postMessage(message, { transfer });
}

/** The buffer of a compile result's image, when the answer is one. */
function transferables(value: unknown): Transferable[] {
  const ir = (value as { ir?: unknown } | null)?.ir;
  return ir instanceof Uint8Array && ir.buffer instanceof ArrayBuffer ? [ir.buffer] : [];
}
