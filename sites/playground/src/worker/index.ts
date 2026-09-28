/**
 * The one compiler worker the app runs, and the channel to it.
 *
 * <para>One worker serves the whole session: evaluation and language queries share a host, so a
 * hover costs no second copy of the module and no second analysis of the same text. It is started
 * when the editor view mounts, so the module is downloading while the page paints.</para>
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

/** Starts the worker so the module is compiling before the first evaluation is asked for. */
export function startNxWorker(): void {
  nxWorkerChannel().start();
}
