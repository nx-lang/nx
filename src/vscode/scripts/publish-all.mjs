import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const vsixPath = process.argv[2];

if (!vsixPath) {
  console.error('Usage: pnpm run publish:all -- <extension.vsix>');
  process.exit(1);
}

if (!existsSync(join(packageRoot, vsixPath)) && !existsSync(vsixPath)) {
  console.error(`VSIX not found: ${vsixPath}`);
  process.exit(1);
}

const missing = ['VSCE_PAT', 'OVSX_PAT'].filter((name) => !process.env[name]);
if (missing.length > 0) {
  console.error(`Missing required environment variable(s): ${missing.join(', ')}`);
  process.exit(1);
}

function run(command, args) {
  const executable = process.platform === 'win32' ? `${command}.cmd` : command;
  const result = spawnSync(join(packageRoot, 'node_modules', '.bin', executable), args, {
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
}

run('vsce', ['publish', '--packagePath', vsixPath]);
run('ovsx', ['publish', vsixPath]);
