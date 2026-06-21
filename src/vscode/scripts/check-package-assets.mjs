import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const platformId =
  process.env.NX_LSP_PLATFORM ?? process.env.NX_VSCODE_TARGET ?? `${process.platform}-${process.arch}`;
const executableName = platformId.startsWith('win32-') ? 'nx-lsp.exe' : 'nx-lsp';
const required = [
  join(packageRoot, 'out', 'extension.cjs'),
  join(packageRoot, 'out', 'serverPath.js'),
  join(packageRoot, 'server', platformId, executableName)
];

const missing = required.filter((candidate) => !existsSync(candidate));
if (missing.length > 0) {
  for (const candidate of missing) {
    console.error(`Missing package asset: ${candidate}`);
  }
  process.exit(1);
}

console.log(`Verified NX VS Code package assets for ${platformId}.`);
