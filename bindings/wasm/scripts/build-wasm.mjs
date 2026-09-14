// Builds the NX WebAssembly module and puts it where the package's loader looks for it.
//
// The pinned WASI sysroot is fetched on demand; clang compiles tree-sitter's C sources against it.
// Pass `--debug-trap` to build a second module that exports `nx_wasm_trap`, which the trap-handling
// tests use and the shipped module does not carry.

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { fetchWasiSysroot } from "../../../scripts/fetch-wasi-sysroot.mjs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(packageRoot, "../..");
const target = "wasm32-wasip1";
const profile = "wasm-release";
const profileDirectory = "wasm-release";

const withDebugTrap = process.argv.includes("--debug-trap");
const outputName = withDebugTrap ? "nx-debug-trap.wasm" : "nx.wasm";

const sysroot = await fetchWasiSysroot();

const cargoArguments = [
  "build",
  "-p",
  "nx-sdk-wasm-native",
  "--target",
  target,
  "--profile",
  profile
];
if (withDebugTrap) {
  // The same target directory as the shipped module. `debug-trap` is a feature of this crate alone
  // and changes nothing below it, so cargo rebuilds this crate and reuses every dependency it
  // already built — where a separate target directory meant a cold compile of the whole graph on
  // every CI run. The two builds overwrite one artifact, which is harmless: each is copied to its
  // own name in `dist/` before the next one runs.
  cargoArguments.push("--features", "debug-trap");
}

const cargo = spawnSync("cargo", cargoArguments, {
  cwd: repositoryRoot,
  stdio: "inherit",
  env: {
    ...process.env,
    CC_wasm32_wasip1: process.env.CC_wasm32_wasip1 ?? "clang",
    CFLAGS_wasm32_wasip1: `--sysroot=${sysroot}${
      process.env.CFLAGS_wasm32_wasip1 === undefined ? "" : ` ${process.env.CFLAGS_wasm32_wasip1}`
    }`
  }
});

if (cargo.status !== 0) {
  process.exit(cargo.status ?? 1);
}

const builtModule = path.join(
  repositoryRoot,
  "target",
  target,
  profileDirectory,
  "nx_sdk_wasm_native.wasm"
);

if (!existsSync(builtModule)) {
  throw new Error(`The NX wasm module was not produced at ${builtModule}.`);
}

const distributionRoot = path.join(packageRoot, "dist");
mkdirSync(distributionRoot, { recursive: true });
copyFileSync(builtModule, path.join(distributionRoot, outputName));
