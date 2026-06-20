import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const registry = process.argv[2];
const vsixPath = process.argv[3];

const registryConfig = {
  vsce: {
    command: 'vsce',
    args: (path) => ['publish', '--packagePath', path],
    tokenName: 'VSCE_PAT',
    usage: 'pnpm run publish:vsce -- <extension.vsix>',
  },
  ovsx: {
    command: 'ovsx',
    args: (path) => ['publish', path],
    tokenName: 'OVSX_PAT',
    usage: 'pnpm run publish:ovsx -- <extension.vsix>',
  },
};

const config = registryConfig[registry];
if (!config) {
  console.error('Usage: node ./scripts/publish-vsix.mjs <vsce|ovsx> <extension.vsix>');
  process.exit(1);
}

if (!vsixPath) {
  console.error(`Usage: ${config.usage}`);
  process.exit(1);
}

if (!existsSync(join(packageRoot, vsixPath)) && !existsSync(vsixPath)) {
  console.error(`VSIX not found: ${vsixPath}`);
  process.exit(1);
}

if (!process.env[config.tokenName]) {
  console.error(`Missing required environment variable: ${config.tokenName}`);
  process.exit(1);
}

const executable = process.platform === 'win32' ? `${config.command}.cmd` : config.command;
const result = spawnSync(join(packageRoot, 'node_modules', '.bin', executable), config.args(vsixPath), {
  cwd: packageRoot,
  env: process.env,
  stdio: 'inherit',
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
