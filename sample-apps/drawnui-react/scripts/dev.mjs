/**
 * Runs the two halves of development together: Vite on 5173 and the compile server on 5174 (or
 * wherever `PORT` puts it — the Vite proxy reads the same setting, so the two cannot drift apart).
 *
 * They are separate processes because they are separate concerns — the compile server is what
 * production runs, and Vite never ships. Starting them together only removes the failure mode of
 * forgetting the second one, which shows up in the app as compiles that never answer.
 *
 * Either one exiting takes the other with it, so a crashed compile server cannot leave a dev
 * session quietly serving an app that no longer compiles.
 *
 * Usage: npm run dev:all
 */
import { spawn } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { COMPILE_PORT } from "../server/port.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// Both are launched through this Node rather than through a shell, so the script behaves the same
// on Windows, where the `.bin` shims are not executable files.
const children = [
  start("vite", [join(appRoot, "node_modules/vite/bin/vite.js"), ...process.argv.slice(2)]),
  start("compile", [join(appRoot, "server/index.mjs")]),
];

let shuttingDown = false;

function start(name, args) {
  // The port is passed down resolved rather than inherited, so both children agree on it however
  // the ambient environment is set.
  const child = spawn(process.execPath, args, {
    cwd: appRoot,
    env: { ...process.env, PORT: String(COMPILE_PORT) },
    stdio: ["ignore", "pipe", "pipe"],
  });
  prefix(name, child.stdout, process.stdout);
  prefix(name, child.stderr, process.stderr);
  child.on("error", (error) => {
    console.error(`[${name}] failed to start:`, error.message);
    shutDown(1);
  });
  child.on("exit", (code, signal) => {
    if (!shuttingDown) {
      console.error(`[${name}] exited (${signal ?? code})`);
    }
    shutDown(code ?? 1);
  });
  return child;
}

/** Tags each line with the process it came from, so two interleaved streams stay readable. */
function prefix(name, source, sink) {
  let pending = "";
  source.setEncoding("utf8");
  source.on("data", (chunk) => {
    pending += chunk;
    const lines = pending.split("\n");
    pending = lines.pop() ?? "";
    for (const line of lines) {
      sink.write(`[${name}] ${line}\n`);
    }
  });
  // A last line without its newline — a prompt, or output cut off by exit — is still worth showing.
  source.on("end", () => {
    if (pending !== "") {
      sink.write(`[${name}] ${pending}\n`);
    }
  });
}

function shutDown(code) {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;
  for (const child of children) {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGTERM");
    }
  }
  process.exitCode = code;
}

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => shutDown(0));
}
