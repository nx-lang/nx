/**
 * Proves a process whose main thread is stuck does not stay up.
 *
 * The stand-in for a native call that never returns is `Atomics.wait` with no timeout, which
 * blocks the main thread as completely as the binding would: no timer, no request and no signal
 * handler written in JavaScript runs again. Only the watchdog's own thread can act, and the proof
 * is that the process ends, by the signal the watchdog sends, well before the test's deadline.
 */
import { strict as assert } from "node:assert";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

const watchdog = fileURLToPath(new URL("./watchdog.mjs", import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), "playground-watchdog-"));

after(() => rmSync(scratch, { recursive: true, force: true }));

/** Runs `script` as its own process — a file rather than `-e`, so the module loads as the server's would. */
function run(name, script) {
  const file = join(scratch, `${name}.mjs`);
  writeFileSync(file, script);
  return new Promise((fulfil, reject) => {
    const child = spawn(process.execPath, [file], { stdio: ["ignore", "ignore", "pipe"] });
    let stderr = "";
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", reject);
    child.on("exit", (code, signal) => fulfil({ code, signal, stderr }));
  });
}

test("a main thread that stops answering is killed within the deadline", async () => {
  const started = Date.now();
  const { signal, stderr } = await run("stuck", `
    import { startWatchdog } from ${JSON.stringify(watchdog)};
    startWatchdog({ deadlineMs: 1500 });
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0);
  `);
  assert.equal(signal, "SIGKILL", stderr);
  assert.match(stderr, /watchdog: the main thread has not answered/);
  assert.ok(Date.now() - started < 10_000, "the process should have been ended promptly");
});

test("a main thread that keeps answering is left alone, and the watchdog does not hold the process open", async () => {
  const { code, signal } = await run("responsive", `
    import { startWatchdog } from ${JSON.stringify(watchdog)};
    startWatchdog({ deadlineMs: 1500 });
    // Busy for longer than the deadline, but yielding to the event loop, so the heartbeat continues.
    const until = Date.now() + 2500;
    const tick = () => { if (Date.now() < until) setTimeout(tick, 100); };
    tick();
  `);
  assert.equal(signal, null);
  assert.equal(code, 0);
});
