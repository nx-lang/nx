// Removes the build output, so a module or test deleted from the sources does not survive a rebuild.

import { rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

rmSync(path.join(packageRoot, "dist"), { recursive: true, force: true });
