import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
// `pnpm run publish:vsce -- <vsix>` passes the `--` through, so it is not an argument.
const [registry, vsixPath] = process.argv.slice(2).filter((arg) => arg !== '--');

// Each registry skips a VSIX whose version and target it already has, so a release with one VSIX
// per platform publishes every platform, and a repair run republishes only what is missing.
const registryConfig = {
  vsce: {
    command: 'vsce',
    // The Marketplace retires global personal access tokens on 2026-12-01. CI signs in as the
    // nx-vscode-publisher managed identity (`azure/login`), which --azure-credential picks up, as
    // does a maintainer's `az login`. VSCE_PAT still works while tokens last.
    args: (path) => [
      'publish',
      '--packagePath',
      path,
      '--skip-duplicate',
      ...(process.env.VSCE_PAT ? [] : ['--azure-credential']),
    ],
    usage: 'pnpm run publish:vsce -- <extension.vsix>',
  },
  ovsx: {
    command: 'ovsx',
    args: (path) => ['publish', path, '--skip-duplicate'],
    requiredToken: 'OVSX_PAT',
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

const resolvedVsixPath = existsSync(join(packageRoot, vsixPath)) ? join(packageRoot, vsixPath) : vsixPath;

if (!existsSync(resolvedVsixPath)) {
  console.error(`VSIX not found: ${vsixPath}`);
  process.exit(1);
}

if (config.requiredToken && !process.env[config.requiredToken]) {
  console.error(`Missing required environment variable: ${config.requiredToken}`);
  process.exit(1);
}

const executable = process.platform === 'win32' ? `${config.command}.cmd` : config.command;
const result = spawnSync(join(packageRoot, 'node_modules', '.bin', executable), config.args(resolvedVsixPath), {
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
