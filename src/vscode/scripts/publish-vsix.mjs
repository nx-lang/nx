import { execFileSync, spawnSync } from 'node:child_process';
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
    displayName: 'Visual Studio Marketplace',
    probeArgs: (extensionId) => ['show', extensionId, '--json'],
    tokenName: 'VSCE_PAT',
    usage: 'pnpm run publish:vsce -- <extension.vsix>',
  },
  ovsx: {
    command: 'ovsx',
    args: (path) => ['publish', path],
    displayName: 'Open VSX',
    probeArgs: (extensionId, version) => ['get', extensionId, '--metadata', '--versionRange', version],
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

const resolvedVsixPath = existsSync(join(packageRoot, vsixPath)) ? join(packageRoot, vsixPath) : vsixPath;

if (!existsSync(resolvedVsixPath)) {
  console.error(`VSIX not found: ${vsixPath}`);
  process.exit(1);
}

if (!process.env[config.tokenName]) {
  console.error(`Missing required environment variable: ${config.tokenName}`);
  process.exit(1);
}

const executable = process.platform === 'win32' ? `${config.command}.cmd` : config.command;
const executablePath = join(packageRoot, 'node_modules', '.bin', executable);
const manifest = JSON.parse(execFileSync('unzip', ['-p', resolvedVsixPath, 'extension/package.json'], {
  encoding: 'utf8',
}));
const extensionId = `${manifest.publisher}.${manifest.name}`;
const version = manifest.version;

function versionAppearsInOutput(output) {
  return output.split(/\r?\n/).some((line) => line.trim() === version || line.includes(`"${version}"`));
}

function isNotPublishedResult(result) {
  const output = `${result.stdout?.toString() ?? ''}\n${result.stderr?.toString() ?? ''}`;
  return result.status !== 0 && /not found|does not exist|doesn't exist|could not find|no extension|404/i.test(output);
}

function isAlreadyPublished() {
  const result = spawnSync(executablePath, config.probeArgs(extensionId, version), {
    cwd: packageRoot,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  if (result.status === 0) {
    return versionAppearsInOutput(output);
  }

  if (isNotPublishedResult(result)) {
    return false;
  }

  console.error(output.trim());
  process.exit(result.status ?? 1);
}

if (isAlreadyPublished()) {
  console.log(`${extensionId} ${version} already exists in ${config.displayName}; skipping publish.`);
  process.exit(0);
}

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
