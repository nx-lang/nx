import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const executable = process.platform === 'win32' ? 'vsce.cmd' : 'vsce';
const useShell = process.platform === 'win32';
const target = process.env.NX_VSCODE_TARGET;
const args = ['package'];

if (target) {
  args.push('--target', target);
}

args.push('--no-dependencies');

const result = spawnSync(join(packageRoot, 'node_modules', '.bin', executable), args, {
  cwd: packageRoot,
  env: process.env,
  shell: useShell,
  stdio: 'inherit'
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
