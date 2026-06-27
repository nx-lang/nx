import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageRoot, "../..");
const cargoArgs = ["build", "-p", "nx-sdk-node-native", "--release"];
const cargo = spawnSync("cargo", cargoArgs, {
  cwd: repoRoot,
  stdio: "inherit"
});

if (cargo.status !== 0) {
  process.exit(cargo.status ?? 1);
}

const libraryName =
  process.platform === "win32"
    ? "nx_sdk_node_native.dll"
    : process.platform === "darwin"
      ? "libnx_sdk_node_native.dylib"
      : "libnx_sdk_node_native.so";
const source = resolve(repoRoot, "target", "release", libraryName);

if (!existsSync(source)) {
  throw new Error(`Native library was not produced at ${source}.`);
}

const nativeRoot = resolve(packageRoot, "native");
mkdirSync(nativeRoot, { recursive: true });
copyFileSync(source, resolve(nativeRoot, "nx_sdk_node.node"));
