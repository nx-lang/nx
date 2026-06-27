import { rmSync } from "node:fs";
import { resolve } from "node:path";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

rmSync(resolve(packageRoot, "dist"), { recursive: true, force: true });
rmSync(resolve(packageRoot, "native", "nx_sdk_node.node"), { force: true });
