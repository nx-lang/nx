/**
 * Ends the process when its main thread stops answering.
 *
 * Compile and language requests call the native binding synchronously on the main thread, and a
 * call that never returns blocks every request after it — health included — with nothing in
 * JavaScript able to interrupt it. The hosting platform cannot help either: Railway checks health
 * only while a deployment is starting, not once it is live. So the process watches itself. The
 * main thread writes a heartbeat into shared memory once a second; a worker thread, which keeps
 * running while the main thread is stuck, reads it back, and when the heartbeat is older than the
 * deadline it kills the process outright. SIGKILL rather than anything gentler, because a gentler
 * signal would be delivered to the very thread that is not listening. The exit is a failure, and
 * the platform's restart policy starts a fresh process.
 *
 * A deadline rather than a per-request timeout, because the calls are synchronous: there is no
 * point at which the main thread could check a timer. Twenty seconds is far beyond any real
 * compile or analysis of the catalog, which take milliseconds, and short enough that a stuck
 * service is back within the time a visitor would spend reloading.
 */
import { writeSync } from "node:fs";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";

/** How long the main thread may go without a heartbeat before the process is ended. */
export const WATCHDOG_DEADLINE_MS = 20_000;

/** How often the main thread writes a heartbeat and the worker reads it. */
const BEAT_MS = 1000;

/**
 * Starts the watchdog on the main thread. Returns a function that stops it, for tests; the server
 * runs it for its whole life.
 */
export function startWatchdog({ deadlineMs = WATCHDOG_DEADLINE_MS } = {}) {
  const shared = new SharedArrayBuffer(8);
  // A 64-bit integer, because `Atomics` reads and writes integers only and the monotonic clock's
  // nanoseconds do not fit in 32 bits.
  const beat = new BigInt64Array(shared);
  Atomics.store(beat, 0, now());
  // Neither the interval nor the worker should keep the process alive on its own; the server does.
  const timer = setInterval(() => Atomics.store(beat, 0, now()), BEAT_MS);
  timer.unref();
  const worker = new Worker(new URL(import.meta.url), { workerData: { shared, deadlineMs } });
  worker.unref();
  return () => {
    clearInterval(timer);
    return worker.terminate();
  };
}

/**
 * The monotonic clock, in nanoseconds: the same origin on every thread of the process, and no NTP
 * step or resume from sleep can move it, so a healthy process is never mistaken for a stuck one by
 * the wall clock jumping forward.
 */
function now() {
  return process.hrtime.bigint();
}

if (!isMainThread) {
  const beat = new BigInt64Array(workerData.shared);
  const { deadlineMs } = workerData;
  setInterval(() => {
    const stale = Number((now() - Atomics.load(beat, 0)) / 1_000_000n);
    if (stale > deadlineMs) {
      // Written straight to the descriptor: a worker's `console` goes through the main thread,
      // which is exactly the thread that is not answering.
      writeSync(2, `watchdog: the main thread has not answered for ${Math.round(stale / 1000)}s; ending the process\n`);
      process.kill(process.pid, "SIGKILL");
    }
  }, BEAT_MS);
  parentPort?.unref();
}
