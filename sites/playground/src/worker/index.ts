/**
 * The one compiler worker the app runs, and the channel to it.
 *
 * <para>One worker serves the whole session: compilation and language queries share a host, so a
 * hover costs no second copy of the module and no second analysis of the same text. It is started
 * when the editor view mounts rather than on the gallery, so the module's download does not precede
 * the first paint of a page that never compiles.</para>
 */
import { createWorkerChannel, type WorkerChannel } from "./channel.ts";

let channel: WorkerChannel | null = null;

/** The channel, starting the worker the first time it is asked for. */
export function nxWorkerChannel(): WorkerChannel {
  channel ??= createWorkerChannel({
    startWorker: () =>
      new Worker(new URL("./nx.worker.ts", import.meta.url), {
        type: "module",
        name: "nx-compiler"
      })
  });
  return channel;
}

/** Starts the worker so the module is compiling before the first compile is asked for. */
export function startNxWorker(): void {
  nxWorkerChannel().start();
}
